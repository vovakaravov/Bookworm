pub mod image_util;
pub mod library_view;
pub mod reader_view;
pub mod settings_view;

use crate::config::{
    BookwormMobileConfig, app_cache_dir, app_storage_dir, books_root_candidates,
    default_books_root, has_external_storage_access, state_root_for,
};
use crate::theme::ReaderTheme;
use bookworm_core::epub::{EpubBlock, EpubBook, create_sample_epub};
use bookworm_core::export_md::export_library_notes_to_dir;
use bookworm_core::library::{
    BookBookmark, BookHighlight, BookRecord, HighlightColor, HighlightDecoration, LibraryStore,
    ReaderThemePref, ReadingStatus, scan_books_cached,
};
use bookworm_core::{can_read_dir, count_direct_matching_files, list_subdirectories};
use gpui::{prelude::*, *};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{Icon, IconName, h_flex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default)]
pub struct SafeArea {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSurface {
    Library,
    Reader,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryFilter {
    All,
    Reading,
    Unread,
    Finished,
}

impl LibraryFilter {
    pub fn all() -> &'static [LibraryFilter] {
        &[Self::All, Self::Reading, Self::Unread, Self::Finished]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Reading => "Reading",
            Self::Unread => "Unread",
            Self::Finished => "Finished",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FolderPickerState {
    pub browsing_dir: PathBuf,
    pub subdirs: Vec<(String, PathBuf)>,
    pub direct_match_count: usize,
    pub error: Option<String>,
    pub create_candidate: Option<PathBuf>,
}

fn expand_picker_path(raw: &str) -> PathBuf {
    let mut trimmed = raw.trim().to_string();
    while trimmed.len() > 1 && trimmed.ends_with('/') {
        trimmed.pop();
    }
    if (trimmed == "~" || trimmed.starts_with("~/"))
        && let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
    {
        return if trimmed == "~" {
            home
        } else {
            home.join(&trimmed[2..])
        };
    }
    PathBuf::from(trimmed)
}

#[derive(Debug, Clone)]
pub struct HighlightEditorDraft {
    pub block_idx: usize,
    pub quote: String,
    pub existing_id: Option<String>,
    pub color: HighlightColor,
    pub style: HighlightDecoration,
}

pub struct ReaderState {
    pub book_rel: String,
    pub book_title: String,
    pub book: EpubBook,
    pub chapter_idx: usize,
    pub show_settings: bool,
    pub show_toc: bool,
    pub show_notes: bool,
    pub notes_color_filter: Option<HighlightColor>,
    pub notes_only_filter: bool,
    pub draft: Option<HighlightEditorDraft>,
    pub rendered_layouts: Rc<RefCell<Vec<(usize, usize, TextLayout)>>>,
    pub decoded_images: RefCell<HashMap<(usize, usize), Arc<Image>>>,
    pub pending_scroll_block: Option<(usize, usize)>,
    pub pending_scroll_fraction: Option<f32>,
    pub scroll_jump_gen: u64,
    pub last_saved_scroll_pct: f32,
    pub scroll_save_scheduled: bool,
}

pub struct BookwormMobileApp {
    pub image_cache: Entity<RetainAllImageCache>,
    pub surface: AppSurface,
    pub safe_area: SafeArea,
    pub has_storage_access: bool,

    pub books: Vec<BookRecord>,
    pub library_scanning: bool,
    pub books_root: PathBuf,
    pub books_root_candidates: Vec<(String, PathBuf)>,
    pub library_state_root: PathBuf,
    pub library_store: LibraryStore,
    pub library_filter: LibraryFilter,
    pub reader: Option<ReaderState>,
    pub last_opened_rel: Option<String>,
    pub folder_picker: Option<FolderPickerState>,

    pub reader_theme: ReaderTheme,
    pub reader_font_size: f32,
    pub two_column_spread: bool,
    pub status_banner: Option<String>,

    pub library_search_input: Entity<InputState>,
    pub highlight_note_input: Entity<InputState>,
    pub folder_picker_input: Entity<InputState>,

    pub library_scroll: ScrollHandle,
    pub reader_scroll: ScrollHandle,
    pub toc_scroll: ScrollHandle,
    pub notes_scroll: ScrollHandle,
    pub settings_scroll: ScrollHandle,
    pub folder_picker_scroll: ScrollHandle,

    last_back_at: Option<Instant>,
    _library_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl BookwormMobileApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let image_cache = RetainAllImageCache::new(cx);
        let cfg = BookwormMobileConfig::load();
        let has_storage_access = has_external_storage_access();

        let books_root = cfg
            .books_root
            .filter(|p| can_read_dir(p))
            .unwrap_or_else(default_books_root);
        let root_candidates = books_root_candidates(&books_root);
        let library_state_root = state_root_for(&books_root);
        let (library_store, initial_conflicts) =
            LibraryStore::load_with_conflict_report(&library_state_root);

        let reader_theme = match library_store.reader_prefs.theme {
            ReaderThemePref::Dark => ReaderTheme::DragonDark,
            ReaderThemePref::Amoled => ReaderTheme::Amoled,
            ReaderThemePref::Sepia => ReaderTheme::Sepia,
            ReaderThemePref::Paper => ReaderTheme::Paper,
        };
        let reader_font_size = library_store.reader_prefs.font_size.clamp(13.0, 28.0);

        let library_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Filter books, authors, tags…"));
        let highlight_note_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Add margin note (optional)…"));
        let folder_picker_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("/sdcard/Documents/Library"));

        let mut subs = Vec::new();
        subs.push(
            cx.subscribe_in(&library_search_input, window, |_, _, ev, window, cx| match ev {
                InputEvent::Focus | InputEvent::Blur => {}
                InputEvent::PressEnter { .. } => {
                    window.blur(cx);
                    gpui_mobile::hide_keyboard();
                    cx.notify();
                }
                InputEvent::Change => cx.notify(),
            }),
        );
        subs.push(
            cx.subscribe_in(&highlight_note_input, window, |this, _, ev, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.reader_save_highlight_draft(window, cx);
                }
            }),
        );
        subs.push(
            cx.subscribe_in(&folder_picker_input, window, |this, _, ev, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.apply_folder_picker_input(window, cx);
                }
            }),
        );

        let weak_app = cx.entity().downgrade();
        subs.push(cx.intercept_keystrokes(move |ev, window, cx| {
            if ev.keystroke.key == "escape"
                && !ev.keystroke.modifiers.control
                && !ev.keystroke.modifiers.alt
                && !ev.keystroke.modifiers.platform
            {
                cx.stop_propagation();
                let _ = weak_app.update(cx, |this, cx| {
                    this.handle_back_action(window, cx);
                });
            }
        }));

        let initial_banner = (initial_conflicts > 0).then(|| {
            format!("Merged {initial_conflicts} synced library conflict(s)")
        });

        let mut app = Self {
            image_cache,
            surface: AppSurface::Library,
            safe_area: Self::query_safe_area(),
            has_storage_access,
            books: Vec::new(),
            library_scanning: false,
            books_root,
            books_root_candidates: root_candidates,
            library_state_root,
            library_store,
            library_filter: LibraryFilter::All,
            reader: None,
            last_opened_rel: None,
            folder_picker: None,
            reader_theme,
            reader_font_size,
            two_column_spread: cfg.two_column_spread,
            status_banner: initial_banner,
            library_search_input,
            highlight_note_input,
            folder_picker_input,
            library_scroll: ScrollHandle::new(),
            reader_scroll: ScrollHandle::new(),
            toc_scroll: ScrollHandle::new(),
            notes_scroll: ScrollHandle::new(),
            settings_scroll: ScrollHandle::new(),
            folder_picker_scroll: ScrollHandle::new(),
            last_back_at: None,
            _library_task: None,
            _subs: subs,
        };

        app.refresh_library_async(cx);
        app
    }

    pub fn query_safe_area() -> SafeArea {
        #[cfg(target_os = "android")]
        {
            use gpui_mobile::android::jni;
            if let Some(platform) = jni::platform()
                && let Some(win) = platform.primary_window()
            {
                let insets = win.safe_area_insets_logical();
                return SafeArea {
                    top: insets.top.max(0.0),
                    bottom: insets.bottom.max(0.0),
                    left: insets.left.max(0.0),
                    right: insets.right.max(0.0),
                };
            }
        }
        SafeArea::default()
    }

    fn persist_mobile_config(&self, cx: &mut Context<Self>) {
        let cfg = BookwormMobileConfig {
            books_root: Some(self.books_root.clone()),
            two_column_spread: self.two_column_spread,
        };
        cx.background_executor()
            .spawn(async move {
                let _ = cfg.save();
            })
            .detach();
    }

    pub fn most_recent_in_progress_book(&self) -> Option<&BookRecord> {
        self.books
            .iter()
            .filter(|b| {
                b.user.status != ReadingStatus::Finished
                    && (b.user.last_opened.is_some()
                        || b.user.chapter_idx > 0
                        || b.user.scroll_pct > 0.01)
            })
            .max_by(|a, b| a.user.last_opened.cmp(&b.user.last_opened))
    }

    pub fn refresh_library_async(&mut self, cx: &mut Context<Self>) {
        let books_root = self.books_root.clone();
        self.library_scanning = true;
        cx.notify();

        self._library_task = Some(cx.spawn(async move |weak_self, cx| {
            let (scanned_root, state_root, store, merged_conflicts, books, candidates) = cx
                .background_executor()
                .spawn(async move {
                    let state_root = state_root_for(&books_root);
                    let cover_cache_dir = app_cache_dir("covers");
                    let (store, merged_conflicts) =
                        LibraryStore::load_with_conflict_report(&state_root);
                    let books = scan_books_cached(
                        std::slice::from_ref(&books_root),
                        &state_root,
                        &store,
                        &cover_cache_dir,
                    );
                    let candidates = books_root_candidates(&books_root);
                    (
                        books_root,
                        state_root,
                        store,
                        merged_conflicts,
                        books,
                        candidates,
                    )
                })
                .await;

            _ = weak_self.update(cx, |this, cx| {
                if this.books_root == scanned_root {
                    this.library_state_root = state_root;
                    this.library_store = store;
                    this.books = books;
                    this.books_root_candidates = candidates;
                    this.library_scanning = false;
                    if merged_conflicts > 0 {
                        this.status_banner = Some(format!(
                            "Merged {merged_conflicts} synced library conflict(s)"
                        ));
                    }
                    cx.notify();
                }
            });
        }));
    }

    pub fn recheck_storage_and_refresh(&mut self, cx: &mut Context<Self>) {
        let access = has_external_storage_access();
        self.has_storage_access = access;
        let fallback_books = app_storage_dir("books");
        if access && (!can_read_dir(&self.books_root) || self.books_root == fallback_books) {
            let detected = default_books_root();
            self.books_root = detected.clone();
            self.library_state_root = state_root_for(&detected);
            self.library_store = LibraryStore::load(&self.library_state_root);
        }
        self.books_root_candidates = books_root_candidates(&self.books_root);
        self.persist_mobile_config(cx);
        self.refresh_library_async(cx);
    }

    pub fn set_books_root(&mut self, new_root: PathBuf, cx: &mut Context<Self>) {
        if !can_read_dir(&new_root) {
            self.status_banner = Some(format!("Cannot read {}", new_root.display()));
            cx.notify();
            return;
        }
        self.books_root = new_root.clone();
        self.library_state_root = state_root_for(&new_root);
        self.library_store = LibraryStore::load(&self.library_state_root);
        self.books.clear();
        self.reader = None;
        self.books_root_candidates = books_root_candidates(&new_root);
        self.status_banner = Some(format!("Books Root: {}", self.books_root.display()));
        self.persist_mobile_config(cx);
        self.refresh_library_async(cx);
    }

    pub fn export_notes_to_markdown(&mut self, cx: &mut Context<Self>) {
        let out_dir = self.books_root.join("Bookworm Notes");
        match export_library_notes_to_dir(&self.books, &out_dir) {
            Ok(0) => {
                self.status_banner = Some("No highlights or bookmarks to export yet.".to_string());
            }
            Ok(n) => {
                self.status_banner = Some(format!(
                    "Exported {n} Markdown file(s) to {}",
                    out_dir.display()
                ));
            }
            Err(err) => {
                self.status_banner = Some(format!("Export failed: {err}"));
            }
        }
        cx.notify();
    }

    pub fn open_surface_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        self.surface = AppSurface::Library;
        cx.notify();
    }

    pub fn open_surface_reader_or_resume(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.reader.is_some() {
            window.blur(cx);
            gpui_mobile::hide_keyboard();
            self.surface = AppSurface::Reader;
            cx.notify();
            return;
        }
        if let Some(rel) = self
            .last_opened_rel
            .clone()
            .or_else(|| self.most_recent_in_progress_book().map(|b| b.rel_path.clone()))
            .or_else(|| self.books.first().map(|b| b.rel_path.clone()))
        {
            self.open_book(&rel, window, cx);
        }
    }

    pub fn open_surface_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        self.surface = AppSurface::Settings;
        cx.notify();
    }

    pub fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_surface_library(window, cx);
    }

    pub fn open_folder_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.blur(cx);
        gpui_mobile::hide_keyboard();

        let browsing_dir = if can_read_dir(&self.books_root) {
            self.books_root.clone()
        } else {
            #[cfg(target_os = "android")]
            {
                let sd = PathBuf::from("/sdcard");
                if can_read_dir(&sd) {
                    sd
                } else {
                    app_storage_dir("")
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .filter(|p| can_read_dir(p))
                    .unwrap_or_else(|| app_storage_dir(""))
            }
        };

        let subdirs = list_subdirectories(&browsing_dir);
        let direct_match_count = count_direct_matching_files(&browsing_dir, &["epub"]);
        let path_str = browsing_dir.to_string_lossy().into_owned();
        self.folder_picker_input
            .update(cx, |inp, cx| inp.set_value(path_str, window, cx));
        self.folder_picker_scroll.set_offset(point(px(0.), px(0.)));
        self.folder_picker = Some(FolderPickerState {
            browsing_dir,
            subdirs,
            direct_match_count,
            error: None,
            create_candidate: None,
        });
        cx.notify();
    }

    pub fn browse_folder_picker_to(
        &mut self,
        dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(picker) = self.folder_picker.as_mut() else {
            return;
        };
        if !can_read_dir(&dir) {
            picker.error = Some(format!("Cannot read {}", dir.display()));
            picker.create_candidate = None;
            cx.notify();
            return;
        }
        picker.browsing_dir = dir.clone();
        picker.subdirs = list_subdirectories(&dir);
        picker.direct_match_count = count_direct_matching_files(&dir, &["epub"]);
        picker.error = None;
        picker.create_candidate = None;
        let path_str = dir.to_string_lossy().into_owned();
        self.folder_picker_input
            .update(cx, |inp, cx| inp.set_value(path_str, window, cx));
        self.folder_picker_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    pub fn folder_picker_go_up(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.folder_picker.as_ref().map(|p| p.browsing_dir.clone()) else {
            return;
        };
        if let Some(parent) = current.parent().map(Path::to_path_buf) {
            if can_read_dir(&parent) {
                self.browse_folder_picker_to(parent, window, cx);
                return;
            }
            #[cfg(target_os = "android")]
            {
                let sdcard = PathBuf::from("/sdcard");
                if current != sdcard && current.starts_with(&sdcard) && can_read_dir(&sdcard) {
                    self.browse_folder_picker_to(sdcard, window, cx);
                    return;
                }
            }
            if let Some(p) = self.folder_picker.as_mut() {
                p.error = Some(format!(
                    "Already at top accessible folder ({})",
                    current.display()
                ));
                p.create_candidate = None;
                cx.notify();
            }
        }
    }

    pub fn apply_folder_picker_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.folder_picker_input.read(cx).value().to_string();
        let Some(fallback_dir) = self.folder_picker.as_ref().map(|p| p.browsing_dir.clone()) else {
            return;
        };

        let candidate = if raw.trim().is_empty() {
            fallback_dir
        } else {
            expand_picker_path(&raw)
        };

        if can_read_dir(&candidate) {
            self.folder_picker = None;
            window.blur(cx);
            gpui_mobile::hide_keyboard();
            self.set_books_root(candidate, cx);
            return;
        }

        if let Some(p) = self.folder_picker.as_mut() {
            if !candidate.exists() {
                p.error = Some(format!("Folder does not exist: {}", candidate.display()));
                p.create_candidate = Some(candidate);
            } else {
                p.error = Some(format!(
                    "Cannot read folder {} (check path or storage permission)",
                    candidate.display()
                ));
                p.create_candidate = None;
            }
        }
        cx.notify();
    }

    pub fn create_and_apply_folder_picker_path(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self
            .folder_picker
            .as_ref()
            .and_then(|p| p.create_candidate.clone())
        else {
            return;
        };
        match fs::create_dir_all(&path) {
            Ok(()) if can_read_dir(&path) => {
                self.folder_picker = None;
                window.blur(cx);
                gpui_mobile::hide_keyboard();
                self.set_books_root(path, cx);
            }
            Ok(()) => {
                if let Some(p) = self.folder_picker.as_mut() {
                    p.error = Some(format!(
                        "Created {}, but folder is not readable",
                        path.display()
                    ));
                    p.create_candidate = None;
                }
                cx.notify();
            }
            Err(err) => {
                if let Some(p) = self.folder_picker.as_mut() {
                    p.error = Some(format!("Create failed: {err}"));
                }
                cx.notify();
            }
        }
    }

    pub fn close_folder_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.folder_picker = None;
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        cx.notify();
    }

    pub fn handle_back_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        if let Some(prev) = self.last_back_at
            && now.duration_since(prev) < Duration::from_millis(120)
        {
            return;
        }
        self.last_back_at = Some(now);

        if self.folder_picker.is_some() {
            self.close_folder_picker(window, cx);
            return;
        }

        match self.surface {
            AppSurface::Reader => {
                if let Some(r) = &mut self.reader {
                    if r.draft.take().is_some() {
                        window.blur(cx);
                        gpui_mobile::hide_keyboard();
                        cx.notify();
                        return;
                    }
                    if r.show_toc {
                        r.show_toc = false;
                        cx.notify();
                        return;
                    }
                    if r.show_notes {
                        r.show_notes = false;
                        cx.notify();
                        return;
                    }
                    if r.show_settings {
                        r.show_settings = false;
                        cx.notify();
                        return;
                    }
                }
                self.close_reader_to_library(window, cx);
            }
            AppSurface::Settings => {
                self.close_settings(window, cx);
            }
            AppSurface::Library => {
                if !self.library_search_input.read(cx).value().is_empty() {
                    self.library_search_input
                        .update(cx, |inp, cx| inp.set_value("", window, cx));
                    window.blur(cx);
                    gpui_mobile::hide_keyboard();
                    cx.notify();
                } else {
                    crate::minimize_android_app();
                }
            }
        }
    }

    pub fn create_sample_book(&mut self, cx: &mut Context<Self>) {
        let target_dir = if can_read_dir(&self.books_root) {
            self.books_root.clone()
        } else {
            app_storage_dir("books")
        };
        let sample_path = target_dir.join("Notes from the Signal.epub");
        match create_sample_epub(&sample_path) {
            Ok(()) => {
                self.status_banner = Some("Created sample EPUB in library!".to_string());
                self.refresh_library_async(cx);
            }
            Err(err) => {
                self.status_banner = Some(format!("Failed to create sample EPUB: {err}"));
                cx.notify();
            }
        }
    }

    pub(crate) fn compute_block_scroll_target_y(
        block_win_y: f32,
        viewport_top_y: f32,
        current_offset_y: f32,
        max_scroll_y: f32,
        top_margin_px: f32,
    ) -> f32 {
        if max_scroll_y <= 0.0 {
            return 0.0;
        }
        let content_y = (block_win_y - viewport_top_y) - current_offset_y;
        (content_y - top_margin_px.max(0.0)).clamp(0.0, max_scroll_y)
    }

    pub fn reader_current_scroll_pct(&self) -> f32 {
        let max_y = f32::from(self.reader_scroll.max_offset().y);
        if max_y <= 0.0 {
            0.0
        } else {
            (-f32::from(self.reader_scroll.offset().y) / max_y).clamp(0.0, 1.0)
        }
    }

    fn persist_reader_progress(&mut self, scroll_pct: f32) {
        let clamped = scroll_pct.clamp(0.0, 1.0);
        let Some(r) = &mut self.reader else {
            return;
        };
        let already_finished = self
            .library_store
            .books
            .get(&r.book_rel)
            .is_some_and(|u| u.status == ReadingStatus::Finished);
        let total = r.book.chapters.len().max(1);
        self.library_store.update_progress(
            &r.book_rel,
            r.chapter_idx,
            total,
            clamped,
            already_finished,
        );
        r.last_saved_scroll_pct = clamped;
        let _ = self.library_store.sync_save(&self.library_state_root);
    }

    pub fn maybe_schedule_reader_scroll_save(&mut self, cx: &mut Context<Self>) {
        let cur = self.reader_current_scroll_pct();
        let Some(r) = &mut self.reader else {
            return;
        };
        if r.show_toc
            || r.show_notes
            || r.scroll_save_scheduled
            || r.pending_scroll_block.is_some()
            || r.pending_scroll_fraction.is_some()
            || (cur - r.last_saved_scroll_pct).abs() < 0.04
        {
            return;
        }
        r.scroll_save_scheduled = true;
        let book_rel = r.book_rel.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(600))
                .await;
            let _ = this.update(cx, |app, _cx| {
                let cur_now = app.reader_current_scroll_pct();
                let should_save = if let Some(r) = &mut app.reader {
                    if r.book_rel == book_rel {
                        r.scroll_save_scheduled = false;
                        !r.show_toc
                            && !r.show_notes
                            && r.pending_scroll_block.is_none()
                            && r.pending_scroll_fraction.is_none()
                    } else {
                        false
                    }
                } else {
                    false
                };
                if should_save {
                    app.persist_reader_progress(cur_now);
                }
            });
        })
        .detach();
    }

    fn apply_pending_reader_scroll(&mut self) -> bool {
        let Some(r) = &mut self.reader else {
            return true;
        };
        if r.pending_scroll_block.is_none() && r.pending_scroll_fraction.is_none() {
            return true;
        }
        if r.show_toc || r.show_notes {
            return false;
        }
        let scroll_bounds = self.reader_scroll.bounds();
        let vp_h = f32::from(scroll_bounds.size.height);
        let ch_has_text = r.book.chapters.get(r.chapter_idx).is_some_and(|c| {
            c.blocks.iter().any(|b| {
                matches!(
                    b,
                    EpubBlock::Heading { .. }
                        | EpubBlock::Paragraph { .. }
                        | EpubBlock::ListItem { .. }
                )
            })
        });
        if vp_h <= 0.0 || (ch_has_text && r.rendered_layouts.borrow().is_empty()) {
            return false;
        }

        let max_y = f32::from(self.reader_scroll.max_offset().y);
        if let Some((target_blk, target_byte)) = r.pending_scroll_block {
            let layouts = r.rendered_layouts.borrow();
            if let Some((_, len, layout)) = layouts.iter().find(|(b, _, _)| *b == target_blk) {
                let win_y = if target_byte > 0 {
                    layout
                        .position_for_index(target_byte.min(*len))
                        .map(|p| f32::from(p.y))
                        .unwrap_or_else(|| f32::from(layout.bounds().origin.y))
                } else {
                    f32::from(layout.bounds().origin.y)
                };
                let cur_off_y = f32::from(self.reader_scroll.offset().y);
                let vp_top_y = f32::from(scroll_bounds.top());
                let top_margin = (vp_h * 0.12).clamp(24.0, 56.0);
                let target_y = Self::compute_block_scroll_target_y(
                    win_y,
                    vp_top_y,
                    cur_off_y,
                    max_y,
                    top_margin,
                );
                drop(layouts);
                r.pending_scroll_block = None;
                r.pending_scroll_fraction = None;
                if max_y > 0.0 {
                    self.reader_scroll.set_offset(point(px(0.), px(-target_y)));
                }
                r.last_saved_scroll_pct = if max_y > 0.0 {
                    (target_y / max_y).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return true;
            }
            drop(layouts);
            r.pending_scroll_block = None;
        }

        if let Some(frac) = r.pending_scroll_fraction.take() {
            let clamped = frac.clamp(0.0, 1.0);
            if max_y > 0.0 {
                let target_y = (clamped * max_y).clamp(0.0, max_y);
                self.reader_scroll.set_offset(point(px(0.), px(-target_y)));
            }
            r.last_saved_scroll_pct = clamped;
        }
        true
    }

    fn schedule_pending_reader_scroll(&mut self, cx: &mut Context<Self>) {
        let Some(r) = &mut self.reader else {
            return;
        };
        r.scroll_jump_gen = r.scroll_jump_gen.wrapping_add(1);
        let book_rel = r.book_rel.clone();
        let jump_gen = r.scroll_jump_gen;
        cx.spawn(async move |this, cx| {
            for delay_ms in [16u64, 32, 64, 128, 250] {
                cx.background_executor()
                    .timer(Duration::from_millis(delay_ms))
                    .await;
                let mut stop = false;
                let updated = this.update(cx, |app, cx| {
                    let matches_jump = app
                        .reader
                        .as_ref()
                        .is_some_and(|r| r.book_rel == book_rel && r.scroll_jump_gen == jump_gen);
                    if !matches_jump {
                        stop = true;
                        return;
                    }
                    if app.apply_pending_reader_scroll() {
                        let pct = app.reader_current_scroll_pct();
                        app.persist_reader_progress(pct);
                        cx.notify();
                        stop = true;
                    } else {
                        cx.notify();
                    }
                });
                if updated.is_err() || stop {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn open_book(&mut self, rel_path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rec) = self.books.iter().find(|b| b.rel_path == rel_path).cloned() else {
            return;
        };
        let (loaded_store, mut merged_conflicts) =
            LibraryStore::load_with_conflict_report(&self.library_state_root);
        if LibraryStore::state_path(&self.library_state_root).is_file() || merged_conflicts > 0 {
            self.library_store = loaded_store;
        }
        let user_state = self
            .library_store
            .books
            .get(&rec.rel_path)
            .cloned()
            .unwrap_or_else(|| rec.user.clone());
        let book = match EpubBook::load(&rec.path) {
            Ok(b) => b,
            Err(err) => {
                self.status_banner = Some(format!("Failed to open EPUB: {err}"));
                cx.notify();
                return;
            }
        };
        let max_ch = book.chapters.len().saturating_sub(1);
        let start_ch = user_state.chapter_idx.min(max_ch);
        let restored_scroll = user_state.scroll_pct.clamp(0.0, 1.0);
        let already_finished = user_state.status == ReadingStatus::Finished;
        let book_title = rec.display_title().to_string();
        let total = book.chapters.len().max(1);
        self.library_store.update_progress(
            &rec.rel_path,
            start_ch,
            total,
            restored_scroll,
            already_finished,
        );
        if let Ok(extra_conflicts) = self.library_store.sync_save(&self.library_state_root) {
            merged_conflicts += extra_conflicts;
        }
        if merged_conflicts > 0 {
            self.status_banner = Some(format!(
                "Merged {merged_conflicts} synced library conflict(s)"
            ));
        }

        self.reader_scroll.set_offset(point(px(0.), px(0.)));
        self.notes_scroll.set_offset(point(px(0.), px(0.)));
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        let pending_scroll_fraction = (restored_scroll > 0.005).then_some(restored_scroll);
        self.last_opened_rel = Some(rec.rel_path.clone());
        self.reader = Some(ReaderState {
            book_rel: rec.rel_path,
            book_title,
            book,
            chapter_idx: start_ch,
            show_settings: false,
            show_toc: false,
            show_notes: false,
            notes_color_filter: None,
            notes_only_filter: false,
            draft: None,
            rendered_layouts: Rc::new(RefCell::new(Vec::new())),
            decoded_images: RefCell::new(HashMap::new()),
            pending_scroll_block: None,
            pending_scroll_fraction,
            scroll_jump_gen: 0,
            last_saved_scroll_pct: restored_scroll,
            scroll_save_scheduled: false,
        });
        if pending_scroll_fraction.is_some() {
            self.schedule_pending_reader_scroll(cx);
        }
        self.surface = AppSurface::Reader;
        cx.notify();
    }

    pub fn reader_go_chapter(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = &mut self.reader {
            if r.book.chapters.is_empty() {
                return;
            }
            let clamped = idx.min(r.book.chapters.len() - 1);
            r.chapter_idx = clamped;
            r.show_toc = false;
            r.show_notes = false;
            r.draft = None;
            r.rendered_layouts.borrow_mut().clear();
            r.pending_scroll_block = None;
            r.pending_scroll_fraction = None;
            r.scroll_jump_gen = r.scroll_jump_gen.wrapping_add(1);
            self.reader_scroll.set_offset(point(px(0.), px(0.)));
            self.persist_reader_progress(0.0);
            window.blur(cx);
            gpui_mobile::hide_keyboard();
        }
        cx.notify();
    }

    pub fn reader_jump_to_highlight(
        &mut self,
        hl: &BookHighlight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(r) = &mut self.reader {
            if r.book.chapters.is_empty() {
                return;
            }
            let target_ch = hl.chapter_idx.min(r.book.chapters.len() - 1);
            let total_blocks = r
                .book
                .chapters
                .get(target_ch)
                .map(|c| c.blocks.len().max(1))
                .unwrap_or(1);
            let frac = ((hl.start_block as f32 / total_blocks as f32) - 0.06).clamp(0.0, 1.0);
            r.chapter_idx = target_ch;
            r.show_toc = false;
            r.show_notes = false;
            r.draft = None;
            r.rendered_layouts.borrow_mut().clear();
            r.pending_scroll_block = Some((hl.start_block, hl.start_offset));
            r.pending_scroll_fraction = Some(frac);
            self.reader_scroll.set_offset(point(px(0.), px(0.)));
            self.persist_reader_progress(frac);
            self.schedule_pending_reader_scroll(cx);
            window.blur(cx);
            gpui_mobile::hide_keyboard();
        }
        cx.notify();
    }

    pub fn reader_jump_to_bookmark(
        &mut self,
        bm: &BookBookmark,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(r) = &mut self.reader {
            if r.book.chapters.is_empty() {
                return;
            }
            let target_ch = bm.chapter_idx.min(r.book.chapters.len() - 1);
            let frac = bm.scroll_pct.clamp(0.0, 1.0);
            r.chapter_idx = target_ch;
            r.show_toc = false;
            r.show_notes = false;
            r.draft = None;
            r.rendered_layouts.borrow_mut().clear();
            r.pending_scroll_block = None;
            r.pending_scroll_fraction = Some(frac);
            self.reader_scroll.set_offset(point(px(0.), px(0.)));
            self.persist_reader_progress(frac);
            self.schedule_pending_reader_scroll(cx);
            window.blur(cx);
            gpui_mobile::hide_keyboard();
        }
        cx.notify();
    }

    pub fn reader_open_highlight_draft(
        &mut self,
        block_idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(r) = &mut self.reader {
            let Some(blk) = r
                .book
                .chapters
                .get(r.chapter_idx)
                .and_then(|c| c.blocks.get(block_idx))
            else {
                return;
            };
            let quote = match blk {
                EpubBlock::Paragraph { text, .. }
                | EpubBlock::ListItem { text, .. }
                | EpubBlock::Heading { text, .. } => text.clone(),
                EpubBlock::Image { .. } | EpubBlock::Rule => return,
            };
            let existing = self.library_store.books.get(&r.book_rel).and_then(|u| {
                u.highlights.iter().find(|h| {
                    h.chapter_idx == r.chapter_idx
                        && block_idx >= h.start_block
                        && block_idx <= h.end_block
                })
            });

            let (existing_id, color, style, note_txt) = if let Some(hl) = existing {
                (Some(hl.id.clone()), hl.color, hl.style, hl.note.clone())
            } else {
                (
                    None,
                    self.library_store.reader_prefs.last_highlight_color,
                    self.library_store.reader_prefs.last_highlight_style,
                    String::new(),
                )
            };

            self.highlight_note_input
                .update(cx, |inp, cx| inp.set_value(note_txt, window, cx));
            r.draft = Some(HighlightEditorDraft {
                block_idx,
                quote,
                existing_id,
                color,
                style,
            });
        }
        cx.notify();
    }

    pub fn reader_save_highlight_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let note_text = self.highlight_note_input.read(cx).value().trim().to_string();
        if let Some(r) = &mut self.reader
            && let Some(d) = r.draft.take()
        {
            let ch_title = r
                .book
                .chapters
                .get(r.chapter_idx)
                .map(|c| c.title.clone())
                .unwrap_or_default();
            let now = chrono::Utc::now().to_rfc3339();
            let id = d.existing_id.unwrap_or_else(|| {
                format!(
                    "hl-{}-{}-{}",
                    r.chapter_idx,
                    d.block_idx,
                    chrono::Utc::now().timestamp_millis()
                )
            });
            let end_offset = d.quote.len();
            self.library_store.reader_prefs.last_highlight_color = d.color;
            self.library_store.reader_prefs.last_highlight_style = d.style;
            self.library_store.upsert_highlight(
                &r.book_rel,
                BookHighlight {
                    id,
                    chapter_idx: r.chapter_idx,
                    chapter_title: ch_title,
                    start_block: d.block_idx,
                    start_offset: 0,
                    end_block: d.block_idx,
                    end_offset,
                    quote: d.quote,
                    color: d.color,
                    style: d.style,
                    note: note_text,
                    created_at: now.clone(),
                    updated_at: now,
                },
            );
            let _ = self.library_store.sync_save(&self.library_state_root);
        }
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        cx.notify();
    }

    pub fn reader_delete_draft_highlight(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = &mut self.reader
            && let Some(d) = r.draft.take()
            && let Some(id) = d.existing_id
        {
            self.library_store.delete_highlight(&r.book_rel, &id);
            let _ = self.library_store.sync_save(&self.library_state_root);
        }
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        cx.notify();
    }

    pub fn reader_toggle_bookmark(&mut self, cx: &mut Context<Self>) {
        let scroll_pct = self.reader_current_scroll_pct();
        if let Some(r) = &self.reader {
            let ch_title = r
                .book
                .chapters
                .get(r.chapter_idx)
                .map(|c| c.title.as_str())
                .unwrap_or("Chapter");
            self.library_store
                .toggle_bookmark(&r.book_rel, r.chapter_idx, ch_title, scroll_pct);
            let _ = self.library_store.sync_save(&self.library_state_root);
        }
        cx.notify();
    }

    pub fn set_reader_theme(&mut self, theme: ReaderTheme, cx: &mut Context<Self>) {
        self.reader_theme = theme;
        self.library_store.reader_prefs.theme = match theme {
            ReaderTheme::DragonDark => ReaderThemePref::Dark,
            ReaderTheme::Amoled => ReaderThemePref::Amoled,
            ReaderTheme::Sepia => ReaderThemePref::Sepia,
            ReaderTheme::Paper => ReaderThemePref::Paper,
        };
        let _ = self.library_store.sync_save(&self.library_state_root);
        cx.notify();
    }

    pub fn adjust_reader_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.reader_font_size = (self.reader_font_size + delta).clamp(13.0, 28.0);
        self.library_store.reader_prefs.font_size = self.reader_font_size;
        let _ = self.library_store.sync_save(&self.library_state_root);
        cx.notify();
    }

    pub fn toggle_two_column_spread(&mut self, cx: &mut Context<Self>) {
        self.two_column_spread = !self.two_column_spread;
        self.persist_mobile_config(cx);
        cx.notify();
    }

    pub fn close_reader_to_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        let should_save = self.reader.as_ref().is_some_and(|r| {
            !r.show_toc
                && !r.show_notes
                && r.pending_scroll_block.is_none()
                && r.pending_scroll_fraction.is_none()
        });
        if should_save {
            let pct = self.reader_current_scroll_pct();
            self.persist_reader_progress(pct);
        }
        if let Some(r) = &self.reader
            && let Some(book_rec) = self.books.iter_mut().find(|b| b.rel_path == r.book_rel)
            && let Some(user_state) = self.library_store.books.get(&r.book_rel)
        {
            book_rec.user = user_state.clone();
        }
        self.surface = AppSurface::Library;
        cx.notify();
    }
}

fn nav_tab_item(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    active: bool,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let fg = if disabled {
        crate::theme::kd::FG_MUTED
    } else if active {
        crate::theme::kd::GREEN
    } else {
        crate::theme::kd::FG_SUBTLE
    };
    div()
        .id(id)
        .flex_1()
        .h(px(48.))
        .flex()
        .items_center()
        .justify_center()
        .gap_1p5()
        .when(!disabled, |el| el.cursor_pointer().on_click(on_click))
        .child(Icon::new(icon).size(px(16.)).text_color(crate::theme::hex_color(fg)))
        .child(
            div()
                .text_xs()
                .font_weight(if active {
                    FontWeight::BOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(crate::theme::hex_color(fg))
                .child(label),
        )
}

impl Render for BookwormMobileApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.safe_area = Self::query_safe_area();
        let kb_height = gpui_mobile::keyboard_height();
        let bottom_inset = self.safe_area.bottom.max(kb_height);

        let top_bg = match self.surface {
            AppSurface::Reader => self.reader_theme.palette().bg,
            _ => crate::theme::kd::BG_DEEP,
        };

        let body = match self.surface {
            AppSurface::Library => {
                library_view::render_library_tab(self, window, cx).into_any_element()
            }
            AppSurface::Reader => {
                reader_view::render_reader_view(self, window, cx).into_any_element()
            }
            AppSurface::Settings => {
                settings_view::render_settings_tab(self, cx).into_any_element()
            }
        };

        let can_open_reader = self.reader.is_some()
            || self.most_recent_in_progress_book().is_some()
            || !self.books.is_empty();

        let show_bottom_nav = self.surface != AppSurface::Reader;

        div()
            .image_cache(self.image_cache.clone())
            .size_full()
            .flex()
            .flex_col()
            .bg(crate::theme::hex_color(crate::theme::kd::BG_MAIN))
            .text_color(crate::theme::hex_color(crate::theme::kd::FG))
            .child(
                div()
                    .w_full()
                    .h(px(self.safe_area.top))
                    .flex_shrink_0()
                    .bg(crate::theme::hex_color(top_bg)),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .pl(px(self.safe_area.left))
                    .pr(px(self.safe_area.right))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(body),
                    )
                    .when(show_bottom_nav, |col| {
                        col.child(
                            h_flex()
                                .w_full()
                                .flex_shrink_0()
                                .bg(crate::theme::hex_color(crate::theme::kd::BG_DEEP))
                                .border_t_1()
                                .border_color(crate::theme::hex_color(
                                    crate::theme::kd::BORDER_SUBTLE,
                                ))
                                .child(nav_tab_item(
                                    "nav-library",
                                    IconName::Folder,
                                    "Library",
                                    self.surface == AppSurface::Library,
                                    false,
                                    cx.listener(|this, _, window, cx| {
                                        this.open_surface_library(window, cx);
                                    }),
                                ))
                                .child(nav_tab_item(
                                    "nav-reader",
                                    IconName::BookOpen,
                                    "Reader",
                                    self.surface == AppSurface::Reader,
                                    !can_open_reader,
                                    cx.listener(|this, _, window, cx| {
                                        this.open_surface_reader_or_resume(window, cx);
                                    }),
                                ))
                                .child(nav_tab_item(
                                    "nav-settings",
                                    IconName::Settings,
                                    "Settings",
                                    self.surface == AppSurface::Settings,
                                    false,
                                    cx.listener(|this, _, window, cx| {
                                        this.open_surface_settings(window, cx);
                                    }),
                                )),
                        )
                        .when(self.safe_area.bottom > 0.0, |c| {
                            c.child(
                                div()
                                    .w_full()
                                    .h(px(self.safe_area.bottom))
                                    .flex_shrink_0()
                                    .bg(crate::theme::hex_color(crate::theme::kd::BG_DEEP)),
                            )
                        })
                    })
                    .when(self.folder_picker.is_some(), |frame| {
                        frame.child(settings_view::render_folder_picker_modal(
                            self,
                            bottom_inset,
                            cx,
                        ))
                    }),
            )
    }
}
