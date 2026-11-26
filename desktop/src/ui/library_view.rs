use super::image_util::decode_cover_image;
use super::BookwormApp;
use bookworm_core::epub::{
    EpubCoverData, EpubMetadata, image_media_type_for_path, write_epub_metadata,
};
use bookworm_core::library::{
    BookBookmark, BookFormat, BookHighlight, BookRecord, DictDefLang, LibraryStore, ReadingStatus,
    scan_library, suggested_book_filename,
};
use bookworm_core::metadata::{
    BookMatchCandidate, default_search_query, enrich_candidate_details, fetch_cover_bytes,
    merge_into_metadata, needs_metadata_enrichment, search_book_metadata,
};
use bookworm_core::normalize_rel_key;
use crate::theme::{hex_alpha, hex_color, kd};
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::input::{Editor, EditorState, Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{Disableable as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, Image, InteractiveElement as _, IntoElement, MouseButton, ObjectFit, ParentElement,
    PathPromptOptions, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled,
    StyledImage as _, Subscription, Window, div, img, prelude::FluentBuilder as _, px, relative,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySort {
    Recent,
    Title,
    Author,
    Series,
    Progress,
}

impl LibrarySort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Title => "Title",
            Self::Author => "Author",
            Self::Series => "Series",
            Self::Progress => "Progress",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Recent => Self::Title,
            Self::Title => Self::Author,
            Self::Author => Self::Series,
            Self::Series => Self::Progress,
            Self::Progress => Self::Recent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryViewMode {
    Grid,
    List,
}

impl LibraryViewMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Grid => Self::List,
            Self::List => Self::Grid,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BookJumpTarget {
    Highlight(BookHighlight),
    Bookmark(BookBookmark),
}

#[derive(Debug, Clone)]
pub enum LibraryViewEvent {
    OpenBook {
        path: PathBuf,
        jump: Option<BookJumpTarget>,
    },
    MetadataSaved {
        path: PathBuf,
    },
    BookRenamed {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    ExportNotes,
    SetDictDefLang(DictDefLang),
    DownloadDictionary(DictDefLang),
    StatusToast(String),
    ErrorToast(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchApplyMode {
    ApplyAndSave,
    FillForm,
    FillEmpty,
}

pub struct LibraryView {
    pub workspace_root: PathBuf,
    pub store: LibraryStore,
    pub books: Vec<BookRecord>,
    cover_cache: HashMap<String, Arc<Image>>,

    search_input: Entity<InputState>,
    search_query: String,
    status_filter: Option<ReadingStatus>,
    active_tag: Option<String>,
    active_series: Option<String>,
    sort: LibrarySort,
    view_mode: LibraryViewMode,

    selected_rel: Option<String>,
    inspector_open: bool,
    pending_cover: Option<(Vec<u8>, String, Arc<Image>)>,

    matcher_open: bool,
    match_query_input: Entity<InputState>,
    match_loading: bool,
    match_applying: bool,
    match_error: Option<String>,
    match_results: Vec<BookMatchCandidate>,
    match_cover_cache: HashMap<String, (Vec<u8>, String, Arc<Image>)>,
    match_covers_fetching: HashSet<String>,
    match_gen: u64,

    batch_running: bool,
    batch_progress: Option<(usize, usize, String)>,
    batch_cancel: Option<Arc<AtomicBool>>,

    edit_title: Entity<InputState>,
    edit_authors: Entity<InputState>,
    edit_series: Entity<InputState>,
    edit_series_idx: Entity<InputState>,
    edit_subjects: Entity<InputState>,
    edit_shelves: Entity<InputState>,
    edit_publisher: Entity<InputState>,
    edit_date: Entity<InputState>,
    edit_isbn: Entity<InputState>,
    edit_language: Entity<InputState>,
    edit_description: Entity<EditorState>,
    inspector_dirty: bool,
    syncing_inspector: bool,

    pub initial_conflicts: usize,
    grid_scroll: ScrollHandle,
    inspector_scroll: ScrollHandle,
    focus_handle: FocusHandle,
    _subs: Vec<Subscription>,
}

impl EventEmitter<LibraryViewEvent> for LibraryView {}

impl Focusable for LibraryView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn cover_palette_for_title(title: &str) -> (u32, u32) {
    const PALETTES: &[(u32, u32)] = &[
        (0x362026, 0xc4746e),
        (0x1e302a, 0x87a987),
        (0x20273b, 0x7e9cd8),
        (0x2c2036, 0xa292a3),
        (0x35291d, 0xe6c384),
        (0x1b2d33, 0x7aa89f),
    ];
    let mut hash: u32 = 2166136261;
    for b in title.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    PALETTES[(hash as usize) % PALETTES.len()]
}

fn split_csv_unique(raw: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() && seen.insert(trimmed.to_ascii_lowercase()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn format_size_short(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", (bytes as f64 / 1024.0).max(1.0))
    }
}

impl LibraryView {
    pub fn new(workspace_root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (store, initial_conflicts) = LibraryStore::load_with_conflict_report(&workspace_root);
        let books = scan_library(&workspace_root, &store);
        let mut cover_cache = HashMap::new();
        for b in &books {
            if let Some(cov) = &b.cover
                && let Some(img) = decode_cover_image(cov)
            {
                cover_cache.insert(b.rel_path.clone(), img);
            }
        }

        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Filter title, author, series, or #tag…")
        });
        let s_search = cx.subscribe(&search_input, |this, state, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.search_query = state.read(cx).value().to_string();
                cx.notify();
            }
        });

        let edit_title = cx.new(|cx| InputState::new(window, cx).placeholder("Book title"));
        let edit_authors =
            cx.new(|cx| InputState::new(window, cx).placeholder("Author One, Author Two"));
        let edit_series = cx.new(|cx| InputState::new(window, cx).placeholder("Series name"));
        let edit_series_idx = cx.new(|cx| InputState::new(window, cx).placeholder("1"));
        let edit_subjects =
            cx.new(|cx| InputState::new(window, cx).placeholder("Sci-Fi, Philosophy, Classics"));
        let edit_shelves =
            cx.new(|cx| InputState::new(window, cx).placeholder("Favorites, 2026-Reads"));
        let edit_publisher = cx.new(|cx| InputState::new(window, cx).placeholder("Publisher"));
        let edit_date = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD"));
        let edit_isbn = cx.new(|cx| InputState::new(window, cx).placeholder("978-…"));
        let edit_language = cx.new(|cx| InputState::new(window, cx).placeholder("en"));
        let edit_description = cx.new(|cx| {
            EditorState::new(window, cx)
                .searchable(false)
                .soft_wrap(true)
                .placeholder("Synopsis or book blurb…")
        });
        let match_query_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search Title Author or ISBN…")
        });

        let mut subs = vec![s_search];
        subs.push(
            cx.subscribe_in(&match_query_input, window, |this, _, event, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.start_online_match(false, cx);
                }
            }),
        );
        for inp in [
            &edit_title,
            &edit_authors,
            &edit_series,
            &edit_series_idx,
            &edit_subjects,
            &edit_shelves,
            &edit_publisher,
            &edit_date,
            &edit_isbn,
            &edit_language,
        ] {
            let sub = cx.subscribe(inp, |this, _, event, cx| {
                if !this.syncing_inspector && matches!(event, InputEvent::Change) {
                    this.inspector_dirty = true;
                    cx.notify();
                }
            });
            subs.push(sub);
        }
        subs.push(cx.subscribe(&edit_description, |this, _, event, cx| {
            if !this.syncing_inspector && matches!(event, InputEvent::Change) {
                this.inspector_dirty = true;
                cx.notify();
            }
        }));

        let initial_selected = books.first().map(|b| b.rel_path.clone());
        let inspector_open = initial_selected.is_some();

        let mut view = Self {
            workspace_root,
            store,
            books,
            cover_cache,
            search_input,
            search_query: String::new(),
            status_filter: None,
            active_tag: None,
            active_series: None,
            sort: LibrarySort::Recent,
            view_mode: LibraryViewMode::Grid,
            selected_rel: None,
            inspector_open,
            pending_cover: None,
            matcher_open: false,
            match_query_input,
            match_loading: false,
            match_applying: false,
            match_error: None,
            match_results: Vec::new(),
            match_cover_cache: HashMap::new(),
            match_covers_fetching: HashSet::new(),
            match_gen: 0,
            batch_running: false,
            batch_progress: None,
            batch_cancel: None,
            edit_title,
            edit_authors,
            edit_series,
            edit_series_idx,
            edit_subjects,
            edit_shelves,
            edit_publisher,
            edit_date,
            edit_isbn,
            edit_language,
            edit_description,
            inspector_dirty: false,
            syncing_inspector: false,
            initial_conflicts,
            grid_scroll: ScrollHandle::new(),
            inspector_scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _subs: subs,
        };

        if let Some(first_rel) = initial_selected {
            view.select_book(&first_rel, window, cx);
        }

        view
    }

    pub fn summary(&self) -> String {
        let reading = self
            .books
            .iter()
            .filter(|b| b.user.status == ReadingStatus::Reading)
            .count();
        format!("{} books · {} reading", self.books.len(), reading)
    }

    pub fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |s, cx| {
            let len = s.value().len();
            s.focus(window, cx);
            if len > 0 {
                s.set_selected_range(0..len, cx);
            }
        });
        cx.notify();
    }

    pub fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |s, cx| s.set_value("", window, cx));
        self.search_query.clear();
        cx.notify();
    }

    pub fn set_workspace_root(
        &mut self,
        workspace_root: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace_root = workspace_root;
        self.selected_rel = None;
        self.active_tag = None;
        self.active_series = None;
        self.refresh_books(cx);
        if let Some(first) = self.books.first().map(|b| b.rel_path.clone()) {
            self.select_book(&first, window, cx);
        }
        cx.notify();
    }

    pub fn refresh_books(&mut self, cx: &mut Context<Self>) {
        let (store, merged_conflicts) = LibraryStore::load_with_conflict_report(&self.workspace_root);
        self.store = store;
        self.books = scan_library(&self.workspace_root, &self.store);
        self.cover_cache.clear();
        for b in &self.books {
            if let Some(cov) = &b.cover
                && let Some(img) = decode_cover_image(cov)
            {
                self.cover_cache.insert(b.rel_path.clone(), img);
            }
        }
        if let Some(sel) = &self.selected_rel
            && !self.books.iter().any(|b| &b.rel_path == sel)
        {
            self.selected_rel = self.books.first().map(|b| b.rel_path.clone());
        }
        if merged_conflicts > 0 {
            cx.emit(LibraryViewEvent::StatusToast(format!(
                "Merged {merged_conflicts} Syncthing library conflict(s)"
            )));
        }
        cx.notify();
    }

    pub fn sync_from_store(&mut self, store: LibraryStore, cx: &mut Context<Self>) {
        self.store = store;
        for b in &mut self.books {
            b.user = self.store.books.get(&b.rel_path).cloned().unwrap_or_default();
        }
        cx.notify();
    }

    pub fn select_book_by_path(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rel = normalize_rel_key(&self.workspace_root, path);
        if !self.books.iter().any(|b| b.rel_path == rel) {
            self.refresh_books(cx);
        }
        if self.books.iter().any(|b| b.rel_path == rel) {
            self.inspector_open = true;
            self.select_book(&rel, window, cx);
        }
    }

    fn populate_inspector_fields(
        &mut self,
        meta: &EpubMetadata,
        shelves: &[String],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.syncing_inspector = true;
        self.edit_title
            .update(cx, |s, cx| s.set_value(meta.title.clone(), window, cx));
        self.edit_authors
            .update(cx, |s, cx| s.set_value(meta.authors.join(", "), window, cx));
        self.edit_series.update(cx, |s, cx| {
            s.set_value(meta.series.clone().unwrap_or_default(), window, cx)
        });
        self.edit_series_idx.update(cx, |s, cx| {
            let txt = meta
                .series_index
                .map(|v| {
                    if (v.fract()).abs() < 0.001 {
                        format!("{:.0}", v)
                    } else {
                        format!("{v}")
                    }
                })
                .unwrap_or_default();
            s.set_value(txt, window, cx);
        });
        self.edit_subjects
            .update(cx, |s, cx| s.set_value(meta.subjects.join(", "), window, cx));
        self.edit_shelves
            .update(cx, |s, cx| s.set_value(shelves.join(", "), window, cx));
        self.edit_publisher.update(cx, |s, cx| {
            s.set_value(meta.publisher.clone().unwrap_or_default(), window, cx)
        });
        self.edit_date.update(cx, |s, cx| {
            s.set_value(meta.date.clone().unwrap_or_default(), window, cx)
        });
        self.edit_isbn.update(cx, |s, cx| {
            s.set_value(meta.isbn.clone().unwrap_or_default(), window, cx)
        });
        self.edit_language.update(cx, |s, cx| {
            s.set_value(meta.language.clone().unwrap_or_default(), window, cx)
        });
        self.edit_description.update(cx, |s, cx| {
            s.set_value(meta.description.clone().unwrap_or_default(), window, cx)
        });
        self.syncing_inspector = false;
    }

    fn select_book(&mut self, rel_path: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_rel = Some(rel_path.to_string());
        self.pending_cover = None;
        self.inspector_dirty = false;
        self.matcher_open = false;
        self.match_loading = false;
        self.match_applying = false;
        self.match_error = None;
        self.match_results.clear();
        self.match_gen = self.match_gen.wrapping_add(1);

        let Some(rec) = self.books.iter().find(|b| b.rel_path == rel_path).cloned() else {
            return;
        };

        self.populate_inspector_fields(&rec.metadata, &rec.user.shelves, window, cx);
        cx.notify();
    }

    fn selected_book(&self) -> Option<&BookRecord> {
        let rel = self.selected_rel.as_deref()?;
        self.books.iter().find(|b| b.rel_path == rel)
    }

    fn collect_inspector_metadata(&self, cx: &App) -> (EpubMetadata, Vec<String>) {
        let title = self.edit_title.read(cx).value().trim().to_string();
        let authors = split_csv_unique(&self.edit_authors.read(cx).value());
        let series_raw = self.edit_series.read(cx).value().trim().to_string();
        let series = (!series_raw.is_empty()).then_some(series_raw);
        let series_index = self
            .edit_series_idx
            .read(cx)
            .value()
            .trim()
            .parse::<f32>()
            .ok();
        let subjects = split_csv_unique(&self.edit_subjects.read(cx).value());
        let shelves = split_csv_unique(&self.edit_shelves.read(cx).value());

        let opt_input = |inp: &Entity<InputState>| -> Option<String> {
            let v = inp.read(cx).value().trim().to_string();
            (!v.is_empty()).then_some(v)
        };
        let desc_raw = self.edit_description.read(cx).value().trim().to_string();
        let description = (!desc_raw.is_empty()).then_some(desc_raw);

        let meta = EpubMetadata {
            title: if title.is_empty() {
                "Untitled".to_string()
            } else {
                title
            },
            authors,
            series,
            series_index,
            subjects,
            description,
            publisher: opt_input(&self.edit_publisher),
            date: opt_input(&self.edit_date),
            language: opt_input(&self.edit_language),
            isbn: opt_input(&self.edit_isbn),
        };
        (meta, shelves)
    }

    fn set_selected_status(&mut self, status: ReadingStatus, cx: &mut Context<Self>) {
        let Some(rel) = self.selected_rel.clone() else {
            return;
        };
        {
            let entry = self.store.books.entry(rel.clone()).or_default();
            entry.status = status;
            entry.updated_at = Some(chrono::Utc::now().to_rfc3339());
            if status == ReadingStatus::Finished {
                entry.overall_pct = 1.0;
            }
        }
        if let Err(err) = self.store.sync_save(&self.workspace_root) {
            cx.emit(LibraryViewEvent::ErrorToast(format!(
                "Failed to save library state: {err}"
            )));
            return;
        }
        if let Some(book) = self.books.iter_mut().find(|b| b.rel_path == rel) {
            book.user = self.store.books.get(&rel).cloned().unwrap_or_default();
        }
        cx.notify();
    }

    fn set_selected_rating(&mut self, rating: u8, cx: &mut Context<Self>) {
        let Some(rel) = self.selected_rel.clone() else {
            return;
        };
        {
            let entry = self.store.books.entry(rel.clone()).or_default();
            entry.rating = if entry.rating == rating { 0 } else { rating };
            entry.updated_at = Some(chrono::Utc::now().to_rfc3339());
        }
        if let Err(err) = self.store.sync_save(&self.workspace_root) {
            cx.emit(LibraryViewEvent::ErrorToast(format!(
                "Failed to save rating: {err}"
            )));
            return;
        }
        if let Some(book) = self.books.iter_mut().find(|b| b.rel_path == rel) {
            book.user = self.store.books.get(&rel).cloned().unwrap_or_default();
        }
        cx.notify();
    }

    fn prompt_pick_cover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Book Cover Image (PNG / JPG / WebP)".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(img_path) = paths.into_iter().next() else {
                return;
            };
            let Some(mime) = image_media_type_for_path(&img_path) else {
                let _ = this.update_in(cx, |_, _, cx| {
                    cx.emit(LibraryViewEvent::ErrorToast(
                        "Unsupported cover image format (use PNG, JPG, WebP, or GIF)".to_string(),
                    ));
                });
                return;
            };
            let Ok(bytes) = fs::read(&img_path) else {
                let _ = this.update_in(cx, |_, _, cx| {
                    cx.emit(LibraryViewEvent::ErrorToast(
                        "Failed to read selected cover image".to_string(),
                    ));
                });
                return;
            };
            let cov = EpubCoverData {
                bytes: bytes.clone(),
                media_type: mime.to_string(),
                zip_path: img_path.to_string_lossy().to_string(),
            };
            if let Some(decoded) = decode_cover_image(&cov) {
                let _ = this.update_in(cx, |view, _, cx| {
                    view.pending_cover = Some((bytes, mime.to_string(), decoded));
                    view.inspector_dirty = true;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn apply_saved_book_metadata_ui(
        &mut self,
        rel: &str,
        book_path: &Path,
        fmt: BookFormat,
        meta: EpubMetadata,
        shelves: Option<Vec<String>>,
        new_cover: Option<(Vec<u8>, String, Arc<Image>)>,
        new_file_size: Option<u64>,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some((bytes, mt, img)) = new_cover {
            let new_cov = EpubCoverData {
                bytes,
                media_type: mt,
                zip_path: String::new(),
            };
            self.cover_cache.insert(rel.to_string(), img);
            if let Some(book) = self.books.iter_mut().find(|b| b.rel_path == rel) {
                book.cover = Some(new_cov);
            }
        }

        let effective_shelves = {
            let entry = self.store.books.entry(rel.to_string()).or_default();
            if let Some(sh) = shelves {
                entry.shelves = sh;
            }
            if fmt != BookFormat::Epub {
                entry.metadata_override = Some(meta.clone());
            }
            entry.updated_at = Some(chrono::Utc::now().to_rfc3339());
            entry.shelves.clone()
        };

        if let Err(err) = self.store.sync_save(&self.workspace_root) {
            cx.emit(LibraryViewEvent::ErrorToast(format!(
                "Failed to save library state: {err}"
            )));
            return false;
        }

        if let Some(book) = self.books.iter_mut().find(|b| b.rel_path == rel) {
            book.metadata = meta.clone();
            book.user = self.store.books.get(rel).cloned().unwrap_or_default();
            if let Some(sz) = new_file_size {
                book.file_size_bytes = sz;
            }
        }

        if let Some(w) = window
            && self.selected_rel.as_deref() == Some(rel)
        {
            self.pending_cover = None;
            self.inspector_dirty = false;
            self.populate_inspector_fields(&meta, &effective_shelves, w, cx);
        }

        if fmt == BookFormat::Epub {
            cx.emit(LibraryViewEvent::MetadataSaved {
                path: book_path.to_path_buf(),
            });
        }
        true
    }

    fn save_selected_metadata(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rec) = self.selected_book().cloned() else {
            return;
        };
        let (meta, shelves) = self.collect_inspector_metadata(cx);
        let rel = rec.rel_path.clone();

        if rec.format == BookFormat::Epub {
            let pending_ref = self
                .pending_cover
                .as_ref()
                .map(|(b, mt, _)| (b.as_slice(), mt.as_str()));
            if let Err(err) = write_epub_metadata(&rec.path, &meta, pending_ref) {
                cx.emit(LibraryViewEvent::ErrorToast(format!(
                    "Failed to write EPUB tags: {err}"
                )));
                return;
            }
            let pending = self.pending_cover.take();
            let sz = fs::metadata(&rec.path)
                .map(|m| m.len())
                .unwrap_or(rec.file_size_bytes);
            if self.apply_saved_book_metadata_ui(
                &rel,
                &rec.path,
                rec.format,
                meta,
                Some(shelves),
                pending,
                Some(sz),
                Some(window),
                cx,
            ) {
                cx.emit(LibraryViewEvent::StatusToast(format!(
                    "Saved tags into {}",
                    rec.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("EPUB")
                )));
            }
        } else if self.apply_saved_book_metadata_ui(
            &rel,
            &rec.path,
            rec.format,
            meta,
            Some(shelves),
            None,
            None,
            Some(window),
            cx,
        ) {
            cx.emit(LibraryViewEvent::StatusToast(
                "Saved PDF metadata to library store".to_string(),
            ));
        }
        cx.notify();
    }

    fn toggle_online_matcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.matcher_open {
            self.matcher_open = false;
            cx.notify();
            return;
        }
        let Some(rec) = self.selected_book().cloned() else {
            return;
        };
        self.matcher_open = true;
        let query = default_search_query(&rec.metadata);
        self.match_query_input
            .update(cx, |s, cx| s.set_value(query, window, cx));
        self.start_online_match(true, cx);
    }

    fn start_online_match(&mut self, use_isbn_hint: bool, cx: &mut Context<Self>) {
        let query = self.match_query_input.read(cx).value().trim().to_string();
        if query.is_empty() {
            self.match_error = Some("Enter a title, author, or ISBN to search.".to_string());
            self.match_loading = false;
            cx.notify();
            return;
        }
        self.match_gen = self.match_gen.wrapping_add(1);
        let gen_id = self.match_gen;
        self.match_loading = true;
        self.match_error = None;
        self.match_results.clear();

        let (isbn_hint, lang_hint) = self
            .selected_book()
            .map(|r| {
                (
                    if use_isbn_hint {
                        r.metadata.isbn.clone()
                    } else {
                        None
                    },
                    r.metadata.language.clone(),
                )
            })
            .unwrap_or((None, None));
        cx.notify();

        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    search_book_metadata(&query, isbn_hint.as_deref(), lang_hint.as_deref())
                })
                .await;

            let _ = this.update(cx, |view, cx| {
                if view.match_gen != gen_id {
                    return;
                }
                view.match_loading = false;
                match res {
                    Ok(cands) => {
                        let urls: Vec<String> = cands
                            .iter()
                            .take(6)
                            .filter_map(|c| c.cover_url.clone())
                            .collect();
                        view.match_results = cands;
                        view.prefetch_match_covers(urls, cx);
                    }
                    Err(err) => {
                        view.match_error = Some(format!("{err}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn prefetch_match_covers(&mut self, urls: Vec<String>, cx: &mut Context<Self>) {
        for url in urls {
            if self.match_cover_cache.contains_key(&url)
                || self.match_covers_fetching.contains(&url)
            {
                continue;
            }
            self.match_covers_fetching.insert(url.clone());
            let gen_id = self.match_gen;
            cx.spawn(async move |this, cx| {
                let fetched = cx
                    .background_executor()
                    .spawn({
                        let u = url.clone();
                        async move { fetch_cover_bytes(&u).ok() }
                    })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    view.match_covers_fetching.remove(&url);
                    if let Some((bytes, mime)) = fetched {
                        let tmp = EpubCoverData {
                            bytes: bytes.clone(),
                            media_type: mime.clone(),
                            zip_path: String::new(),
                        };
                        if let Some(img) = decode_cover_image(&tmp) {
                            view.match_cover_cache.insert(url.clone(), (bytes, mime, img));
                        }
                    }
                    if view.match_gen == gen_id {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }

    fn apply_match_candidate(
        &mut self,
        idx: usize,
        mode: MatchApplyMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.match_applying {
            return;
        }
        let Some(cand) = self.match_results.get(idx).cloned() else {
            return;
        };
        let Some(rec) = self.selected_book().cloned() else {
            return;
        };

        let target_rel = rec.rel_path.clone();
        let (form_meta, form_shelves) = self.collect_inspector_metadata(cx);
        let base_meta = if mode == MatchApplyMode::FillEmpty {
            form_meta
        } else {
            rec.metadata.clone()
        };
        let cached_cov = cand
            .cover_url
            .as_deref()
            .and_then(|u| self.match_cover_cache.get(u).cloned());
        let need_cover = rec.format == BookFormat::Epub
            && match mode {
                MatchApplyMode::ApplyAndSave | MatchApplyMode::FillForm => cand.cover_url.is_some(),
                MatchApplyMode::FillEmpty => {
                    rec.cover.is_none() && self.pending_cover.is_none() && cand.cover_url.is_some()
                }
            };
        let gen_id = self.match_gen;
        let cand_cover_url = cand.cover_url.clone();
        let rec_path = rec.path.clone();
        let rec_fmt = rec.format;

        self.match_applying = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let outcome: anyhow::Result<(
                EpubMetadata,
                Option<(Vec<u8>, String, Arc<Image>)>,
                Option<u64>,
                bool,
            )> = cx
                .background_executor()
                .spawn(async move {
                    let enriched = enrich_candidate_details(&cand);
                    let overwrite = mode != MatchApplyMode::FillEmpty;
                    let merged = merge_into_metadata(&base_meta, &enriched, overwrite);

                    let cover_data = if need_cover {
                        if let Some(hit) = cached_cov {
                            Some(hit)
                        } else if let Some(url) = enriched.cover_url.as_deref()
                            && let Ok((bytes, mime)) = fetch_cover_bytes(url)
                        {
                            let cov = EpubCoverData {
                                bytes: bytes.clone(),
                                media_type: mime.clone(),
                                zip_path: String::new(),
                            };
                            decode_cover_image(&cov).map(|img| (bytes, mime, img))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let mut new_size = None;
                    if mode == MatchApplyMode::ApplyAndSave && rec.format == BookFormat::Epub {
                        let cov_ref = cover_data
                            .as_ref()
                            .map(|(b, mt, _)| (b.as_slice(), mt.as_str()));
                        write_epub_metadata(&rec.path, &merged, cov_ref)?;
                        new_size = fs::metadata(&rec.path).ok().map(|m| m.len());
                    }

                    let changed = merged != base_meta || cover_data.is_some();
                    Ok((merged, cover_data, new_size, changed))
                })
                .await;

            let _ = this.update_in(cx, |view, window, cx| {
                view.match_applying = false;
                match outcome {
                    Ok((merged, cover_data, new_size, changed)) => {
                        if let (Some(url), Some(cov)) = (cand_cover_url.as_deref(), &cover_data) {
                            view.match_cover_cache
                                .entry(url.to_string())
                                .or_insert_with(|| cov.clone());
                        }
                        if mode == MatchApplyMode::ApplyAndSave {
                            if view.apply_saved_book_metadata_ui(
                                &target_rel,
                                &rec_path,
                                rec_fmt,
                                merged.clone(),
                                Some(form_shelves),
                                cover_data,
                                new_size,
                                Some(window),
                                cx,
                            ) {
                                if view.selected_rel.as_deref() == Some(&target_rel) {
                                    view.matcher_open = false;
                                }
                                cx.emit(LibraryViewEvent::StatusToast(format!(
                                    "Matched & saved metadata for \"{}\"",
                                    merged.title
                                )));
                            }
                        } else if view.match_gen == gen_id
                            && view.selected_rel.as_deref() == Some(&target_rel)
                        {
                            view.populate_inspector_fields(&merged, &form_shelves, window, cx);
                            if let Some(cov) = cover_data {
                                view.pending_cover = Some(cov);
                            }
                            if changed {
                                view.inspector_dirty = true;
                            }
                            view.matcher_open = false;
                            cx.emit(LibraryViewEvent::StatusToast(
                                "Populated form from match — review and click Save Tags*"
                                    .to_string(),
                            ));
                        }
                    }
                    Err(err) => {
                        cx.emit(LibraryViewEvent::ErrorToast(format!(
                            "Failed applying metadata match: {err}"
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn start_batch_match(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.batch_running {
            return;
        }
        let targets: Vec<(String, PathBuf, BookFormat, EpubMetadata, bool)> = self
            .books
            .iter()
            .filter(|b| needs_metadata_enrichment(b))
            .map(|b| {
                (
                    b.rel_path.clone(),
                    b.path.clone(),
                    b.format,
                    b.metadata.clone(),
                    b.cover.is_some(),
                )
            })
            .collect();
        if targets.is_empty() {
            cx.emit(LibraryViewEvent::StatusToast(
                "All books already have complete metadata and covers.".to_string(),
            ));
            return;
        }

        let total = targets.len();
        let cancel = Arc::new(AtomicBool::new(false));
        self.batch_running = true;
        self.batch_cancel = Some(cancel.clone());
        self.batch_progress = Some((0, total, targets[0].3.title.clone()));
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let mut matched = 0usize;
            let mut skipped = 0usize;

            for (idx, (rel, path, fmt, existing_meta, has_cover)) in targets.into_iter().enumerate()
            {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let title_lbl = existing_meta.title.clone();
                if this
                    .update_in(cx, |view, _, cx| {
                        view.batch_progress = Some((idx + 1, total, title_lbl.clone()));
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }

                let work_res = cx
                    .background_executor()
                    .spawn(async move {
                        let query = default_search_query(&existing_meta);
                        if query.is_empty() {
                            return Ok(None);
                        }
                        let candidates = search_book_metadata(
                            &query,
                            existing_meta.isbn.as_deref(),
                            existing_meta.language.as_deref(),
                        )?;
                        let Some(best) = candidates.into_iter().find(|c| c.score >= 3.4) else {
                            return Ok(None);
                        };
                        let enriched = enrich_candidate_details(&best);
                        let merged = merge_into_metadata(&existing_meta, &enriched, false);

                        let cov_data = if !has_cover && fmt == BookFormat::Epub {
                            enriched
                                .cover_url
                                .as_deref()
                                .and_then(|u| fetch_cover_bytes(u).ok())
                                .and_then(|(bytes, mime)| {
                                    let tmp = EpubCoverData {
                                        bytes: bytes.clone(),
                                        media_type: mime.clone(),
                                        zip_path: String::new(),
                                    };
                                    decode_cover_image(&tmp).map(|img| (bytes, mime, img))
                                })
                        } else {
                            None
                        };

                        if merged == existing_meta && cov_data.is_none() {
                            return Ok(None);
                        }

                        let mut new_size = None;
                        if fmt == BookFormat::Epub {
                            let cov_ref = cov_data
                                .as_ref()
                                .map(|(b, mt, _)| (b.as_slice(), mt.as_str()));
                            write_epub_metadata(&path, &merged, cov_ref)?;
                            new_size = fs::metadata(&path).ok().map(|m| m.len());
                        }
                        anyhow::Ok(Some((path, merged, cov_data, new_size)))
                    })
                    .await;

                match work_res {
                    Ok(Some((book_path, merged, cov_data, new_size))) => {
                        let updated_ok = this.update_in(cx, |view, window, cx| {
                            let sync_win = if !view.inspector_dirty {
                                Some(window)
                            } else {
                                None
                            };
                            if view.apply_saved_book_metadata_ui(
                                &rel,
                                &book_path,
                                fmt,
                                merged,
                                None,
                                cov_data,
                                new_size,
                                sync_win,
                                cx,
                            ) {
                                cx.notify();
                                true
                            } else {
                                false
                            }
                        });
                        match updated_ok {
                            Ok(true) => matched += 1,
                            Ok(false) => skipped += 1,
                            Err(_) => return,
                        }
                    }
                    Ok(None) | Err(_) => {
                        skipped += 1;
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
            }

            let cancelled = cancel.load(Ordering::Relaxed);
            let _ = this.update_in(cx, |view, _, cx| {
                view.batch_running = false;
                view.batch_progress = None;
                view.batch_cancel = None;
                let msg = if cancelled {
                    format!("Batch match cancelled ({matched} updated, {skipped} skipped)")
                } else {
                    format!("Matched {matched} of {total} books ({skipped} skipped)")
                };
                cx.emit(LibraryViewEvent::StatusToast(msg));
                cx.notify();
            });
        })
        .detach();
    }

    fn rename_selected_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rec) = self.selected_book().cloned() else {
            return;
        };
        let (meta, _) = self.collect_inspector_metadata(cx);
        let new_fname = suggested_book_filename(&meta, rec.format.extension());
        let parent = rec.path.parent().unwrap_or(&self.workspace_root);
        let new_path = parent.join(&new_fname);
        if new_path == rec.path {
            cx.emit(LibraryViewEvent::StatusToast(
                "Filename already matches metadata".to_string(),
            ));
            return;
        }
        if new_path.exists() {
            cx.emit(LibraryViewEvent::ErrorToast(format!(
                "Cannot rename: {new_fname} already exists"
            )));
            return;
        }
        if let Err(err) = fs::rename(&rec.path, &new_path) {
            cx.emit(LibraryViewEvent::ErrorToast(format!(
                "Rename failed: {err}"
            )));
            return;
        }

        let old_rel = rec.rel_path.clone();
        let new_rel = normalize_rel_key(&self.workspace_root, &new_path);
        if self
            .store
            .relocate_key(&self.workspace_root, &rec.path, &new_path)
        {
            let _ = self.store.sync_save(&self.workspace_root);
        }
        if let Some(cov_img) = self.cover_cache.remove(&old_rel) {
            self.cover_cache.insert(new_rel.clone(), cov_img);
        }
        if let Some(book) = self.books.iter_mut().find(|b| b.rel_path == old_rel) {
            book.path = new_path.clone();
            book.rel_path = new_rel.clone();
        }
        self.selected_rel = Some(new_rel.clone());
        self.select_book(&new_rel, window, cx);
        cx.emit(LibraryViewEvent::BookRenamed {
            old_path: rec.path,
            new_path,
        });
        cx.emit(LibraryViewEvent::StatusToast(format!(
            "Renamed to {new_fname}"
        )));
    }

    fn filtered_books(&self) -> Vec<BookRecord> {
        let q = self.search_query.trim().to_ascii_lowercase();
        let mut out: Vec<BookRecord> = self
            .books
            .iter()
            .filter(|b| {
                if let Some(st) = self.status_filter
                    && b.user.status != st
                {
                    return false;
                }
                if let Some(series_f) = &self.active_series
                    && b.metadata
                        .series
                        .as_deref()
                        .is_none_or(|s| !s.eq_ignore_ascii_case(series_f))
                {
                    return false;
                }
                if let Some(tag_f) = &self.active_tag {
                    let has_tag = b
                        .all_tags()
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(tag_f));
                    if !has_tag {
                        return false;
                    }
                }
                if q.is_empty() {
                    return true;
                }
                let q_tag = q.strip_prefix('#').unwrap_or(&q);
                b.display_title().to_ascii_lowercase().contains(&q)
                    || b.authors_display().to_ascii_lowercase().contains(&q)
                    || b.metadata
                        .series
                        .as_deref()
                        .is_some_and(|s| s.to_ascii_lowercase().contains(&q))
                    || b.rel_path.to_ascii_lowercase().contains(&q)
                    || b.all_tags()
                        .iter()
                        .any(|t| t.to_ascii_lowercase().contains(q_tag))
            })
            .cloned()
            .collect();

        match self.sort {
            LibrarySort::Recent => out.sort_by(|a, b| {
                b.user
                    .last_opened
                    .as_deref()
                    .unwrap_or("")
                    .cmp(a.user.last_opened.as_deref().unwrap_or(""))
                    .then_with(|| {
                        a.display_title()
                            .to_ascii_lowercase()
                            .cmp(&b.display_title().to_ascii_lowercase())
                    })
            }),
            LibrarySort::Title => out.sort_by(|a, b| {
                a.display_title()
                    .to_ascii_lowercase()
                    .cmp(&b.display_title().to_ascii_lowercase())
            }),
            LibrarySort::Author => out.sort_by(|a, b| {
                a.authors_display()
                    .to_ascii_lowercase()
                    .cmp(&b.authors_display().to_ascii_lowercase())
                    .then_with(|| {
                        a.display_title()
                            .to_ascii_lowercase()
                            .cmp(&b.display_title().to_ascii_lowercase())
                    })
            }),
            LibrarySort::Series => out.sort_by(|a, b| {
                let sa = a.metadata.series.as_deref().unwrap_or("~");
                let sb = b.metadata.series.as_deref().unwrap_or("~");
                sa.to_ascii_lowercase()
                    .cmp(&sb.to_ascii_lowercase())
                    .then_with(|| {
                        a.metadata
                            .series_index
                            .unwrap_or(9999.0)
                            .total_cmp(&b.metadata.series_index.unwrap_or(9999.0))
                    })
                    .then_with(|| {
                        a.display_title()
                            .to_ascii_lowercase()
                            .cmp(&b.display_title().to_ascii_lowercase())
                    })
            }),
            LibrarySort::Progress => out.sort_by(|a, b| {
                b.progress_pct()
                    .cmp(&a.progress_pct())
                    .then_with(|| {
                        a.display_title()
                            .to_ascii_lowercase()
                            .cmp(&b.display_title().to_ascii_lowercase())
                    })
            }),
        }

        out
    }

    fn render_continue_reading_strip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let unfiltered = self.search_query.trim().is_empty()
            && self.status_filter.is_none()
            && self.active_tag.is_none()
            && self.active_series.is_none();
        if !unfiltered {
            return None;
        }
        let mut recent: Vec<BookRecord> = self
            .books
            .iter()
            .filter(|b| {
                b.user.status != ReadingStatus::Finished
                    && (b.user.last_opened.is_some()
                        || b.user.chapter_idx > 0
                        || b.user.scroll_pct > 0.01)
            })
            .cloned()
            .collect();
        if recent.is_empty() {
            return None;
        }
        recent.sort_by(|a, b| b.user.last_opened.cmp(&a.user.last_opened));
        recent.truncate(4);

        let deep_bg = hex_color(kd::BG_DEEP);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER_SUBTLE);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let violet = hex_color(kd::VIOLET);

        let cards = recent.into_iter().enumerate().map(|(ix, rec)| {
            let open_path = rec.path.clone();
            let cov = self.render_cover_art(&rec, None, 48.0, 68.0, false);
            let title = rec.display_title().to_string();
            let author = rec.authors_display();
            let pct = rec.progress_pct();
            let ch_info = if rec.user.total_pages > 0 && rec.user.current_page > 0 {
                format!(
                    "Page {}/{} · {}% · Resume",
                    rec.user.current_page, rec.user.total_pages, pct
                )
            } else if rec.user.total_chapters > 0 {
                format!(
                    "Ch {}/{} · {}% · Resume",
                    rec.user.chapter_idx + 1,
                    rec.user.total_chapters,
                    pct
                )
            } else {
                format!("{pct}% · Resume")
            };

            h_flex()
                .id(("lib-continue-card", ix))
                .w(px(268.))
                .p_2p5()
                .gap_2p5()
                .items_center()
                .rounded(px(8.))
                .bg(deep_bg)
                .border_1()
                .border_color(border)
                .cursor_pointer()
                .hover(move |s| s.bg(panel_bg).border_color(violet))
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.emit(LibraryViewEvent::OpenBook {
                        path: open_path.clone(),
                        jump: None,
                    });
                }))
                .child(cov)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(fg)
                                .truncate()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(muted)
                                .truncate()
                                .child(author),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(violet)
                                .child(ch_info),
                        ),
                )
        });

        Some(
            v_flex()
                .w_full()
                .mb_4()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child("CONTINUE READING"),
                )
                .child(
                    h_flex()
                        .w_full()
                        .gap_3()
                        .flex_wrap()
                        .items_stretch()
                        .children(cards),
                )
                .into_any_element(),
        )
    }

    fn render_cover_art(
        &self,
        rec: &BookRecord,
        img_override: Option<Arc<Image>>,
        width: f32,
        height: f32,
        show_badge: bool,
    ) -> AnyElement {
        let cached = img_override.or_else(|| self.cover_cache.get(&rec.rel_path).cloned());
        let border_c = hex_color(kd::BORDER_SUBTLE);
        let badge_txt = rec
            .series_badge()
            .unwrap_or_else(|| rec.format.label().to_string());
        let pct = rec.progress_pct();
        let status = rec.user.status;

        div()
            .relative()
            .w(px(width))
            .h(px(height))
            .flex_shrink_0()
            .rounded(px(8.))
            .overflow_hidden()
            .border_1()
            .border_color(border_c)
            .child(if let Some(cov) = cached {
                img(cov)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element()
            } else {
                let (bg_hex, spine_hex) = cover_palette_for_title(rec.display_title());
                h_flex()
                    .size_full()
                    .bg(hex_color(bg_hex))
                    .child(div().w(px(6.)).h_full().bg(hex_color(spine_hex)))
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .p_2p5()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px((width * 0.06).max(9.0)))
                                    .text_color(hex_color(spine_hex))
                                    .truncate()
                                    .child(rec.format.label()),
                            )
                            .child(
                                div()
                                    .text_size(px((width * 0.088).clamp(10.0, 15.0)))
                                    .font_weight(FontWeight::BOLD)
                                    .font_family("serif")
                                    .text_color(hex_color(kd::FG))
                                    .child(rec.display_title().to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px((width * 0.068).clamp(9.0, 11.5)))
                                    .text_color(hex_color(kd::FG_MUTED))
                                    .truncate()
                                    .child(rec.authors_display()),
                            ),
                    )
                    .into_any_element()
            })
            .when(show_badge, |box_el| {
                box_el
                    .child(
                        div()
                            .absolute()
                            .top(px(6.))
                            .left(px(6.))
                            .max_w(px(width - 42.0))
                            .px_1p5()
                            .py_0p5()
                            .rounded(px(4.))
                            .bg(hex_alpha(0x121216d8))
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(hex_color(kd::FG))
                            .truncate()
                            .child(badge_txt),
                    )
                    .when(status != ReadingStatus::Unread, |box_el| {
                        let dot_color = match status {
                            ReadingStatus::Reading => hex_color(kd::ACCENT),
                            ReadingStatus::Finished => hex_color(kd::GREEN),
                            ReadingStatus::OnHold => hex_color(kd::YELLOW),
                            ReadingStatus::Unread => hex_color(kd::FG_MUTED),
                        };
                        box_el.child(
                            div()
                                .absolute()
                                .top(px(7.))
                                .right(px(7.))
                                .w(px(10.))
                                .h(px(10.))
                                .rounded_full()
                                .bg(dot_color)
                                .border_1()
                                .border_color(hex_color(kd::BG_DEEP)),
                        )
                    })
                    .when(pct > 0, |box_el| {
                        let frac = (pct as f32 / 100.0).clamp(0.0, 1.0);
                        let bar_c = if status == ReadingStatus::Finished {
                            hex_color(kd::GREEN)
                        } else {
                            hex_color(kd::VIOLET)
                        };
                        box_el.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(4.))
                                .bg(hex_alpha(0x00000090))
                                .child(div().h_full().w(relative(frac)).bg(bar_c)),
                        )
                    })
            })
            .into_any_element()
    }

    fn render_matcher_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let deep_bg = hex_color(kd::BG_DEEP);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER_SUBTLE);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let violet = hex_color(kd::VIOLET);

        let cards: Vec<AnyElement> = self
            .match_results
            .iter()
            .take(6)
            .enumerate()
            .map(|(idx, cand)| {
                let thumb = cand
                    .cover_url
                    .as_deref()
                    .and_then(|u| self.match_cover_cache.get(u).map(|(_, _, img)| Arc::clone(img)));
                let authors_str = if cand.authors.is_empty() {
                    "Unknown Author".to_string()
                } else {
                    cand.authors.join(", ")
                };
                let mut meta_parts = Vec::new();
                if let Some(d) = &cand.date {
                    meta_parts.push(d.clone());
                }
                if let Some(p) = &cand.publisher {
                    meta_parts.push(p.clone());
                }
                if let Some(i) = &cand.isbn {
                    meta_parts.push(format!("ISBN {i}"));
                }
                let meta_line = meta_parts.join(" · ");
                let desc_snippet = cand.description.as_deref().map(|d| {
                    let clean = d.split_whitespace().collect::<Vec<_>>().join(" ");
                    if clean.chars().count() > 130 {
                        let short: String = clean.chars().take(130).collect();
                        format!("{short}…")
                    } else {
                        clean
                    }
                });
                let subjects: Vec<String> = cand.subjects.iter().take(3).cloned().collect();
                let source_lbl = cand.source.label();
                let applying = self.match_applying;

                v_flex()
                    .id(("lib-match-cand", idx))
                    .p_2()
                    .gap_1p5()
                    .rounded(px(6.))
                    .bg(panel_bg)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_start()
                            .child(
                                div()
                                    .w(px(42.))
                                    .h(px(62.))
                                    .flex_shrink_0()
                                    .rounded(px(4.))
                                    .overflow_hidden()
                                    .bg(deep_bg)
                                    .border_1()
                                    .border_color(border)
                                    .child(if let Some(img_arc) = thumb {
                                        img(img_arc)
                                            .size_full()
                                            .object_fit(ObjectFit::Cover)
                                            .into_any_element()
                                    } else {
                                        div()
                                            .size_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                Icon::new(IconName::BookOpen)
                                                    .xsmall()
                                                    .text_color(muted),
                                            )
                                            .into_any_element()
                                    }),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(fg)
                                            .truncate()
                                            .child(cand.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .text_color(muted)
                                            .truncate()
                                            .child(authors_str),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(
                                                div()
                                                    .px_1()
                                                    .rounded(px(3.))
                                                    .bg(deep_bg)
                                                    .text_size(px(9.5))
                                                    .text_color(violet)
                                                    .child(source_lbl),
                                            )
                                            .when(!meta_line.is_empty(), |row| {
                                                row.child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .text_size(px(9.5))
                                                        .text_color(muted)
                                                        .truncate()
                                                        .child(meta_line),
                                                )
                                            }),
                                    ),
                            ),
                    )
                    .when_some(desc_snippet, |card, snip| {
                        card.child(
                            div()
                                .text_size(px(10.5))
                                .text_color(muted)
                                .max_h(px(30.))
                                .overflow_hidden()
                                .child(snip),
                        )
                    })
                    .when(!subjects.is_empty(), |card| {
                        card.child(h_flex().gap_1().flex_wrap().children(subjects.into_iter().map(
                            |sub| {
                                div()
                                    .px_1()
                                    .rounded(px(3.))
                                    .bg(deep_bg)
                                    .text_size(px(9.5))
                                    .text_color(muted)
                                    .child(sub)
                            },
                        )))
                    })
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .child(
                                Button::new(("lib-match-apply", idx))
                                    .primary()
                                    .xsmall()
                                    .icon(IconName::Check)
                                    .label(if applying { "Saving…" } else { "Apply & Save" })
                                    .disabled(applying)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.apply_match_candidate(
                                            idx,
                                            MatchApplyMode::ApplyAndSave,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new(("lib-match-fill", idx))
                                    .secondary()
                                    .xsmall()
                                    .label("Fill Form")
                                    .disabled(applying)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.apply_match_candidate(
                                            idx,
                                            MatchApplyMode::FillForm,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new(("lib-match-empty", idx))
                                    .ghost()
                                    .xsmall()
                                    .label("Fill Empty")
                                    .disabled(applying)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.apply_match_candidate(
                                            idx,
                                            MatchApplyMode::FillEmpty,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    )
                    .into_any_element()
            })
            .collect();

        v_flex()
            .p_2p5()
            .gap_2()
            .rounded(px(8.))
            .bg(deep_bg)
            .border_1()
            .border_color(violet)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(fg)
                            .child("Online Metadata Match"),
                    )
                    .child(
                        Button::new("lib-match-close")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.matcher_open = false;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.match_query_input).xsmall()),
                    )
                    .child(
                        Button::new("lib-match-go")
                            .primary()
                            .xsmall()
                            .icon(IconName::Search)
                            .label(if self.match_loading { "…" } else { "Search" })
                            .disabled(self.match_loading)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_online_match(false, cx);
                            })),
                    ),
            )
            .child(if self.match_loading {
                div()
                    .py_2()
                    .text_xs()
                    .text_color(muted)
                    .child("Searching Open Library & Apple Books…")
                    .into_any_element()
            } else if let Some(err) = &self.match_error {
                div()
                    .p_2()
                    .rounded(px(5.))
                    .bg(panel_bg)
                    .text_xs()
                    .text_color(hex_color(kd::RED))
                    .child(err.clone())
                    .into_any_element()
            } else if cards.is_empty() {
                div()
                    .py_2()
                    .text_xs()
                    .text_color(muted)
                    .child("No matching editions found. Try editing the query above.")
                    .into_any_element()
            } else {
                v_flex().gap_2().children(cards).into_any_element()
            })
            .into_any_element()
    }

    fn render_inspector(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let sidebar_bg = hex_color(kd::BG_SIDEBAR);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER_SUBTLE);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let violet = hex_color(kd::VIOLET);

        let Some(rec) = self.selected_book().cloned() else {
            return v_flex()
                .w(px(344.))
                .h_full()
                .bg(sidebar_bg)
                .border_l_1()
                .border_color(border)
                .items_center()
                .justify_center()
                .p_4()
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("Select a book to inspect or edit tags."),
                )
                .into_any_element();
        };

        let pending_img = self.pending_cover.as_ref().map(|(_, _, img)| Arc::clone(img));
        let cover_el = self.render_cover_art(&rec, pending_img, 108.0, 158.0, false);
        let is_epub = rec.format == BookFormat::Epub;
        let pct = rec.progress_pct();
        let read_label = if pct > 0 && rec.user.status != ReadingStatus::Finished {
            format!("Continue ({pct}%)")
        } else {
            "Read Book".to_string()
        };

        let status_pills = h_flex().gap_1().flex_wrap().children(
            ReadingStatus::all().iter().copied().map(|st| {
                let active = rec.user.status == st;
                Button::new(SharedString::from(format!("lib-st-{}", st.label())))
                    .with_variant(if active {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    })
                    .xsmall()
                    .label(st.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_selected_status(st, cx);
                    }))
            }),
        );

        let stars = h_flex().gap_0p5().items_center().children((1..=5u8).map(|n| {
            let filled = n <= rec.user.rating;
            let color = if filled {
                hex_color(kd::YELLOW)
            } else {
                hex_color(kd::BORDER)
            };
            div()
                .id(("lib-star", n as usize))
                .cursor_pointer()
                .p_0p5()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_selected_rating(n, cx);
                }))
                .child(Icon::new(IconName::Star).xsmall().text_color(color))
        }));

        let field_label = |txt: &'static str| {
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(muted)
                .child(txt)
        };

        let save_label = if self.inspector_dirty {
            if is_epub {
                "Save Tags*"
            } else {
                "Save Meta*"
            }
        } else if is_epub {
            "Save Tags"
        } else {
            "Save Meta"
        };

        let rec_open = rec.clone();

        v_flex()
            .w(px(344.))
            .h_full()
            .bg(sidebar_bg)
            .border_l_1()
            .border_color(border)
            .child(
                h_flex()
                    .h(px(38.))
                    .px_3()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(fg)
                                    .child("Metadata & Cover"),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(4.))
                                    .bg(panel_bg)
                                    .text_size(px(10.))
                                    .text_color(violet)
                                    .child(if is_epub { "EPUB OPF" } else { "PDF Store" }),
                            ),
                    )
                    .child(
                        Button::new("lib-insp-close")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.inspector_open = false;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("lib-insp-scroll")
                    .track_scroll(&self.inspector_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_3()
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_3()
                                    .items_start()
                                    .child(cover_el)
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap_1p5()
                                            .child(
                                                Button::new("lib-insp-read")
                                                    .primary()
                                                    .xsmall()
                                                    .icon(IconName::BookOpen)
                                                    .label(read_label)
                                                    .on_click(cx.listener(move |_, _, _, cx| {
                                                        cx.emit(LibraryViewEvent::OpenBook {
                                                            path: rec_open.path.clone(),
                                                            jump: None,
                                                        });
                                                    })),
                                            )
                                            .child(
                                                Button::new("lib-insp-match")
                                                    .with_variant(if self.matcher_open {
                                                        ButtonVariant::Primary
                                                    } else {
                                                        ButtonVariant::Secondary
                                                    })
                                                    .xsmall()
                                                    .icon(IconName::Search)
                                                    .label("Match Online")
                                                    .disabled(self.batch_running)
                                                    .tooltip("Search Open Library & Apple Books for metadata and cover")
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        this.toggle_online_matcher(window, cx);
                                                    })),
                                            )
                                            .when(is_epub, |col| {
                                                col.child(
                                                    Button::new("lib-insp-cover")
                                                        .ghost()
                                                        .xsmall()
                                                        .icon(IconName::ImagePlus)
                                                        .label(if self.pending_cover.is_some() {
                                                            "Cover Staged*"
                                                        } else {
                                                            "Change Cover…"
                                                        })
                                                        .on_click(cx.listener(
                                                            |this, _, window, cx| {
                                                                this.prompt_pick_cover(window, cx);
                                                            },
                                                        )),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .text_size(px(10.5))
                                                    .text_color(muted)
                                                    .truncate()
                                                    .child(if rec.chapter_count > 0 {
                                                        format!(
                                                            "{} · {} ch · {}",
                                                            rec.format.label(),
                                                            rec.chapter_count,
                                                            format_size_short(rec.file_size_bytes)
                                                        )
                                                    } else {
                                                        format!(
                                                            "{} · {}",
                                                            rec.format.label(),
                                                            format_size_short(rec.file_size_bytes)
                                                        )
                                                    }),
                                            ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap_1p5()
                                    .p_2()
                                    .rounded(px(6.))
                                    .bg(panel_bg)
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(field_label("READING STATUS"))
                                            .child(stars),
                                    )
                                    .child(status_pills),
                            )
                            .when(self.matcher_open, |col| {
                                col.child(self.render_matcher_panel(cx))
                            })
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .child(field_label("TITLE"))
                                            .child(Input::new(&self.edit_title).xsmall()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .child(field_label("AUTHORS (COMMA-SEPARATED)"))
                                            .child(Input::new(&self.edit_authors).xsmall()),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .gap_0p5()
                                                    .child(field_label("SERIES"))
                                                    .child(Input::new(&self.edit_series).xsmall()),
                                            )
                                            .child(
                                                v_flex()
                                                    .w(px(64.))
                                                    .gap_0p5()
                                                    .child(field_label("#"))
                                                    .child(
                                                        Input::new(&self.edit_series_idx).xsmall(),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .child(field_label(if is_epub {
                                                "SUBJECTS / TAGS (IN EPUB)"
                                            } else {
                                                "TAGS (IN LIBRARY)"
                                            }))
                                            .child(Input::new(&self.edit_subjects).xsmall()),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .child(field_label("SHELVES (LIBRARY DOTFILE)"))
                                            .child(Input::new(&self.edit_shelves).xsmall()),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .gap_0p5()
                                                    .child(field_label("PUBLISHER"))
                                                    .child(
                                                        Input::new(&self.edit_publisher).xsmall(),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .w(px(96.))
                                                    .gap_0p5()
                                                    .child(field_label("DATE"))
                                                    .child(Input::new(&self.edit_date).xsmall()),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .gap_0p5()
                                                    .child(field_label("ISBN"))
                                                    .child(Input::new(&self.edit_isbn).xsmall()),
                                            )
                                            .child(
                                                v_flex()
                                                    .w(px(72.))
                                                    .gap_0p5()
                                                    .child(field_label("LANG"))
                                                    .child(
                                                        Input::new(&self.edit_language).xsmall(),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .child(field_label("DESCRIPTION"))
                                            .child(
                                                div()
                                                    .h(px(92.))
                                                    .w_full()
                                                    .rounded(px(6.))
                                                    .bg(hex_color(kd::BG_DEEP))
                                                    .border_1()
                                                    .border_color(border)
                                                    .child(
                                                        Editor::new(&self.edit_description)
                                                            .size_full()
                                                            .appearance(false)
                                                            .bordered(false),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("lib-insp-save")
                                            .with_variant(if self.inspector_dirty {
                                                ButtonVariant::Primary
                                            } else {
                                                ButtonVariant::Secondary
                                            })
                                            .small()
                                            .flex_1()
                                            .icon(IconName::Save)
                                            .label(save_label)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.save_selected_metadata(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("lib-insp-rename")
                                            .secondary()
                                            .small()
                                            .icon(IconName::Pencil)
                                            .label("Rename File")
                                            .tooltip("Rename book file on disk from Title / Series / Author metadata")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.rename_selected_file(window, cx);
                                            })),
                                    ),
                            )
                            .when(!rec.user.highlights.is_empty(), |col| {
                                let mut hls = rec.user.highlights.clone();
                                hls.sort_by_key(|h| (h.chapter_idx, h.start_block, h.start_offset));
                                let hl_count = hls.len();
                                let book_path = rec.path.clone();
                                col.child(
                                    v_flex()
                                        .pt_2()
                                        .border_t_1()
                                        .border_color(border)
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(muted)
                                                .child(format!("HIGHLIGHTS & NOTES ({hl_count})")),
                                        )
                                        .children(hls.into_iter().enumerate().map(|(ix, hl)| {
                                            let open_path = book_path.clone();
                                            let hl_jump = hl.clone();
                                            let bar_c = hex_color(hl.color.rgb_hex());
                                            let note_preview = hl.note.trim().to_string();
                                            v_flex()
                                                .id(("lib-hl", ix))
                                                .px_2()
                                                .py_1p5()
                                                .gap_1()
                                                .rounded(px(5.))
                                                .bg(panel_bg)
                                                .border_l_4()
                                                .border_color(bar_c)
                                                .cursor_pointer()
                                                .hover(|s| s.bg(hex_alpha(0x8ba4b026)))
                                                .on_click(cx.listener(move |_, _, _, cx| {
                                                    cx.emit(LibraryViewEvent::OpenBook {
                                                        path: open_path.clone(),
                                                        jump: Some(BookJumpTarget::Highlight(
                                                            hl_jump.clone(),
                                                        )),
                                                    });
                                                }))
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(bar_c)
                                                        .child(format!(
                                                            "Ch {} · {}",
                                                            hl.chapter_idx + 1,
                                                            hl.style.label()
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .italic()
                                                        .text_color(fg)
                                                        .max_h(px(34.))
                                                        .overflow_hidden()
                                                        .child(hl.quote),
                                                )
                                                .when(!note_preview.is_empty(), |card| {
                                                    card.child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(hex_color(kd::ACCENT))
                                                            .truncate()
                                                            .child(note_preview),
                                                    )
                                                })
                                        })),
                                )
                            })
                            .when(!rec.user.bookmarks.is_empty(), |col| {
                                let bms = rec.user.bookmarks.clone();
                                let book_path = rec.path.clone();
                                col.child(
                                    v_flex()
                                        .pt_2()
                                        .border_t_1()
                                        .border_color(border)
                                        .gap_1p5()
                                        .child(field_label("SAVED BOOKMARKS"))
                                        .children(bms.into_iter().enumerate().map(|(ix, bm)| {
                                            let open_path = book_path.clone();
                                            let bm_jump = bm.clone();
                                            let pct = (bm.scroll_pct.clamp(0.0, 1.0) * 100.0)
                                                .round() as u8;
                                            let line = if bm.note.trim().is_empty() {
                                                format!("Ch {} · {pct}% · {}", bm.chapter_idx + 1, bm.chapter_title)
                                            } else {
                                                format!("Ch {} ({pct}%): {}", bm.chapter_idx + 1, bm.note.trim())
                                            };
                                            h_flex()
                                                .id(("lib-bm", ix))
                                                .px_2()
                                                .py_1()
                                                .gap_1p5()
                                                .items_center()
                                                .rounded(px(5.))
                                                .bg(panel_bg)
                                                .cursor_pointer()
                                                .hover(|s| s.bg(hex_alpha(0x8ba4b026)))
                                                .on_click(cx.listener(move |_, _, _, cx| {
                                                    cx.emit(LibraryViewEvent::OpenBook {
                                                        path: open_path.clone(),
                                                        jump: Some(BookJumpTarget::Bookmark(
                                                            bm_jump.clone(),
                                                        )),
                                                    });
                                                }))
                                                .child(
                                                    Icon::new(IconName::Bookmark)
                                                        .xsmall()
                                                        .text_color(violet),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .text_xs()
                                                        .text_color(fg)
                                                        .truncate()
                                                        .child(line),
                                                )
                                        })),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}

impl Render for LibraryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let deep_bg = hex_color(kd::BG_DEEP);
        let sidebar_bg = hex_color(kd::BG_SIDEBAR);
        let main_bg = hex_color(kd::BG_MAIN);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER_SUBTLE);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let accent = hex_color(kd::ACCENT);
        let violet = hex_color(kd::VIOLET);

        let query_non_empty = !self.search_query.is_empty();
        let clear_btn = Button::new(SharedString::from(format!(
            "lib-search-x-{:?}",
            cx.entity_id()
        )))
        .text()
        .xsmall()
        .tab_stop(false)
        .icon(IconName::Close)
        .tooltip("Clear Filter")
        .on_click(cx.listener(|this, _, window, cx| {
            this.clear_search(window, cx);
        }));

        // Collect tag & series chip inventories
        let mut series_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut tag_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut reading_cnt = 0usize;
        let mut unread_cnt = 0usize;
        let mut finished_cnt = 0usize;
        let mut hold_cnt = 0usize;

        for b in &self.books {
            match b.user.status {
                ReadingStatus::Reading => reading_cnt += 1,
                ReadingStatus::Unread => unread_cnt += 1,
                ReadingStatus::Finished => finished_cnt += 1,
                ReadingStatus::OnHold => hold_cnt += 1,
            }
            if let Some(s) = b.metadata.series.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                *series_counts.entry(s.to_string()).or_insert(0) += 1;
            }
            for t in b.all_tags() {
                *tag_counts.entry(t).or_insert(0) += 1;
            }
        }

        let status_buttons: Vec<(&'static str, Option<ReadingStatus>, usize)> = vec![
            ("All", None, self.books.len()),
            ("Reading", Some(ReadingStatus::Reading), reading_cnt),
            ("Unread", Some(ReadingStatus::Unread), unread_cnt),
            ("Finished", Some(ReadingStatus::Finished), finished_cnt),
            ("On Hold", Some(ReadingStatus::OnHold), hold_cnt),
        ];

        let missing_meta_count = self
            .books
            .iter()
            .filter(|b| needs_metadata_enrichment(b))
            .count();

        let toolbar = h_flex()
            .h(px(40.))
            .w_full()
            .px_3()
            .gap_2()
            .items_center()
            .justify_between()
            .bg(sidebar_bg)
            .border_b_1()
            .border_color(border)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .child(
                        div().w(px(236.)).child(
                            Input::new(&self.search_input)
                                .xsmall()
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .xsmall()
                                        .text_color(muted),
                                )
                                .when(query_non_empty, |i| i.suffix(clear_btn)),
                        ),
                    )
                    .child(h_flex().gap_1().items_center().children(
                        status_buttons.into_iter().map(|(lbl, filter_val, cnt)| {
                            let active = self.status_filter == filter_val;
                            Button::new(SharedString::from(format!("lib-flt-{lbl}")))
                                .with_variant(if active {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Ghost
                                })
                                .xsmall()
                                .label(format!("{lbl} ({cnt})"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.status_filter = filter_val;
                                    cx.notify();
                                }))
                        }),
                    )),
            )
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Button::new("lib-sort")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ArrowUpDown)
                            .label(self.sort.label())
                            .tooltip("Cycle Sort Order")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sort = this.sort.next();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("lib-view-mode")
                            .ghost()
                            .xsmall()
                            .icon(match self.view_mode {
                                LibraryViewMode::Grid => IconName::LayoutGrid,
                                LibraryViewMode::List => IconName::List,
                            })
                            .label(match self.view_mode {
                                LibraryViewMode::Grid => "Grid",
                                LibraryViewMode::List => "List",
                            })
                            .tooltip("Toggle Grid / List View")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.view_mode = this.view_mode.toggle();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("lib-match-missing")
                            .ghost()
                            .xsmall()
                            .icon(IconName::BookCheck)
                            .label(format!("Match Missing ({missing_meta_count})"))
                            .disabled(missing_meta_count == 0 || self.batch_running)
                            .tooltip("Auto-match missing metadata & covers via Open Library + Apple Books")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.start_batch_match(window, cx);
                            })),
                    )
                    .child(
                        Button::new("lib-insp-toggle")
                            .with_variant(if self.inspector_open {
                                ButtonVariant::Secondary
                            } else {
                                ButtonVariant::Ghost
                            })
                            .xsmall()
                            .icon(IconName::SlidersHorizontal)
                            .label("Inspector")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.inspector_open = !this.inspector_open;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("lib-export-md")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Save)
                            .label("Export Notes")
                            .tooltip("Export highlights & notes for all books to Markdown (.md)")
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(LibraryViewEvent::ExportNotes);
                            })),
                    )
                    .child({
                        let def_lang = self.store.reader_prefs.dict_def_lang;
                        let next_def = match def_lang {
                            DictDefLang::En => DictDefLang::Ru,
                            DictDefLang::Ru => DictDefLang::ZhHans,
                            DictDefLang::ZhHans => DictDefLang::En,
                        };
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                Button::new("lib-dict-lang")
                                    .ghost()
                                    .xsmall()
                                    .label(format!("Def: {}", def_lang.short_label()))
                                    .tooltip("Cycle Dictionary Definition Language (English / Русский / 中文)")
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(LibraryViewEvent::SetDictDefLang(next_def));
                                    })),
                            )
                            .child(
                                Button::new("lib-download-dict")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::BookOpen)
                                    .label(format!("Dict ({})", def_lang.short_label()))
                                    .tooltip(format!(
                                        "Download / Update Offline {} Dictionary Pack",
                                        def_lang.label()
                                    ))
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(LibraryViewEvent::DownloadDictionary(def_lang));
                                    })),
                            )
                    })
                    .child(
                        Button::new("lib-refresh")
                            .ghost()
                            .xsmall()
                            .icon(IconName::RefreshCw)
                            .tooltip("Rescan Library Workspace")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_books(cx);
                            })),
                    ),
            );

        let has_chips = !series_counts.is_empty() || !tag_counts.is_empty();
        let chip_bar = has_chips.then(|| {
            let mut series_vec: Vec<(String, usize)> = series_counts.into_iter().collect();
            series_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            series_vec.truncate(8);

            let mut tag_vec: Vec<(String, usize)> = tag_counts.into_iter().collect();
            tag_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            tag_vec.truncate(14);

            h_flex()
                .w_full()
                .px_3()
                .py_1p5()
                .gap_1()
                .flex_wrap()
                .items_center()
                .bg(deep_bg)
                .border_b_1()
                .border_color(border)
                .children(series_vec.into_iter().map(|(s, _)| {
                    let active = self
                        .active_series
                        .as_deref()
                        .is_some_and(|a| a.eq_ignore_ascii_case(&s));
                    let s_click = s.clone();
                    Button::new(SharedString::from(format!("lib-ser-{s}")))
                        .with_variant(if active {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Secondary
                        })
                        .xsmall()
                        .label(format!("Series: {s}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.active_series = if active { None } else { Some(s_click.clone()) };
                            cx.notify();
                        }))
                }))
                .children(tag_vec.into_iter().map(|(t, _)| {
                    let active = self
                        .active_tag
                        .as_deref()
                        .is_some_and(|a| a.eq_ignore_ascii_case(&t));
                    let t_click = t.clone();
                    Button::new(SharedString::from(format!("lib-tag-{t}")))
                        .with_variant(if active {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .xsmall()
                        .label(format!("#{t}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.active_tag = if active { None } else { Some(t_click.clone()) };
                            cx.notify();
                        }))
                }))
                .when(
                    self.active_tag.is_some()
                        || self.active_series.is_some()
                        || self.status_filter.is_some()
                        || !self.search_query.trim().is_empty(),
                    |row| {
                        row.child(
                            Button::new("lib-clear-filters")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Close)
                                .label("Clear")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.status_filter = None;
                                    this.active_tag = None;
                                    this.active_series = None;
                                    this.clear_search(window, cx);
                                })),
                        )
                    },
                )
        });

        let continue_strip = self.render_continue_reading_strip(cx);
        let filtered = self.filtered_books();
        let catalog_body: AnyElement = if self.books.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p_8()
                .child(
                    v_flex()
                        .max_w(px(420.))
                        .p_6()
                        .gap_3()
                        .items_center()
                        .rounded(px(10.))
                        .bg(sidebar_bg)
                        .border_1()
                        .border_color(border)
                        .child(Icon::new(IconName::Library).size(px(36.)).text_color(violet))
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::BOLD)
                                .text_color(fg)
                                .child("No Books Found in Library"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child(format!(
                                    "Drop .epub or .pdf files into {}, then click Refresh.",
                                    self.workspace_root.display()
                                )),
                        )
                        .child(
                            Button::new("lib-empty-refresh")
                                .primary()
                                .small()
                                .icon(IconName::RefreshCw)
                                .label("Refresh Library")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.refresh_books(cx);
                                })),
                        ),
                )
                .into_any_element()
        } else if filtered.is_empty() {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child("No books match the current filter."),
                )
                .into_any_element()
        } else if self.view_mode == LibraryViewMode::Grid {
            let cards = filtered.into_iter().enumerate().map(|(ix, rec)| {
                let is_sel = self.selected_rel.as_deref() == Some(rec.rel_path.as_str());
                let rel_click = rec.rel_path.clone();
                let open_path = rec.path.clone();
                let cover = self.render_cover_art(&rec, None, 152.0, 220.0, true);
                let title = rec.display_title().to_string();
                let author = rec.authors_display();
                let pct = rec.progress_pct();
                let rating = rec.user.rating;

                v_flex()
                    .id(("lib-card", ix))
                    .w(px(164.))
                    .p_1p5()
                    .gap_1p5()
                    .rounded(px(9.))
                    .bg(if is_sel { panel_bg } else { deep_bg })
                    .border_1()
                    .border_color(if is_sel { accent } else { border })
                    .cursor_pointer()
                    .hover(|s| s.border_color(violet))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, ev: &gpui_kit::MouseDownEvent, window, cx| {
                            if ev.click_count >= 2 {
                                cx.emit(LibraryViewEvent::OpenBook {
                                    path: open_path.clone(),
                                    jump: None,
                                });
                            } else {
                                this.inspector_open = true;
                                this.select_book(&rel_click, window, cx);
                            }
                        }),
                    )
                    .child(cover)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(fg)
                            .truncate()
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(muted)
                            .truncate()
                            .child(author),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(if rating > 0 {
                                        hex_color(kd::YELLOW)
                                    } else {
                                        muted
                                    })
                                    .child(if rating > 0 {
                                        "★".repeat(rating as usize)
                                    } else {
                                        rec.user.status.label().to_string()
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(if pct > 0 { violet } else { muted })
                                    .child(if pct > 0 {
                                        format!("{pct}%")
                                    } else {
                                        format_size_short(rec.file_size_bytes)
                                    }),
                            ),
                    )
            });

            v_flex()
                .id("lib-grid-scroll")
                .track_scroll(&self.grid_scroll)
                .size_full()
                .overflow_y_scrollbar()
                .p_5()
                .when_some(continue_strip, |col, strip| col.child(strip))
                .child(div().flex().flex_row().flex_wrap().gap_4().items_start().children(cards))
                .into_any_element()
        } else {
            let rows = filtered.into_iter().enumerate().map(|(ix, rec)| {
                let is_sel = self.selected_rel.as_deref() == Some(rec.rel_path.as_str());
                let rel_click = rec.rel_path.clone();
                let open_path = rec.path.clone();
                let open_btn_path = rec.path.clone();
                let mini_cov = self.render_cover_art(&rec, None, 32.0, 46.0, false);
                let title = rec.display_title().to_string();
                let series = rec.series_badge();
                let author = rec.authors_display();
                let tags = rec.all_tags();
                let pct = rec.progress_pct();

                h_flex()
                    .id(("lib-row", ix))
                    .w_full()
                    .px_3()
                    .py_2()
                    .gap_3()
                    .items_center()
                    .rounded(px(7.))
                    .bg(if is_sel { panel_bg } else { deep_bg })
                    .border_1()
                    .border_color(if is_sel { accent } else { border })
                    .cursor_pointer()
                    .hover(|s| s.border_color(violet))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, ev: &gpui_kit::MouseDownEvent, window, cx| {
                            if ev.click_count >= 2 {
                                cx.emit(LibraryViewEvent::OpenBook {
                                    path: open_path.clone(),
                                    jump: None,
                                });
                            } else {
                                this.inspector_open = true;
                                this.select_book(&rel_click, window, cx);
                            }
                        }),
                    )
                    .child(mini_cov)
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_0p5()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(fg)
                                            .truncate()
                                            .child(title),
                                    )
                                    .when_some(series, |row, sb| {
                                        row.child(
                                            div()
                                                .px_1p5()
                                                .py_0p5()
                                                .rounded(px(4.))
                                                .bg(sidebar_bg)
                                                .text_size(px(10.))
                                                .text_color(violet)
                                                .child(sb),
                                        )
                                    }),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .truncate()
                                            .child(author),
                                    )
                                    .children(tags.into_iter().take(3).map(|t| {
                                        div()
                                            .px_1()
                                            .rounded(px(3.))
                                            .bg(sidebar_bg)
                                            .text_size(px(10.))
                                            .text_color(muted)
                                            .child(format!("#{t}"))
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .w(px(76.))
                            .text_xs()
                            .text_color(muted)
                            .child(rec.user.status.label()),
                    )
                    .child(
                        div()
                            .w(px(64.))
                            .text_xs()
                            .text_color(if pct > 0 { violet } else { muted })
                            .child(format!("{pct}%")),
                    )
                    .child(
                        Button::new(SharedString::from(format!("lib-row-read-{ix}")))
                            .secondary()
                            .xsmall()
                            .icon(IconName::BookOpen)
                            .label("Read")
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.emit(LibraryViewEvent::OpenBook {
                                    path: open_btn_path.clone(),
                                    jump: None,
                                });
                            })),
                    )
            });

            v_flex()
                .id("lib-list-scroll")
                .track_scroll(&self.grid_scroll)
                .size_full()
                .overflow_y_scrollbar()
                .p_4()
                .when_some(continue_strip, |col, strip| col.child(strip))
                .child(v_flex().gap_2().children(rows))
                .into_any_element()
        };

        let inspector = if self.inspector_open {
            Some(self.render_inspector(cx))
        } else {
            None
        };

        let batch_banner = if self.batch_running {
            let (done, total, cur_title) = self
                .batch_progress
                .as_ref()
                .map(|(d, t, title)| (*d, *t, title.as_str()))
                .unwrap_or((0, 0, ""));
            Some(
                h_flex()
                    .w_full()
                    .px_3()
                    .py_1()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .bg(panel_bg)
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(fg)
                            .truncate()
                            .child(format!(
                                "Matching metadata {done} / {total}: {cur_title}"
                            )),
                    )
                    .child(
                        Button::new("lib-batch-cancel")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .label("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(flag) = &this.batch_cancel {
                                    flag.store(true, Ordering::Relaxed);
                                }
                                cx.notify();
                            })),
                    ),
            )
        } else {
            None
        };

        v_flex()
            .id(SharedString::from(format!(
                "library-view-{:?}",
                cx.entity_id()
            )))
            .size_full()
            .bg(main_bg)
            .track_focus(&self.focus_handle)
            .child(toolbar)
            .when_some(batch_banner, |col, bar| col.child(bar))
            .when_some(chip_bar, |col, bar| col.child(bar))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(div().flex_1().min_w_0().h_full().child(catalog_body))
                    .when_some(inspector, |row, insp| row.child(insp)),
            )
            .into_any_element()
    }
}

pub fn render_library_tab(
    app: &BookwormApp,
    cx: &mut Context<BookwormApp>,
) -> impl IntoElement {
    let _ = cx;
    div().size_full().child(app.library_view.clone()).into_any_element()
}
