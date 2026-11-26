pub mod epub_view;
pub mod find_bar;
pub mod image_util;
pub mod library_view;

use crate::config::BookwormConfig;
use crate::theme::{accent_alpha, hex_color, kd};
use bookworm_core::epub::EpubBook;
use bookworm_core::export_md::export_library_notes_to_dir;
use bookworm_core::library::{LibraryStore, ReadingStatus, library_state_root_for};
use bookworm_core::normalize_rel_key;
use epub_view::{EpubView, EpubViewEvent, render_epub_tab};
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, Focusable, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement,
    PathPromptOptions, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Window, actions, div, prelude::FluentBuilder as _, px,
};
use library_view::{BookJumpTarget, LibraryView, LibraryViewEvent, render_library_tab};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static NEXT_TAB_ID: AtomicUsize = AtomicUsize::new(1);

fn next_tab_id() -> usize {
    NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed)
}

actions!(
    bookworm,
    [
        FindInTab,
        CloseTab,
        NextTab,
        PrevTab,
        ToggleOutline,
        OpenFolderDialog,
        EscapeModal,
    ]
);

pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-f", FindInTab, None),
        KeyBinding::new("ctrl-w", CloseTab, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PrevTab, None),
        KeyBinding::new("ctrl-shift-b", ToggleOutline, None),
        KeyBinding::new("ctrl-shift-o", OpenFolderDialog, None),
        KeyBinding::new("escape", EscapeModal, None),
    ]);
}

pub struct BookTab {
    pub id: usize,
    pub path: PathBuf,
    pub rel_path: String,
    pub title: String,
    pub file_size_bytes: u64,
    pub view: Option<Entity<EpubView>>,
    _sub: Subscription,
}

#[derive(Clone)]
struct DraggedBookTab {
    from_idx: usize,
    title: String,
}

struct DraggedBookTabPreview {
    title: String,
}

impl Render for DraggedBookTabPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(28.))
            .px_2p5()
            .gap_1p5()
            .items_center()
            .rounded(px(6.))
            .bg(hex_color(kd::BG_MAIN))
            .border_1()
            .border_color(hex_color(kd::GREEN))
            .opacity(0.92)
            .child(
                div()
                    .px_1()
                    .rounded(px(3.))
                    .bg(hex_color(kd::BG_PANEL))
                    .text_size(px(9.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(hex_color(kd::GREEN))
                    .child("EPUB"),
            )
            .child(
                div()
                    .max_w(px(160.))
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(hex_color(kd::FG))
                    .truncate()
                    .child(self.title.clone()),
            )
    }
}

pub struct BookwormApp {
    pub config: BookwormConfig,
    pub workspace_root: PathBuf,
    pub library_view: Entity<LibraryView>,
    _library_sub: Subscription,
    pub tabs: Vec<BookTab>,
    /// `None` = pinned Library tab (Tab 0); `Some(idx)` = open Book tab index.
    pub active_tab: Option<usize>,
    pub outline_scroll: ScrollHandle,
    focus_handle: FocusHandle,
    toast: Option<(String, bool)>,
    toast_gen: u64,
}

impl Focusable for BookwormApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BookwormApp {
    pub fn new(
        initial_target: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut config = BookwormConfig::load_or_default();
        if let Some(target) = &initial_target
            && target.is_dir()
        {
            config.library_root = Some(target.clone());
            let _ = config.save();
        }
        let configured_root = config.effective_library_root();
        let workspace_root =
            library_state_root_for(&configured_root, &BookwormConfig::fallback_books_dir());

        let lib_root = workspace_root.clone();
        let library_view = cx.new(|cx| LibraryView::new(lib_root, window, cx));
        let initial_conflicts = library_view.read(cx).initial_conflicts;

        let library_sub = cx.subscribe_in(&library_view, window, |app, _, event, window, cx| {
            match event {
                LibraryViewEvent::OpenBook { path, jump } => {
                    app.open_book(path.clone(), jump.as_ref(), window, cx);
                }
                LibraryViewEvent::MetadataSaved { path } => {
                    app.reload_open_book(path, cx);
                }
                LibraryViewEvent::BookRenamed { old_path, new_path } => {
                    app.relocate_open_book(old_path, new_path, cx);
                }
                LibraryViewEvent::ExportNotes => {
                    app.export_notes_to_markdown(cx);
                }
                LibraryViewEvent::StatusToast(msg) => {
                    app.show_toast(msg.clone(), false, cx);
                }
                LibraryViewEvent::ErrorToast(msg) => {
                    app.show_toast(msg.clone(), true, cx);
                }
            }
        });

        let mut app = Self {
            config,
            workspace_root,
            library_view,
            _library_sub: library_sub,
            tabs: Vec::new(),
            active_tab: None,
            outline_scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            toast: None,
            toast_gen: 0,
        };

        if initial_conflicts > 0 {
            app.show_toast(
                format!("Merged {initial_conflicts} Syncthing library conflict(s)"),
                false,
                cx,
            );
        }

        if let Some(target) = initial_target
            && target.is_file()
        {
            app.open_book(target, None, window, cx);
        } else {
            app.focus_handle.focus(window, cx);
        }

        app
    }

    fn show_toast(&mut self, msg: String, is_error: bool, cx: &mut Context<Self>) {
        self.toast = Some((msg, is_error));
        self.toast_gen = self.toast_gen.wrapping_add(1);
        let toast_id = self.toast_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(4)).await;
            let _ = this.update(cx, |app, cx| {
                if app.toast_gen == toast_id {
                    app.toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn sync_library_view_store(&self, store: LibraryStore, cx: &mut Context<Self>) {
        self.library_view
            .update(cx, |lv, cx| lv.sync_from_store(store, cx));
    }

    fn open_book(
        &mut self,
        path: PathBuf,
        jump: Option<&BookJumpTarget>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !path.is_file() {
            self.show_toast(format!("Book not found: {}", path.display()), true, cx);
            return;
        }
        let is_epub = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("epub"));
        if !is_epub {
            self.show_toast(
                "Bookworm currently reads EPUB books directly — convert other formats to EPUB first."
                    .to_string(),
                true,
                cx,
            );
            return;
        }

        let canon = fs::canonicalize(&path).unwrap_or(path);
        if let Some(existing_idx) = self.tabs.iter().position(|t| t.path == canon) {
            self.active_tab = Some(existing_idx);
            if let Some(view) = self.tabs[existing_idx].view.clone() {
                if let Some(target) = jump {
                    view.update(cx, |v, cx| match target {
                        BookJumpTarget::Highlight(hl) => v.jump_to_highlight(hl, cx),
                        BookJumpTarget::Bookmark(bm) => v.jump_to_bookmark(bm, cx),
                    });
                }
                let fh = view.read(cx).focus_handle(cx);
                fh.focus(window, cx);
            }
            cx.notify();
            return;
        }

        let file_size_bytes = fs::metadata(&canon).map(|m| m.len()).unwrap_or(0);
        let book = match EpubBook::load(&canon) {
            Ok(book) => book,
            Err(err) => {
                self.show_toast(format!("Failed to open EPUB: {err}"), true, cx);
                return;
            }
        };

        let mut store = LibraryStore::load(&self.workspace_root);
        let rel_key = normalize_rel_key(&self.workspace_root, &canon);
        let title = if book.title.trim().is_empty() {
            canon
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Book")
                .to_string()
        } else {
            book.title.clone()
        };

        let cum_weights = book.chapter_cumulative_weights();
        let now = chrono::Utc::now().to_rfc3339();
        let user_state = {
            let entry = store.books.entry(rel_key.clone()).or_default();
            entry.total_chapters = book.chapters.len().max(1);
            let ch = entry.chapter_idx.min(entry.total_chapters.saturating_sub(1));
            let loc = EpubBook::global_location_from_weights(&cum_weights, ch, entry.scroll_pct);
            entry.total_pages = loc.total_pages;
            if entry.status == ReadingStatus::Finished {
                entry.overall_pct = 1.0;
                entry.current_page = loc.total_pages;
            } else {
                entry.overall_pct = loc.progress_frac;
                entry.current_page = loc.page;
            }
            if entry.status == ReadingStatus::Unread {
                entry.status = ReadingStatus::Reading;
            }
            entry.last_opened = Some(now.clone());
            entry.updated_at = Some(now);
            entry.clone()
        };
        let _ = store.sync_save(&self.workspace_root);
        self.sync_library_view_store(store.clone(), cx);

        let prefs = store.reader_prefs.clone();
        let path_clone = canon.clone();
        let rel_clone = rel_key.clone();
        let view = cx.new(|cx| {
            EpubView::new(
                path_clone,
                rel_clone,
                book,
                user_state,
                prefs,
                window,
                cx,
            )
        });

        let sub = cx.subscribe_in(&view, window, |app, view, event, window, cx| match event {
            EpubViewEvent::ProgressChanged {
                rel_path,
                chapter_idx,
                total_chapters,
                scroll_pct,
                overall_pct,
                current_page,
                total_pages,
                mark_finished,
            } => {
                let mut store = LibraryStore::load(&app.workspace_root);
                store.update_progress_global(
                    rel_path,
                    *chapter_idx,
                    *total_chapters,
                    *scroll_pct,
                    *overall_pct,
                    *current_page,
                    *total_pages,
                    *mark_finished,
                );
                let _ = store.sync_save(&app.workspace_root);
                app.sync_library_view_store(store, cx);
                if *mark_finished {
                    app.show_toast("Marked book as Finished".to_string(), false, cx);
                }
                cx.notify();
            }
            EpubViewEvent::PrefsChanged(prefs) => {
                let mut store = LibraryStore::load(&app.workspace_root);
                store.reader_prefs = prefs.clone();
                let _ = store.sync_save(&app.workspace_root);
                for tab in &app.tabs {
                    if let Some(other) = &tab.view
                        && other != view
                    {
                        other.update(cx, |v, cx| v.apply_prefs(prefs.clone(), cx));
                    }
                }
                cx.notify();
            }
            EpubViewEvent::BookmarksChanged {
                rel_path,
                bookmarks,
                deleted_id,
            } => {
                let mut store = LibraryStore::load(&app.workspace_root);
                let merged =
                    store.apply_bookmarks_update(rel_path, bookmarks, deleted_id.as_deref());
                let _ = store.sync_save(&app.workspace_root);
                if merged != *bookmarks {
                    view.update(cx, |v, _| v.bookmarks = merged);
                }
                app.sync_library_view_store(store, cx);
                cx.notify();
            }
            EpubViewEvent::HighlightsChanged {
                rel_path,
                highlights,
                deleted_id,
            } => {
                let mut store = LibraryStore::load(&app.workspace_root);
                let merged =
                    store.apply_highlights_update(rel_path, highlights, deleted_id.as_deref());
                let _ = store.sync_save(&app.workspace_root);
                if merged != *highlights {
                    view.update(cx, |v, _| v.highlights = merged);
                }
                app.sync_library_view_store(store, cx);
                cx.notify();
            }
            EpubViewEvent::OpenInLibrary { path } => {
                app.active_tab = None;
                let path = path.clone();
                app.library_view
                    .update(cx, |lv, cx| lv.select_book_by_path(&path, window, cx));
                cx.notify();
            }
        });

        if let Some(target) = jump {
            view.update(cx, |v, cx| match target {
                BookJumpTarget::Highlight(hl) => v.jump_to_highlight(hl, cx),
                BookJumpTarget::Bookmark(bm) => v.jump_to_bookmark(bm, cx),
            });
        }
        let fh = view.read(cx).focus_handle(cx);
        fh.focus(window, cx);

        self.tabs.push(BookTab {
            id: next_tab_id(),
            path: canon,
            rel_path: rel_key,
            title,
            file_size_bytes,
            view: Some(view),
            _sub: sub,
        });
        self.active_tab = Some(self.tabs.len() - 1);
        cx.notify();
    }

    fn reload_open_book(&mut self, target_path: &Path, cx: &mut Context<Self>) {
        let canon = fs::canonicalize(target_path).unwrap_or_else(|_| target_path.to_path_buf());
        for tab in &mut self.tabs {
            if tab.path == canon
                && let Some(view) = &tab.view
                && let Ok(book) = EpubBook::load(&canon)
            {
                if !book.title.trim().is_empty() {
                    tab.title = book.title.clone();
                }
                view.update(cx, |v, cx| v.reload_book(book, cx));
            }
        }
        cx.notify();
    }

    fn relocate_open_book(&mut self, old_path: &Path, new_path: &Path, cx: &mut Context<Self>) {
        let old_canon = fs::canonicalize(old_path).unwrap_or_else(|_| old_path.to_path_buf());
        let new_canon = fs::canonicalize(new_path).unwrap_or_else(|_| new_path.to_path_buf());
        for tab in &mut self.tabs {
            if tab.path == old_path || tab.path == old_canon {
                tab.path = new_canon.clone();
                tab.rel_path = normalize_rel_key(&self.workspace_root, &new_canon);
                if let Some(name) = new_canon.file_stem().and_then(|s| s.to_str())
                    && tab.title.is_empty()
                {
                    tab.title = name.to_string();
                }
                if let Some(view) = &tab.view {
                    let p = tab.path.clone();
                    let r = tab.rel_path.clone();
                    view.update(cx, |v, _| v.relocate_book(p, r));
                }
            }
        }
        cx.notify();
    }

    fn close_tab(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        self.active_tab = match self.active_tab {
            None => None,
            Some(_cur) if self.tabs.is_empty() => None,
            Some(cur) if cur == idx => {
                if idx < self.tabs.len() {
                    Some(idx)
                } else {
                    Some(self.tabs.len() - 1)
                }
            }
            Some(cur) if cur > idx => Some(cur - 1),
            Some(cur) => Some(cur),
        };
        self.focus_active_surface(window, cx);
        cx.notify();
    }

    fn reorder_tab(
        &mut self,
        from_idx: usize,
        to_idx: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if from_idx >= self.tabs.len() || to_idx >= self.tabs.len() || from_idx == to_idx {
            return;
        }
        let active_id = self.active_tab.and_then(|i| self.tabs.get(i).map(|t| t.id));
        let moved = self.tabs.remove(from_idx);
        self.tabs.insert(to_idx, moved);
        if let Some(id) = active_id {
            self.active_tab = self.tabs.iter().position(|t| t.id == id);
        }
        self.focus_active_surface(window, cx);
        cx.notify();
    }

    fn focus_active_surface(&self, window: &mut Window, cx: &mut App) {
        if let Some(idx) = self.active_tab
            && let Some(tab) = self.tabs.get(idx)
            && let Some(view) = &tab.view
        {
            let fh = view.read(cx).focus_handle(cx);
            fh.focus(window, cx);
        } else {
            let fh = self.library_view.read(cx).focus_handle(cx);
            fh.focus(window, cx);
        }
    }

    fn prompt_open_library_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select Book Library Folder".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(dir) = paths.into_iter().next() else {
                return;
            };
            if !dir.is_dir() {
                return;
            }
            let _ = this.update_in(cx, |app, window, cx| {
                app.switch_library_root(dir, window, cx);
            });
        })
        .detach();
    }

    fn switch_library_root(
        &mut self,
        dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace_root = dir.clone();
        self.config.library_root = Some(dir.clone());
        let _ = self.config.save();
        self.tabs.clear();
        self.active_tab = None;
        self.library_view
            .update(cx, |lv, cx| lv.set_workspace_root(dir.clone(), window, cx));
        self.show_toast(
            format!("Library folder set to {}", dir.display()),
            false,
            cx,
        );
    }

    fn export_notes_to_markdown(&mut self, cx: &mut Context<Self>) {
        let books = self.library_view.read(cx).books.clone();
        let out_dir = self.config.effective_export_dir();
        match export_library_notes_to_dir(&books, &out_dir) {
            Ok(files_written) => {
                if files_written == 0 {
                    self.show_toast(
                        "No highlights or bookmarks found to export yet.".to_string(),
                        false,
                        cx,
                    );
                } else {
                    let total_highlights: usize =
                        books.iter().map(|b| b.user.highlights.len()).sum();
                    let total_bookmarks: usize =
                        books.iter().map(|b| b.user.bookmarks.len()).sum();
                    self.show_toast(
                        format!(
                            "Exported {files_written} book file(s) ({total_highlights} highlights, {total_bookmarks} bookmarks) to {}",
                            out_dir.display()
                        ),
                        false,
                        cx,
                    );
                }
            }
            Err(err) => {
                self.show_toast(format!("Failed to export notes: {err}"), true, cx);
            }
        }
    }

    fn handle_find_in_tab(&mut self, _: &FindInTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.active_tab
            && let Some(view) = self.tabs.get(idx).and_then(|t| t.view.clone())
        {
            view.update(cx, |v, cx| v.open_find(window, cx));
        } else {
            self.library_view
                .update(cx, |lv, cx| lv.focus_search(window, cx));
        }
    }

    fn handle_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.active_tab {
            self.close_tab(idx, window, cx);
        }
    }

    fn handle_next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = match self.active_tab {
            None => Some(0),
            Some(i) if i + 1 < self.tabs.len() => Some(i + 1),
            Some(_) => None,
        };
        self.focus_active_surface(window, cx);
        cx.notify();
    }

    fn handle_prev_tab(&mut self, _: &PrevTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_tab = match self.active_tab {
            None => Some(self.tabs.len() - 1),
            Some(0) => None,
            Some(i) => Some(i - 1),
        };
        self.focus_active_surface(window, cx);
        cx.notify();
    }

    fn handle_toggle_outline(
        &mut self,
        _: &ToggleOutline,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.config.outline_open = !self.config.outline_open;
        let _ = self.config.save();
        cx.notify();
    }

    fn handle_open_folder_dialog(
        &mut self,
        _: &OpenFolderDialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.prompt_open_library_folder(window, cx);
    }

    fn handle_escape(&mut self, _: &EscapeModal, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.active_tab
            && let Some(view) = self.tabs.get(idx).and_then(|t| t.view.clone())
            && view.read(cx).is_find_open()
        {
            view.update(cx, |v, cx| v.close_find(Some(window), cx));
        }
    }

    fn render_outline_panel(&self, view: &Entity<EpubView>, cx: &mut Context<Self>) -> AnyElement {
        let ev = view.read(cx);
        let entries = ev.outline_headings();
        let active_ch = ev.chapter_idx;
        let total_ch = ev.book.chapters.len();
        let v_jump = view.clone();

        let sidebar_bg = hex_color(kd::BG_SIDEBAR);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER_SUBTLE);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let green = hex_color(kd::GREEN);

        v_flex()
            .w(px(252.))
            .flex_shrink_0()
            .h_full()
            .bg(sidebar_bg)
            .border_l_1()
            .border_color(border)
            .child(
                h_flex()
                    .h(px(36.))
                    .px_3()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(fg)
                            .child("Chapters & Outline"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(muted)
                            .child(format!("{}/{}", active_ch + 1, total_ch.max(1))),
                    ),
            )
            .child(
                div()
                    .id("bw-outline-scroll")
                    .track_scroll(&self.outline_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_2()
                    .child(if entries.is_empty() {
                        div()
                            .p_2()
                            .text_xs()
                            .text_color(muted)
                            .child("No chapters in outline.")
                            .into_any_element()
                    } else {
                        v_flex()
                            .gap_0p5()
                            .children(entries.into_iter().enumerate().map(|(ix, entry)| {
                                let is_active = entry.chapter_idx == active_ch;
                                let indent = (entry.level.saturating_sub(1) as f32 * 10.0) + 8.0;
                                let v_click = v_jump.clone();
                                let ch = entry.chapter_idx;
                                div()
                                    .id(("bw-outline-item", ix))
                                    .pl(px(indent))
                                    .pr_2()
                                    .py_1()
                                    .rounded(px(5.))
                                    .cursor_pointer()
                                    .bg(if is_active {
                                        accent_alpha(0x2e)
                                    } else {
                                        gpui_kit::transparent_black()
                                    })
                                    .hover(move |s| s.bg(panel_bg))
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        v_click.update(cx, |v, cx| v.go_to_chapter(ch, cx));
                                    }))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(if is_active {
                                                FontWeight::SEMIBOLD
                                            } else {
                                                FontWeight::NORMAL
                                            })
                                            .text_color(if is_active { green } else { fg })
                                            .truncate()
                                            .child(entry.title),
                                    )
                            }))
                            .into_any_element()
                    }),
            )
            .into_any_element()
    }
}

impl Render for BookwormApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let deep_bg = hex_color(kd::BG_DEEP);
        let main_bg = hex_color(kd::BG_MAIN);
        let sidebar_bg = hex_color(kd::BG_SIDEBAR);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER_SUBTLE);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let green = hex_color(kd::GREEN);

        let book_summary = self.library_view.read(cx).summary();
        let lib_selected = self.active_tab.is_none();
        let root_display = self
            .workspace_root
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| self.workspace_root.display().to_string());

        let has_active_book = self
            .active_tab
            .and_then(|i| self.tabs.get(i))
            .is_some();

        let book_tab_items = self.tabs.iter().enumerate().map(|(ix, tab)| {
            let is_selected = self.active_tab == Some(ix);
            let tab_id = tab.id;
            let title = tab.title.clone();
            let drag_title = tab.title.clone();

            h_flex()
                .id(SharedString::from(format!("bw-tab-{tab_id}")))
                .h(px(30.))
                .px_2p5()
                .gap_1p5()
                .items_center()
                .rounded_t(px(6.))
                .cursor_pointer()
                .flex_shrink_0()
                .border_t_2()
                .border_color(if is_selected {
                    green
                } else {
                    gpui_kit::transparent_black()
                })
                .bg(if is_selected { main_bg } else { deep_bg })
                .hover(move |s| if is_selected { s } else { s.bg(sidebar_bg) })
                .on_drag(
                    DraggedBookTab {
                        from_idx: ix,
                        title: drag_title,
                    },
                    |drag, _, _, cx| {
                        cx.new(|_| DraggedBookTabPreview {
                            title: drag.title.clone(),
                        })
                    },
                )
                .drag_over::<DraggedBookTab>(move |style, drag, _, _| {
                    if drag.from_idx == ix {
                        style
                    } else if drag.from_idx > ix {
                        style.border_l_2().border_color(green).bg(panel_bg)
                    } else {
                        style.border_r_2().border_color(green).bg(panel_bg)
                    }
                })
                .on_drop(cx.listener(move |this, drag: &DraggedBookTab, window, cx| {
                    this.reorder_tab(drag.from_idx, ix, window, cx);
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.active_tab = Some(ix);
                    this.focus_active_surface(window, cx);
                    cx.notify();
                }))
                .on_mouse_down(
                    MouseButton::Middle,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.close_tab(ix, window, cx);
                    }),
                )
                .child(
                    div()
                        .max_w(px(176.))
                        .text_xs()
                        .font_weight(if is_selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if is_selected { fg } else { muted })
                        .truncate()
                        .child(title),
                )
                .child(
                    Button::new(SharedString::from(format!("bw-tab-close-{tab_id}")))
                        .text()
                        .xsmall()
                        .icon(IconName::Close)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.close_tab(ix, window, cx);
                        })),
                )
        });

        let top_bar = h_flex()
            .h(px(38.))
            .w_full()
            .px_2p5()
            .gap_2()
            .items_center()
            .justify_between()
            .bg(deep_bg)
            .border_b_1()
            .border_color(border)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1p5()
                    .items_end()
                    .child(
                        h_flex()
                            .id("bw-pinned-library-tab")
                            .h(px(30.))
                            .px_3()
                            .gap_1p5()
                            .items_center()
                            .rounded_t(px(6.))
                            .cursor_pointer()
                            .flex_shrink_0()
                            .border_t_2()
                            .border_color(if lib_selected {
                                green
                            } else {
                                gpui_kit::transparent_black()
                            })
                            .bg(if lib_selected { main_bg } else { deep_bg })
                            .hover(move |s| if lib_selected { s } else { s.bg(sidebar_bg) })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.active_tab = None;
                                this.focus_active_surface(window, cx);
                                cx.notify();
                            }))
                            .child(Icon::new(IconName::Library).size(px(14.)).text_color(green))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(if lib_selected { fg } else { muted })
                                    .child(book_summary),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("bw-tabs-scroll")
                            .flex_1()
                            .min_w_0()
                            .overflow_x_scrollbar()
                            .gap_1()
                            .items_end()
                            .children(book_tab_items),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .flex_shrink_0()
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded(px(4.))
                            .bg(sidebar_bg)
                            .border_1()
                            .border_color(border)
                            .text_size(px(11.))
                            .text_color(muted)
                            .child(root_display),
                    )
                    .child(
                        Button::new("bw-open-folder")
                            .ghost()
                            .xsmall()
                            .icon(IconName::FolderOpen)
                            .label("Folder")
                            .tooltip("Change Book Library Folder (Ctrl+Shift+O)")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.prompt_open_library_folder(window, cx);
                            })),
                    )
                    .child(
                        Button::new("bw-export-notes")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Save)
                            .label("Export Notes")
                            .tooltip("Export highlights & bookmarks to Markdown files")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.export_notes_to_markdown(cx);
                            })),
                    )
                    .when(has_active_book, |row| {
                        row.child(
                            Button::new("bw-outline-toggle")
                                .with_variant(if self.config.outline_open {
                                    ButtonVariant::Secondary
                                } else {
                                    ButtonVariant::Ghost
                                })
                                .xsmall()
                                .icon(IconName::PanelRight)
                                .label("Outline")
                                .tooltip("Toggle Chapter Outline (Ctrl+Shift+B)")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.config.outline_open = !this.config.outline_open;
                                    let _ = this.config.save();
                                    cx.notify();
                                })),
                        )
                    }),
            );

        let body: AnyElement = if let Some(active_idx) = self.active_tab
            && let Some(tab) = self.tabs.get(active_idx)
        {
            let reader = render_epub_tab(self, tab, cx).into_any_element();
            let outline = if self.config.outline_open
                && let Some(view) = &tab.view
            {
                Some(self.render_outline_panel(view, cx))
            } else {
                None
            };
            h_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(div().flex_1().min_w_0().h_full().child(reader))
                .when_some(outline, |row, panel| row.child(panel))
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(render_library_tab(self, cx))
                .into_any_element()
        };

        let toast_el = self.toast.as_ref().map(|(msg, is_err)| {
            let accent_c = if *is_err {
                hex_color(kd::RED)
            } else {
                green
            };
            h_flex()
                .h(px(28.))
                .w_full()
                .px_3()
                .gap_2()
                .items_center()
                .justify_between()
                .bg(sidebar_bg)
                .border_t_1()
                .border_color(border)
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(accent_c)
                        .truncate()
                        .child(msg.clone()),
                )
                .child(
                    Button::new("bw-toast-dismiss")
                        .text()
                        .xsmall()
                        .icon(IconName::Close)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toast = None;
                            cx.notify();
                        })),
                )
        });

        v_flex()
            .id("bookworm-app-root")
            .size_full()
            .bg(main_bg)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_find_in_tab))
            .on_action(cx.listener(Self::handle_close_tab))
            .on_action(cx.listener(Self::handle_next_tab))
            .on_action(cx.listener(Self::handle_prev_tab))
            .on_action(cx.listener(Self::handle_toggle_outline))
            .on_action(cx.listener(Self::handle_open_folder_dialog))
            .on_action(cx.listener(Self::handle_escape))
            .child(top_bar)
            .child(body)
            .when_some(toast_el, |col, t| col.child(t))
    }
}
