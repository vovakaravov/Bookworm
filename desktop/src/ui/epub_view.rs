use super::find_bar::{FIND_INPUT_W, find_count_badge, floating_find_card};
use super::image_util::decode_bytes_image;
use super::{BookTab, BookwormApp};
use bookworm_core::epub::{EpubBlock, EpubBook, EpubRun, OutlineEntry};
use bookworm_core::find_all_byte_ranges;
use bookworm_core::library::{
    BookBookmark, BookHighlight, BookUserState, HighlightColor, HighlightDecoration,
    ReaderFontPref, ReaderPaginationPref, ReaderPrefs, ReaderThemePref, ReaderWidthPref,
    ReadingStatus, clamp_to_char_boundary, compute_chapter_page_offsets,
    page_index_for_content_y, word_bounds_at,
};
use crate::theme::{hex_alpha, hex_color, kd};
use std::collections::HashMap;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::input::{Editor, EditorState, Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, ClipboardItem, Context, CursorStyle, Entity,
    EventEmitter, FocusHandle, Focusable, FontStyle, FontWeight, HighlightStyle, Hsla,
    InteractiveElement as _, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ObjectFit, ParentElement, Pixels, Point, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, StrikethroughStyle, Styled, StyledImage as _,
    StyledText, Subscription, TextLayout, UnderlineStyle, Window, canvas, div, img, point,
    prelude::FluentBuilder as _, px, relative,
};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum EpubViewEvent {
    ProgressChanged {
        rel_path: String,
        chapter_idx: usize,
        total_chapters: usize,
        scroll_pct: f32,
        mark_finished: bool,
    },
    PrefsChanged(ReaderPrefs),
    BookmarksChanged {
        rel_path: String,
        bookmarks: Vec<BookBookmark>,
        deleted_id: Option<String>,
    },
    HighlightsChanged {
        rel_path: String,
        highlights: Vec<BookHighlight>,
        deleted_id: Option<String>,
    },
    OpenInLibrary {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TextPos {
    block_idx: usize,
    byte_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationsTab {
    Highlights,
    Bookmarks,
}

#[derive(Debug, Clone)]
enum ActiveHighlightPopup {
    Selection {
        anchor_pos: Point<Pixels>,
        note_mode: bool,
        selected_color: HighlightColor,
        selected_style: HighlightDecoration,
    },
    Inspect {
        highlight_id: String,
        anchor_pos: Point<Pixels>,
    },
}

pub struct EpubView {
    pub path: PathBuf,
    pub rel_path: String,
    pub book: EpubBook,
    pub chapter_idx: usize,
    pub prefs: ReaderPrefs,
    pub is_finished: bool,
    pub bookmarks: Vec<BookBookmark>,
    pub highlights: Vec<BookHighlight>,
    pub bookmarks_open: bool,
    annotations_tab: AnnotationsTab,
    annotations_color_filter: Option<HighlightColor>,
    annotations_notes_only: bool,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    annotations_hl_scroll: ScrollHandle,
    annotations_bm_scroll: ScrollHandle,
    viewport_bounds: Rc<Cell<Bounds<Pixels>>>,
    rendered_layouts: Rc<RefCell<Vec<(usize, usize, TextLayout)>>>,
    selection_anchor: Option<TextPos>,
    selection_head: Option<TextPos>,
    is_dragging_selection: bool,
    drag_start_window_pos: Option<Point<Pixels>>,
    pending_click_highlight_id: Option<String>,
    popup: Option<ActiveHighlightPopup>,
    note_editor: Entity<EditorState>,
    note_needs_focus: bool,
    find_open: bool,
    find_case_sensitive: bool,
    find_input: Entity<InputState>,
    bookmark_note_input: Entity<InputState>,
    _find_sub: Subscription,
    _bm_sub: Subscription,
    find_query: String,
    find_matches: Vec<(usize, usize, Range<usize>)>,
    find_active: usize,
    find_needs_focus: bool,
    pending_scroll_block: Option<(usize, usize)>,
    pending_scroll_fraction: Option<f32>,
    scroll_jump_gen: u64,
    last_reported_scroll_pct: f32,
    scroll_save_scheduled: bool,
    page_offsets: Rc<RefCell<Vec<(f32, f32)>>>,
    current_page: usize,
    pending_last_page: bool,
    needs_page_recompute: bool,
    last_wheel_page_at: Option<std::time::Instant>,
    decoded_images: RefCell<HashMap<(usize, usize), Arc<gpui_kit::Image>>>,
}

impl EventEmitter<EpubViewEvent> for EpubView {}

impl Focusable for EpubView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn block_plain_text(blk: &EpubBlock) -> Option<&str> {
    match blk {
        EpubBlock::Heading { text, .. }
        | EpubBlock::Paragraph { text, .. }
        | EpubBlock::ListItem { text, .. } => Some(text.as_str()),
        EpubBlock::Image { .. } | EpubBlock::Rule => None,
    }
}

fn highlight_fill_alpha(color: HighlightColor, light_paper: bool) -> Hsla {
    let alpha = if light_paper { 0x5c } else { 0x48 };
    hex_alpha((color.rgb_hex() << 8) | alpha)
}

#[allow(clippy::too_many_arguments)]
fn flatten_block_highlights(
    text: &str,
    runs: &[EpubRun],
    block_hls: &[(&BookHighlight, Range<usize>)],
    find_matches: &[Range<usize>],
    active_find_rel: Option<usize>,
    sel_range: Option<Range<usize>>,
    code_bg: Hsla,
    code_fg: Hsla,
    sel_bg: Hsla,
    light_paper: bool,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut pts = Vec::with_capacity(
        (runs.len() + block_hls.len() + find_matches.len() + 1) * 2,
    );
    let mut push_range = |r: &Range<usize>| {
        let a = clamp_to_char_boundary(text, r.start);
        let b = clamp_to_char_boundary(text, r.end);
        if a < b {
            pts.push(a);
            pts.push(b);
        }
    };

    for run in runs {
        push_range(&run.range);
    }
    for (_, r) in block_hls {
        push_range(r);
    }
    for r in find_matches {
        push_range(r);
    }
    if let Some(sr) = &sel_range {
        push_range(sr);
    }

    if pts.len() < 2 {
        return Vec::new();
    }
    pts.sort_unstable();
    pts.dedup();

    let mut out: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }

        let base_run = runs.iter().find(|r| {
            let rs = clamp_to_char_boundary(text, r.range.start);
            let re = clamp_to_char_boundary(text, r.range.end);
            rs <= a && re >= b
        });

        let matching_hls: Vec<&BookHighlight> = block_hls
            .iter()
            .filter_map(|(hl, r)| {
                let rs = clamp_to_char_boundary(text, r.start);
                let re = clamp_to_char_boundary(text, r.end);
                (rs <= a && re >= b).then_some(*hl)
            })
            .collect();

        let match_ix = find_matches.iter().position(|r| {
            let rs = clamp_to_char_boundary(text, r.start);
            let re = clamp_to_char_boundary(text, r.end);
            rs <= a && re >= b
        });

        let in_sel = sel_range.as_ref().is_some_and(|sr| {
            let rs = clamp_to_char_boundary(text, sr.start);
            let re = clamp_to_char_boundary(text, sr.end);
            rs <= a && re >= b
        });

        if base_run.is_none() && matching_hls.is_empty() && match_ix.is_none() && !in_sel {
            continue;
        }

        let mut style = HighlightStyle::default();
        if let Some(run) = base_run {
            if run.code {
                style.color = Some(code_fg);
                style.background_color = Some(code_bg);
            }
            if run.bold {
                style.font_weight = Some(FontWeight::BOLD);
            }
            if run.italic {
                style.font_style = Some(FontStyle::Italic);
            }
            if run.underline {
                style.underline = Some(UnderlineStyle {
                    thickness: px(1.),
                    color: None,
                    wavy: false,
                });
            }
        }

        for hl in matching_hls {
            let solid = hex_color(hl.color.rgb_hex());
            match hl.style {
                HighlightDecoration::Highlight => {
                    style.background_color = Some(highlight_fill_alpha(hl.color, light_paper));
                    if !hl.note.trim().is_empty() {
                        style.underline = Some(UnderlineStyle {
                            thickness: px(1.5),
                            color: Some(solid),
                            wavy: false,
                        });
                    }
                }
                HighlightDecoration::Underline => {
                    style.underline = Some(UnderlineStyle {
                        thickness: px(2.0),
                        color: Some(solid),
                        wavy: false,
                    });
                }
                HighlightDecoration::Squiggly => {
                    style.underline = Some(UnderlineStyle {
                        thickness: px(2.0),
                        color: Some(solid),
                        wavy: true,
                    });
                }
                HighlightDecoration::Strikethrough => {
                    style.strikethrough = Some(StrikethroughStyle {
                        thickness: px(1.5),
                        color: Some(solid),
                    });
                }
            }
        }

        if let Some(ix) = match_ix {
            style.background_color = Some(if active_find_rel == Some(ix) {
                hex_alpha(0xe8a84999)
            } else {
                hex_alpha(0xc4b28a52)
            });
        }

        if in_sel {
            style.background_color = Some(sel_bg);
        }

        if let Some((last_r, last_s)) = out.last_mut()
            && last_r.end == a
            && *last_s == style
        {
            last_r.end = b;
        } else {
            out.push((a..b, style));
        }
    }

    out
}

impl EpubView {
    pub fn new(
        path: PathBuf,
        rel_path: String,
        book: EpubBook,
        user_state: BookUserState,
        prefs: ReaderPrefs,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let find_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in book…"));
        let _find_sub = cx.subscribe(&find_input, |this, state, event, cx| {
            match event {
                InputEvent::Change => {
                    this.find_query = state.read(cx).value().to_string();
                    this.recompute_find(true, cx);
                    cx.notify();
                }
                InputEvent::PressEnter {
                    secondary, shift, ..
                } => {
                    this.step_find(!(*shift || *secondary), cx);
                    cx.notify();
                }
                _ => {}
            }
        });

        let bookmark_note_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Add bookmark note (Enter)…"));
        let _bm_sub = cx.subscribe_in(&bookmark_note_input, window, |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.add_bookmark(window, cx);
            }
        });

        let note_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .searchable(false)
                .soft_wrap(true)
                .placeholder("Write a note about this passage…")
        });

        let ch_count = book.chapters.len();
        let initial_ch = user_state.chapter_idx.min(ch_count.saturating_sub(1));
        let restored_scroll = user_state.scroll_pct.clamp(0.0, 1.0);
        let pending_scroll_fraction = if restored_scroll > 0.01 {
            Some(restored_scroll)
        } else {
            None
        };

        let mut view = Self {
            path,
            rel_path,
            book,
            chapter_idx: initial_ch,
            prefs,
            is_finished: user_state.status == ReadingStatus::Finished,
            bookmarks: user_state.bookmarks,
            highlights: user_state.highlights,
            bookmarks_open: false,
            annotations_tab: AnnotationsTab::Highlights,
            annotations_color_filter: None,
            annotations_notes_only: false,
            focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
            annotations_hl_scroll: ScrollHandle::new(),
            annotations_bm_scroll: ScrollHandle::new(),
            viewport_bounds: Rc::new(Cell::new(Bounds::default())),
            rendered_layouts: Rc::new(RefCell::new(Vec::new())),
            selection_anchor: None,
            selection_head: None,
            is_dragging_selection: false,
            drag_start_window_pos: None,
            pending_click_highlight_id: None,
            popup: None,
            note_editor,
            note_needs_focus: false,
            find_open: false,
            find_case_sensitive: false,
            find_input,
            bookmark_note_input,
            _find_sub,
            _bm_sub,
            find_query: String::new(),
            find_matches: Vec::new(),
            find_active: 0,
            find_needs_focus: false,
            pending_scroll_block: None,
            pending_scroll_fraction,
            scroll_jump_gen: 0,
            last_reported_scroll_pct: restored_scroll,
            scroll_save_scheduled: false,
            page_offsets: Rc::new(RefCell::new(vec![(0.0, 600.0)])),
            current_page: 0,
            pending_last_page: false,
            needs_page_recompute: true,
            last_wheel_page_at: None,
            decoded_images: RefCell::new(HashMap::new()),
        };
        if view.pending_scroll_fraction.is_some() {
            view.schedule_pending_scroll(cx);
        }
        view
    }

    pub fn reload_book(&mut self, book: EpubBook, cx: &mut Context<Self>) {
        self.book = book;
        self.decoded_images.borrow_mut().clear();
        self.rendered_layouts.borrow_mut().clear();
        self.clear_selection_and_popup();
        self.needs_page_recompute = true;
        let max_ch = self.book.chapters.len().saturating_sub(1);
        if self.chapter_idx > max_ch {
            self.chapter_idx = max_ch;
        }
        if !self.find_query.trim().is_empty() {
            self.recompute_find(false, cx);
        }
        cx.notify();
    }

    pub fn relocate_book(&mut self, new_path: PathBuf, new_rel_path: String) {
        self.path = new_path;
        self.rel_path = new_rel_path;
    }

    pub fn apply_prefs(&mut self, prefs: ReaderPrefs, cx: &mut Context<Self>) {
        if self.prefs != prefs {
            self.prefs = prefs;
            cx.notify();
        }
    }

    pub fn current_scroll_pct(&self) -> f32 {
        let max_y = f32::from(self.scroll_handle.max_offset().y);
        if max_y <= 1.0 {
            return 0.0;
        }
        let off_y = if self.prefs.pagination == ReaderPaginationPref::Page {
            self.page_offsets
                .borrow()
                .get(self.current_page)
                .map(|p| p.0)
                .unwrap_or_else(|| -f32::from(self.scroll_handle.offset().y))
        } else {
            -f32::from(self.scroll_handle.offset().y)
        };
        (off_y / max_y).clamp(0.0, 1.0)
    }

    fn collect_line_bands(&self, vp_top_y: f32, cur_off_y: f32) -> Vec<(f32, f32)> {
        let layouts = self.rendered_layouts.borrow();
        let mut bands = Vec::with_capacity(layouts.len() * 3);
        for (_, _, layout) in layouts.iter() {
            let b = layout.bounds();
            let lh = f32::from(layout.line_height()).max(14.0);
            let top = (f32::from(b.top()) - vp_top_y - cur_off_y).max(0.0);
            let bot = (f32::from(b.bottom()) - vp_top_y - cur_off_y).max(top);
            let mut y = top;
            while y + 0.5 < bot {
                let next = (y + lh).min(bot);
                bands.push((y, next));
                y = next;
            }
        }
        bands
    }

    /// Recomputes discrete line-snapped page slices from the previously painted frame and
    /// returns `true` if `current_page` / `page_offsets` changed and needs a follow-up paint.
    fn sync_page_layout(&mut self) -> bool {
        if self.prefs.pagination != ReaderPaginationPref::Page {
            return false;
        }
        let scroll_bounds = self.scroll_handle.bounds();
        let vp_h = f32::from(scroll_bounds.size.height);
        if vp_h <= 20.0 {
            return false;
        }
        let has_text = self
            .book
            .chapters
            .get(self.chapter_idx)
            .is_some_and(|c| c.blocks.iter().any(|b| block_plain_text(b).is_some()));
        if has_text && self.rendered_layouts.borrow().is_empty() {
            return false;
        }

        let vp_top_y = f32::from(scroll_bounds.top());
        let cur_off_y = f32::from(self.scroll_handle.offset().y);
        let max_y = f32::from(self.scroll_handle.max_offset().y).max(0.0);
        let total_content_h = vp_h + max_y;
        let bands = self.collect_line_bands(vp_top_y, cur_off_y);
        let new_pages = compute_chapter_page_offsets(vp_h, total_content_h, &bands);

        let mut changed = *self.page_offsets.borrow() != new_pages;
        *self.page_offsets.borrow_mut() = new_pages;

        let page_count = self.page_offsets.borrow().len().max(1);
        if self.pending_last_page {
            self.pending_last_page = false;
            self.current_page = page_count - 1;
            changed = true;
        } else if self.current_page >= page_count {
            self.current_page = page_count - 1;
            changed = true;
        }

        if let Some((start_y, _)) = self.page_offsets.borrow().get(self.current_page).copied() {
            let clamped_y = start_y.clamp(0.0, max_y);
            let actual_y = -f32::from(self.scroll_handle.offset().y);
            if (actual_y - clamped_y).abs() > 0.5 {
                self.scroll_handle.set_offset(point(px(0.), px(-clamped_y)));
                changed = true;
            }
        }
        self.needs_page_recompute = false;
        changed
    }

    pub fn go_to_page(&mut self, page_idx: usize, cx: &mut Context<Self>) {
        self.sync_page_layout();
        let count = self.page_offsets.borrow().len().max(1);
        let clamped = page_idx.min(count - 1);
        self.current_page = clamped;
        let max_y = f32::from(self.scroll_handle.max_offset().y).max(0.0);
        if let Some((start_y, _)) = self.page_offsets.borrow().get(clamped).copied() {
            self.scroll_handle
                .set_offset(point(px(0.), px(-start_y.clamp(0.0, max_y))));
        }
        self.emit_progress(false, cx);
        cx.notify();
    }

    pub fn next_page_or_chapter(&mut self, cx: &mut Context<Self>) {
        if self.prefs.pagination == ReaderPaginationPref::Page {
            self.sync_page_layout();
            let count = self.page_offsets.borrow().len();
            if self.current_page + 1 < count {
                self.go_to_page(self.current_page + 1, cx);
                return;
            }
        }
        self.next_chapter(cx);
    }

    pub fn prev_page_or_chapter(&mut self, cx: &mut Context<Self>) {
        if self.prefs.pagination == ReaderPaginationPref::Page {
            self.sync_page_layout();
            if self.current_page > 0 {
                self.go_to_page(self.current_page - 1, cx);
                return;
            }
            if self.chapter_idx > 0 {
                self.pending_last_page = true;
                self.go_to_chapter(self.chapter_idx - 1, cx);
                return;
            }
        }
        self.prev_chapter(cx);
    }

    pub fn overall_pct(&self) -> f32 {
        if self.is_finished {
            return 1.0;
        }
        let total = self.book.chapters.len().max(1) as f32;
        ((self.chapter_idx as f32 + self.current_scroll_pct()) / total).clamp(0.0, 1.0)
    }

    fn emit_progress(&mut self, mark_finished: bool, cx: &mut Context<Self>) {
        if mark_finished {
            self.is_finished = true;
        }
        let scroll_pct = self.current_scroll_pct();
        self.last_reported_scroll_pct = scroll_pct;
        cx.emit(EpubViewEvent::ProgressChanged {
            rel_path: self.rel_path.clone(),
            chapter_idx: self.chapter_idx,
            total_chapters: self.book.chapters.len(),
            scroll_pct,
            mark_finished,
        });
    }

    fn emit_prefs(&self, cx: &mut Context<Self>) {
        cx.emit(EpubViewEvent::PrefsChanged(self.prefs.clone()));
    }

    fn emit_highlights(&self, cx: &mut Context<Self>) {
        self.emit_highlights_with_deleted(None, cx);
    }

    fn emit_highlights_with_deleted(&self, deleted_id: Option<String>, cx: &mut Context<Self>) {
        cx.emit(EpubViewEvent::HighlightsChanged {
            rel_path: self.rel_path.clone(),
            highlights: self.highlights.clone(),
            deleted_id,
        });
    }

    fn clear_selection_and_popup(&mut self) {
        self.selection_anchor = None;
        self.selection_head = None;
        self.is_dragging_selection = false;
        self.drag_start_window_pos = None;
        self.pending_click_highlight_id = None;
        self.popup = None;
        self.note_needs_focus = false;
    }

    fn current_chapter_block_text(&self, blk_ix: usize) -> Option<&str> {
        self.book
            .chapters
            .get(self.chapter_idx)
            .and_then(|ch| ch.blocks.get(blk_ix))
            .and_then(block_plain_text)
    }

    fn hit_test_position(&self, pos: Point<Pixels>) -> Option<TextPos> {
        let layouts = self.rendered_layouts.borrow();
        if layouts.is_empty() {
            return None;
        }

        let mut chosen: Option<&(usize, usize, TextLayout)> = None;
        for entry in layouts.iter() {
            let b = entry.2.bounds();
            if b.top() - px(6.) <= pos.y
                && pos.y <= b.bottom() + px(6.)
                && b.left() - px(16.) <= pos.x
                && pos.x <= b.right() + px(16.)
            {
                chosen = Some(entry);
                break;
            }
        }

        if chosen.is_none() {
            let mut best_score = f32::MAX;
            for entry in layouts.iter() {
                let b = entry.2.bounds();
                let dy = if pos.y < b.top() {
                    f32::from(b.top() - pos.y)
                } else if pos.y > b.bottom() {
                    f32::from(pos.y - b.bottom())
                } else {
                    0.0
                };
                let dx = if pos.x < b.left() {
                    f32::from(b.left() - pos.x)
                } else if pos.x > b.right() {
                    f32::from(pos.x - b.right())
                } else {
                    0.0
                };
                // Weight horizontal offset strongly so 2-column drag stays within its column
                let score = dy + dx * 4.0;
                if score < best_score {
                    best_score = score;
                    chosen = Some(entry);
                }
            }
        }

        let (blk_ix, len, layout) = chosen?;
        let raw_ix = match layout.index_for_position(pos) {
            Ok(ix) | Err(ix) => ix,
        }
        .min(*len);
        let safe_ix = if let Some(txt) = self.current_chapter_block_text(*blk_ix) {
            clamp_to_char_boundary(txt, raw_ix)
        } else {
            0
        };
        Some(TextPos {
            block_idx: *blk_ix,
            byte_offset: safe_ix,
        })
    }

    fn normalized_selection(&self) -> Option<(TextPos, TextPos)> {
        let a = self.selection_anchor?;
        let b = self.selection_head?;
        if a == b {
            return None;
        }
        if a <= b { Some((a, b)) } else { Some((b, a)) }
    }

    fn block_selection_range(&self, blk_ix: usize, block_len: usize) -> Option<Range<usize>> {
        let (start, end) = self.normalized_selection()?;
        if blk_ix < start.block_idx || blk_ix > end.block_idx {
            return None;
        }
        let a = if blk_ix == start.block_idx {
            start.byte_offset.min(block_len)
        } else {
            0
        };
        let b = if blk_ix == end.block_idx {
            end.byte_offset.min(block_len)
        } else {
            block_len
        };
        (a < b).then_some(a..b)
    }

    fn extract_text_for_range(&self, start: TextPos, end: TextPos) -> String {
        let Some(ch) = self.book.chapters.get(self.chapter_idx) else {
            return String::new();
        };
        let mut parts = Vec::new();
        for blk_ix in start.block_idx..=end.block_idx.min(ch.blocks.len().saturating_sub(1)) {
            let Some(txt) = ch.blocks.get(blk_ix).and_then(block_plain_text) else {
                continue;
            };
            let len = txt.len();
            let a = if blk_ix == start.block_idx {
                clamp_to_char_boundary(txt, start.byte_offset)
            } else {
                0
            };
            let b = if blk_ix == end.block_idx {
                clamp_to_char_boundary(txt, end.byte_offset)
            } else {
                len
            };
            if a < b {
                let slice = txt[a..b].trim();
                if !slice.is_empty() {
                    parts.push(slice);
                }
            }
        }
        parts.join("\n\n")
    }

    fn highlight_at_pos(&self, pos: TextPos) -> Option<&BookHighlight> {
        self.highlights.iter().rev().find(|hl| {
            if hl.chapter_idx != self.chapter_idx {
                return false;
            }
            let start = TextPos {
                block_idx: hl.start_block,
                byte_offset: hl.start_offset,
            };
            let end = TextPos {
                block_idx: hl.end_block,
                byte_offset: hl.end_offset,
            };
            pos >= start && pos <= end
        })
    }

    fn anchor_for_pos(&self, pos: TextPos, fallback: Point<Pixels>) -> Point<Pixels> {
        let layouts = self.rendered_layouts.borrow();
        if let Some((_, len, layout)) = layouts.iter().find(|(b, _, _)| *b == pos.block_idx)
            && let Some(pt) = layout.position_for_index(pos.byte_offset.min(*len))
        {
            return pt;
        }
        fallback
    }

    fn open_highlight_inspector(
        &mut self,
        highlight_id: &str,
        anchor_pos: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(note) = self
            .highlights
            .iter()
            .find(|h| h.id == highlight_id)
            .map(|h| h.note.clone())
        else {
            return;
        };
        self.selection_anchor = None;
        self.selection_head = None;
        self.note_editor
            .update(cx, |ed, cx| ed.set_value(note, window, cx));
        self.note_needs_focus = true;
        self.popup = Some(ActiveHighlightPopup::Inspect {
            highlight_id: highlight_id.to_string(),
            anchor_pos,
        });
    }

    fn commit_selection_highlight(
        &mut self,
        color: HighlightColor,
        style: HighlightDecoration,
        note: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((start, end)) = self.normalized_selection() else {
            return;
        };
        let quote = self.extract_text_for_range(start, end);
        if quote.trim().is_empty() {
            self.clear_selection_and_popup();
            cx.notify();
            return;
        }
        let ch_title = self
            .book
            .chapters
            .get(self.chapter_idx)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| format!("Chapter {}", self.chapter_idx + 1));
        let now = chrono::Utc::now();
        let id = format!("hl-{}-{}", now.timestamp_millis(), self.highlights.len() + 1);
        let ts = now.to_rfc3339();
        self.highlights.push(BookHighlight {
            id,
            chapter_idx: self.chapter_idx,
            chapter_title: ch_title,
            start_block: start.block_idx,
            start_offset: start.byte_offset,
            end_block: end.block_idx,
            end_offset: end.byte_offset,
            quote,
            color,
            style,
            note: note.trim().to_string(),
            created_at: ts.clone(),
            updated_at: ts,
        });

        let prefs_changed =
            self.prefs.last_highlight_color != color || self.prefs.last_highlight_style != style;
        self.prefs.last_highlight_color = color;
        self.prefs.last_highlight_style = style;
        self.note_editor
            .update(cx, |ed, cx| ed.set_value("", window, cx));
        self.clear_selection_and_popup();
        self.focus_handle.focus(window, cx);
        if prefs_changed {
            self.emit_prefs(cx);
        }
        self.emit_highlights(cx);
        cx.notify();
    }

    fn update_highlight_appearance(
        &mut self,
        id: &str,
        color: Option<HighlightColor>,
        style: Option<HighlightDecoration>,
        cx: &mut Context<Self>,
    ) {
        let Some(hl) = self.highlights.iter_mut().find(|h| h.id == id) else {
            return;
        };
        if let Some(c) = color {
            hl.color = c;
            self.prefs.last_highlight_color = c;
        }
        if let Some(s) = style {
            hl.style = s;
            self.prefs.last_highlight_style = s;
        }
        hl.updated_at = chrono::Utc::now().to_rfc3339();
        self.emit_prefs(cx);
        self.emit_highlights(cx);
        cx.notify();
    }

    fn save_inspected_highlight_note(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let note_text = self.note_editor.read(cx).value().trim().to_string();
        if let Some(hl) = self.highlights.iter_mut().find(|h| h.id == id) {
            hl.note = note_text;
            hl.updated_at = chrono::Utc::now().to_rfc3339();
        }
        self.clear_selection_and_popup();
        self.focus_handle.focus(window, cx);
        self.emit_highlights(cx);
        cx.notify();
    }

    fn delete_highlight(&mut self, id: &str, window: Option<&mut Window>, cx: &mut Context<Self>) {
        let prev_len = self.highlights.len();
        self.highlights.retain(|h| h.id != id);
        if matches!(&self.popup, Some(ActiveHighlightPopup::Inspect { highlight_id, .. }) if highlight_id == id)
        {
            self.clear_selection_and_popup();
            if let Some(window) = window {
                self.focus_handle.focus(window, cx);
            }
        }
        if self.highlights.len() != prev_len {
            self.emit_highlights_with_deleted(Some(id.to_string()), cx);
            cx.notify();
        }
    }

    pub fn jump_to_highlight(&mut self, hl: &BookHighlight, cx: &mut Context<Self>) {
        if self.book.chapters.is_empty() {
            return;
        }
        let target_ch = hl.chapter_idx.min(self.book.chapters.len() - 1);
        let total_blocks = self
            .book
            .chapters
            .get(target_ch)
            .map(|c| c.blocks.len().max(1))
            .unwrap_or(1);
        let frac = ((hl.start_block as f32 / total_blocks as f32) - 0.06).clamp(0.0, 1.0);
        let switched = self.chapter_idx != target_ch;
        self.scroll_jump_gen = self.scroll_jump_gen.wrapping_add(1);
        if switched {
            self.rendered_layouts.borrow_mut().clear();
            self.clear_selection_and_popup();
            self.chapter_idx = target_ch;
            self.scroll_handle.set_offset(point(px(0.), px(0.)));
            self.last_reported_scroll_pct = frac;
            self.emit_progress(false, cx);
        }
        self.pending_scroll_block = Some((hl.start_block, hl.start_offset));
        self.pending_scroll_fraction = Some(frac);
        if switched || !self.apply_pending_scroll() {
            self.schedule_pending_scroll(cx);
        } else {
            self.emit_progress(false, cx);
        }
        cx.notify();
    }

    pub fn set_font_size(&mut self, next: f32, cx: &mut Context<Self>) {
        self.prefs.font_size = next.clamp(11.0, 28.0);
        self.needs_page_recompute = true;
        self.emit_prefs(cx);
        cx.notify();
    }

    pub fn cycle_theme(&mut self, cx: &mut Context<Self>) {
        self.prefs.theme = self.prefs.theme.next();
        self.emit_prefs(cx);
        cx.notify();
    }

    pub fn toggle_font(&mut self, cx: &mut Context<Self>) {
        self.prefs.font = self.prefs.font.toggle();
        self.needs_page_recompute = true;
        self.emit_prefs(cx);
        cx.notify();
    }

    pub fn cycle_width(&mut self, cx: &mut Context<Self>) {
        self.rendered_layouts.borrow_mut().clear();
        self.prefs.width = self.prefs.width.next();
        self.needs_page_recompute = true;
        self.emit_prefs(cx);
        cx.notify();
    }

    pub fn toggle_pagination(&mut self, cx: &mut Context<Self>) {
        let frac = self.current_scroll_pct();
        self.prefs.pagination = self.prefs.pagination.toggle();
        self.needs_page_recompute = true;
        if self.prefs.pagination == ReaderPaginationPref::Page {
            self.pending_scroll_fraction = Some(frac);
            self.schedule_pending_scroll(cx);
        } else {
            self.pending_last_page = false;
        }
        self.emit_prefs(cx);
        cx.notify();
    }

    pub fn cycle_spacing(&mut self, cx: &mut Context<Self>) {
        self.prefs.spacing = self.prefs.spacing.next();
        self.needs_page_recompute = true;
        self.emit_prefs(cx);
        cx.notify();
    }

    pub fn mark_finished(&mut self, cx: &mut Context<Self>) {
        self.emit_progress(true, cx);
        cx.notify();
    }

    pub fn toggle_bookmarks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.bookmarks_open = !self.bookmarks_open;
        if self.bookmarks_open {
            self.find_open = false;
            self.clear_selection_and_popup();
            if self.annotations_tab == AnnotationsTab::Bookmarks {
                self.bookmark_note_input.focus_handle(cx).focus(window, cx);
            }
        }
        cx.notify();
    }

    pub fn add_bookmark(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let note = self
            .bookmark_note_input
            .read(cx)
            .value()
            .trim()
            .to_string();
        let ch_title = self
            .book
            .chapters
            .get(self.chapter_idx)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| format!("Chapter {}", self.chapter_idx + 1));
        let scroll_pct = self.current_scroll_pct();
        let now = chrono::Utc::now();
        let id = format!(
            "bm-{}-{}",
            now.timestamp_millis(),
            self.bookmarks.len() + 1
        );
        self.bookmarks.push(BookBookmark {
            id,
            chapter_idx: self.chapter_idx,
            chapter_title: ch_title,
            scroll_pct,
            note,
            created_at: now.to_rfc3339(),
        });
        self.bookmark_note_input
            .update(cx, |s, cx| s.set_value("", window, cx));
        cx.emit(EpubViewEvent::BookmarksChanged {
            rel_path: self.rel_path.clone(),
            bookmarks: self.bookmarks.clone(),
            deleted_id: None,
        });
        cx.notify();
    }

    pub fn remove_bookmark(&mut self, id: &str, cx: &mut Context<Self>) {
        let prev_len = self.bookmarks.len();
        self.bookmarks.retain(|b| b.id != id);
        if self.bookmarks.len() != prev_len {
            cx.emit(EpubViewEvent::BookmarksChanged {
                rel_path: self.rel_path.clone(),
                bookmarks: self.bookmarks.clone(),
                deleted_id: Some(id.to_string()),
            });
            cx.notify();
        }
    }

    pub fn jump_to_bookmark(&mut self, bm: &BookBookmark, cx: &mut Context<Self>) {
        if self.book.chapters.is_empty() {
            return;
        }
        let target_ch = bm.chapter_idx.min(self.book.chapters.len() - 1);
        let frac = bm.scroll_pct.clamp(0.0, 1.0);
        let switched = self.chapter_idx != target_ch;
        self.scroll_jump_gen = self.scroll_jump_gen.wrapping_add(1);
        if switched {
            self.rendered_layouts.borrow_mut().clear();
            self.clear_selection_and_popup();
            self.chapter_idx = target_ch;
            self.scroll_handle.set_offset(point(px(0.), px(0.)));
            self.last_reported_scroll_pct = frac;
            self.emit_progress(false, cx);
        }
        self.pending_scroll_block = None;
        self.pending_scroll_fraction = Some(frac);
        if switched || !self.apply_pending_scroll() {
            self.schedule_pending_scroll(cx);
        } else {
            self.emit_progress(false, cx);
        }
        cx.notify();
    }

    pub fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.bookmarks_open = false;
        self.clear_selection_and_popup();
        self.find_open = true;
        self.find_needs_focus = true;
        let query = self.find_input.read(cx).value().to_string();
        if query != self.find_query {
            self.find_query = query;
        }
        self.recompute_find(false, cx);
        self.find_input.update(cx, |s, cx| {
            let len = s.value().len();
            s.focus(window, cx);
            if len > 0 {
                s.set_selected_range(0..len, cx);
            }
        });
        cx.notify();
    }

    pub fn close_find(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        if !self.find_open && !self.bookmarks_open && self.popup.is_none() {
            return;
        }
        self.find_open = false;
        self.bookmarks_open = false;
        self.find_needs_focus = false;
        self.pending_scroll_block = None;
        self.pending_scroll_fraction = None;
        self.scroll_jump_gen = self.scroll_jump_gen.wrapping_add(1);
        self.clear_selection_and_popup();
        if let Some(window) = window {
            self.focus_handle.focus(window, cx);
        }
        cx.notify();
    }

    pub fn is_find_open(&self) -> bool {
        self.find_open || self.bookmarks_open || self.popup.is_some()
    }

    pub fn toggle_find_case(&mut self, cx: &mut Context<Self>) {
        self.find_case_sensitive = !self.find_case_sensitive;
        self.recompute_find(true, cx);
        cx.notify();
    }

    pub fn clear_find_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_input.update(cx, |s, cx| s.set_value("", window, cx));
        self.find_query.clear();
        self.recompute_find(false, cx);
        self.find_input.focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    pub fn step_find(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.find_matches.is_empty() {
            return;
        }
        let len = self.find_matches.len();
        self.find_active = if forward {
            (self.find_active + 1) % len
        } else {
            (self.find_active + len - 1) % len
        };
        self.jump_to_active_match(cx);
    }

    fn recompute_find(&mut self, jump_to_nearest: bool, cx: &mut Context<Self>) {
        self.find_matches.clear();
        let query = self.find_query.trim();
        if query.is_empty() {
            self.find_active = 0;
            self.pending_scroll_block = None;
            self.pending_scroll_fraction = None;
            return;
        }
        for (ch_ix, ch) in self.book.chapters.iter().enumerate() {
            for (blk_ix, blk) in ch.blocks.iter().enumerate() {
                let text = block_plain_text(blk).unwrap_or("");
                if text.is_empty() {
                    continue;
                }
                for range in find_all_byte_ranges(text, query, self.find_case_sensitive) {
                    self.find_matches.push((ch_ix, blk_ix, range));
                }
            }
        }
        if self.find_matches.is_empty() {
            self.find_active = 0;
            self.pending_scroll_fraction = None;
            return;
        }
        if jump_to_nearest {
            let cur_ch = self.chapter_idx;
            self.find_active = self
                .find_matches
                .iter()
                .position(|(ch, _, _)| *ch >= cur_ch)
                .unwrap_or(0);
            self.jump_to_active_match(cx);
        } else if self.find_active >= self.find_matches.len() {
            self.find_active = 0;
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

    fn apply_pending_scroll(&mut self) -> bool {
        if self.pending_scroll_block.is_none() && self.pending_scroll_fraction.is_none() {
            return true;
        }
        let scroll_bounds = self.scroll_handle.bounds();
        let vp_h = f32::from(scroll_bounds.size.height);
        let ch_has_text = self
            .book
            .chapters
            .get(self.chapter_idx)
            .is_some_and(|c| c.blocks.iter().any(|b| block_plain_text(b).is_some()));
        if vp_h <= 0.0 || (ch_has_text && self.rendered_layouts.borrow().is_empty()) {
            return false;
        }

        let max_y = f32::from(self.scroll_handle.max_offset().y);
        if let Some((target_blk, target_byte)) = self.pending_scroll_block {
            let layouts = self.rendered_layouts.borrow();
            if let Some((_, len, layout)) = layouts.iter().find(|(b, _, _)| *b == target_blk) {
                let win_y = if target_byte > 0 {
                    layout
                        .position_for_index(target_byte.min(*len))
                        .map(|p| f32::from(p.y))
                        .unwrap_or_else(|| f32::from(layout.bounds().origin.y))
                } else {
                    f32::from(layout.bounds().origin.y)
                };
                let cur_off_y = f32::from(self.scroll_handle.offset().y);
                let vp_top_y = f32::from(scroll_bounds.top());
                let top_margin = (vp_h * 0.12).clamp(28.0, 64.0);
                let target_y = Self::compute_block_scroll_target_y(
                    win_y,
                    vp_top_y,
                    cur_off_y,
                    max_y,
                    top_margin,
                );
                drop(layouts);
                self.pending_scroll_block = None;
                self.pending_scroll_fraction = None;
                if self.prefs.pagination == ReaderPaginationPref::Page {
                    let vp_h = f32::from(scroll_bounds.size.height);
                    let total_content_h = vp_h + max_y.max(0.0);
                    let bands = self.collect_line_bands(vp_top_y, cur_off_y);
                    let pages = compute_chapter_page_offsets(vp_h, total_content_h, &bands);
                    let p_ix = page_index_for_content_y(&pages, target_y);
                    self.current_page = p_ix;
                    let snapped = pages.get(p_ix).map(|p| p.0).unwrap_or(0.0).clamp(0.0, max_y.max(0.0));
                    *self.page_offsets.borrow_mut() = pages;
                    if max_y > 0.0 {
                        self.scroll_handle.set_offset(point(px(0.), px(-snapped)));
                    }
                } else if max_y > 0.0 {
                    self.scroll_handle.set_offset(point(px(0.), px(-target_y)));
                }
                return true;
            }
            drop(layouts);
            self.pending_scroll_block = None;
        }

        if let Some(frac) = self.pending_scroll_fraction.take() {
            let target_y = if max_y > 0.0 {
                (frac.clamp(0.0, 1.0) * max_y).clamp(0.0, max_y)
            } else {
                0.0
            };
            if self.prefs.pagination == ReaderPaginationPref::Page {
                let vp_h = f32::from(scroll_bounds.size.height);
                let total_content_h = vp_h + max_y.max(0.0);
                let vp_top_y = f32::from(scroll_bounds.top());
                let cur_off_y = f32::from(self.scroll_handle.offset().y);
                let bands = self.collect_line_bands(vp_top_y, cur_off_y);
                let pages = compute_chapter_page_offsets(vp_h, total_content_h, &bands);
                let p_ix = page_index_for_content_y(&pages, target_y);
                self.current_page = p_ix;
                let snapped = pages.get(p_ix).map(|p| p.0).unwrap_or(0.0).clamp(0.0, max_y.max(0.0));
                *self.page_offsets.borrow_mut() = pages;
                if max_y > 0.0 {
                    self.scroll_handle.set_offset(point(px(0.), px(-snapped)));
                }
            } else if max_y > 0.0 {
                self.scroll_handle.set_offset(point(px(0.), px(-target_y)));
            }
        }
        true
    }

    fn schedule_pending_scroll(&mut self, cx: &mut Context<Self>) {
        self.scroll_jump_gen = self.scroll_jump_gen.wrapping_add(1);
        let jump_gen = self.scroll_jump_gen;
        cx.spawn(async move |this, cx| {
            for delay_ms in [16u64, 32, 64, 128, 250] {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(delay_ms))
                    .await;
                let mut stop = false;
                let updated = this.update(cx, |v, cx| {
                    if v.scroll_jump_gen != jump_gen {
                        stop = true;
                        return;
                    }
                    if v.apply_pending_scroll() {
                        v.emit_progress(false, cx);
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

    fn jump_to_active_match(&mut self, cx: &mut Context<Self>) {
        let Some((ch_ix, blk_ix, range)) = self.find_matches.get(self.find_active) else {
            return;
        };
        let (ch_ix, blk_ix, byte_start) = (*ch_ix, *blk_ix, range.start);
        let switched_chapter = self.chapter_idx != ch_ix && ch_ix < self.book.chapters.len();
        if switched_chapter {
            self.rendered_layouts.borrow_mut().clear();
            self.clear_selection_and_popup();
            self.chapter_idx = ch_ix;
            self.scroll_handle.set_offset(point(px(0.), px(0.)));
            self.emit_progress(false, cx);
        }
        let total_blocks = self
            .book
            .chapters
            .get(ch_ix)
            .map(|c| c.blocks.len().max(1))
            .unwrap_or(1);
        let frac = ((blk_ix as f32 / total_blocks as f32) - 0.08).clamp(0.0, 1.0);
        self.scroll_jump_gen = self.scroll_jump_gen.wrapping_add(1);
        self.pending_scroll_block = Some((blk_ix, byte_start));
        self.pending_scroll_fraction = Some(frac);
        if switched_chapter || !self.apply_pending_scroll() {
            self.schedule_pending_scroll(cx);
        } else {
            self.emit_progress(false, cx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_block_text(
        &self,
        text: &str,
        runs: &[EpubRun],
        ch_ix: usize,
        blk_ix: usize,
        code_bg: Hsla,
        code_fg: Hsla,
        sel_bg: Hsla,
        light_paper: bool,
    ) -> AnyElement {
        let mut block_hls: Vec<(&BookHighlight, Range<usize>)> = Vec::new();
        for hl in &self.highlights {
            if let Some(range) = hl.block_byte_range(ch_ix, blk_ix, text.len()) {
                block_hls.push((hl, range));
            }
        }

        let mut match_ranges: Vec<Range<usize>> = Vec::new();
        let mut active_rel: Option<usize> = None;
        if self.find_open && !self.find_query.trim().is_empty() {
            for (ix, (m_ch, m_blk, range)) in self.find_matches.iter().enumerate() {
                if *m_ch == ch_ix && *m_blk == blk_ix {
                    if ix == self.find_active {
                        active_rel = Some(match_ranges.len());
                    }
                    match_ranges.push(range.clone());
                }
            }
        }

        let sel_range = if ch_ix == self.chapter_idx {
            self.block_selection_range(blk_ix, text.len())
        } else {
            None
        };

        let merged = flatten_block_highlights(
            text,
            runs,
            &block_hls,
            &match_ranges,
            active_rel,
            sel_range,
            code_bg,
            code_fg,
            sel_bg,
            light_paper,
        );

        let styled = if merged.is_empty() {
            StyledText::new(text.to_string())
        } else {
            StyledText::new(text.to_string()).with_highlights(merged)
        };
        self.rendered_layouts
            .borrow_mut()
            .push((blk_ix, text.len(), styled.layout().clone()));
        styled.into_any_element()
    }

    fn render_find_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let total = self.find_matches.len();
        let query_non_empty = !self.find_query.is_empty();
        let trimmed_non_empty = !self.find_query.trim().is_empty();
        let count_text = if !trimmed_non_empty {
            String::new()
        } else if total == 0 {
            "0 / 0".to_string()
        } else {
            let ch = self.find_matches[self.find_active].0 + 1;
            format!("{} / {} · Ch {}", self.find_active + 1, total, ch)
        };
        let clear_btn = Button::new(SharedString::from(format!(
            "epub-find-x-{:?}",
            cx.entity_id()
        )))
        .text()
        .xsmall()
        .tab_stop(false)
        .icon(IconName::Close)
        .tooltip("Clear")
        .on_click(cx.listener(|this, _, window, cx| {
            this.clear_find_query(window, cx);
        }));

        floating_find_card(SharedString::from(format!(
            "epub-find-card-{:?}",
            cx.entity_id()
        )))
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
            if ev.keystroke.key.as_str() == "escape" {
                cx.stop_propagation();
                this.close_find(Some(window), cx);
            } else if ev.keystroke.key.as_str() == "f3" {
                cx.stop_propagation();
                this.step_find(!ev.keystroke.modifiers.shift, cx);
                cx.notify();
            }
        }))
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child(
                    div().w(px(FIND_INPUT_W)).child(
                        Input::new(&self.find_input)
                            .xsmall()
                            .prefix(
                                Icon::new(IconName::Search)
                                    .xsmall()
                                    .text_color(hex_color(kd::FG_MUTED)),
                            )
                            .when(query_non_empty, |i| i.suffix(clear_btn)),
                    ),
                )
                .when(!count_text.is_empty(), |row| {
                    row.child(find_count_badge(count_text, total == 0 && trimmed_non_empty))
                })
                .child(
                    Button::new(SharedString::from(format!(
                        "epub-find-case-{:?}",
                        cx.entity_id()
                    )))
                    .with_variant(if self.find_case_sensitive {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .xsmall()
                    .label("Aa")
                    .tooltip("Match Case")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_find_case(cx);
                    })),
                )
                .child(
                    Button::new(SharedString::from(format!(
                        "epub-find-prev-{:?}",
                        cx.entity_id()
                    )))
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronUp)
                    .tooltip("Previous Match (Shift+Enter)")
                    .disabled(total == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.step_find(false, cx);
                        cx.notify();
                    })),
                )
                .child(
                    Button::new(SharedString::from(format!(
                        "epub-find-next-{:?}",
                        cx.entity_id()
                    )))
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronDown)
                    .tooltip("Next Match (Enter)")
                    .disabled(total == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.step_find(true, cx);
                        cx.notify();
                    })),
                )
                .child(
                    Button::new(SharedString::from(format!(
                        "epub-find-close-{:?}",
                        cx.entity_id()
                    )))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip("Close (Esc)")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close_find(Some(window), cx);
                    })),
                ),
        )
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .occlude()
        .into_any_element()
    }

    fn render_style_icon(style: HighlightDecoration) -> AnyElement {
        match style {
            HighlightDecoration::Highlight => Icon::new(IconName::Highlighter).xsmall().into_any_element(),
            HighlightDecoration::Underline => Icon::new(IconName::Underline).xsmall().into_any_element(),
            HighlightDecoration::Squiggly => div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .child("~")
                .into_any_element(),
            HighlightDecoration::Strikethrough => {
                Icon::new(IconName::Strikethrough).xsmall().into_any_element()
            }
        }
    }

    fn render_highlight_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let popup = self.popup.clone()?;
        let vp = self.viewport_bounds.get();
        let card_bg = hex_color(kd::BG_SIDEBAR);
        let panel_bg = hex_color(kd::BG_PANEL);
        let border = hex_color(kd::BORDER);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let deep_bg = hex_color(kd::BG_DEEP);

        let vp_w = f32::from(vp.size.width).max(400.0);
        let vp_h = f32::from(vp.size.height).max(300.0);

        match popup {
            ActiveHighlightPopup::Selection {
                anchor_pos,
                note_mode,
                selected_color,
                selected_style,
            } => {
                let (start, end) = self.normalized_selection()?;
                let quote = self.extract_text_for_range(start, end);
                if quote.trim().is_empty() {
                    return None;
                }

                let (pop_w, pop_h) = if note_mode { (360.0, 248.0) } else { (346.0, 44.0) };
                let lx = f32::from(anchor_pos.x - vp.origin.x);
                let ly = f32::from(anchor_pos.y - vp.origin.y);
                let left = (lx - pop_w * 0.5).clamp(12.0, (vp_w - pop_w - 12.0).max(12.0));
                let top = if ly > pop_h + 20.0 {
                    ly - pop_h - 10.0
                } else {
                    ly + 24.0
                }
                .clamp(10.0, (vp_h - pop_h - 10.0).max(10.0));

                if !note_mode {
                    let quote_copy = quote;
                    return Some(
                        h_flex()
                            .id("epub-hl-sel-bar")
                            .absolute()
                            .left(px(left))
                            .top(px(top))
                            .h(px(40.))
                            .px_2p5()
                            .gap_1p5()
                            .items_center()
                            .rounded(px(10.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(border)
                            .shadow_md()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_scroll(|_, _, cx| cx.stop_propagation())
                            .occlude()
                            .children(HighlightColor::all().iter().copied().map(|col| {
                                let is_cur = col == selected_color;
                                let solid = hex_color(col.rgb_hex());
                                div()
                                    .id(("epub-hl-swatch", col as usize))
                                    .w(px(20.))
                                    .h(px(20.))
                                    .rounded_full()
                                    .bg(solid)
                                    .border_2()
                                    .border_color(if is_cur { fg } else { deep_bg })
                                    .cursor_pointer()
                                    .hover(|s| s.border_color(fg))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.commit_selection_highlight(
                                            col,
                                            selected_style,
                                            String::new(),
                                            window,
                                            cx,
                                        );
                                    }))
                            }))
                            .child(div().w(px(1.)).h(px(18.)).bg(border))
                            .children(HighlightDecoration::all().iter().copied().map(|dec| {
                                let is_cur = dec == selected_style;
                                Button::new(SharedString::from(format!("epub-hl-dec-{}", dec.label())))
                                    .with_variant(if is_cur {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .xsmall()
                                    .tooltip(dec.label())
                                    .child(Self::render_style_icon(dec))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(ActiveHighlightPopup::Selection {
                                            selected_style: st,
                                            ..
                                        }) = &mut this.popup
                                        {
                                            *st = dec;
                                        }
                                        this.prefs.last_highlight_style = dec;
                                        this.emit_prefs(cx);
                                        cx.notify();
                                    }))
                            }))
                            .child(div().w(px(1.)).h(px(18.)).bg(border))
                            .child(
                                Button::new("epub-hl-note-btn")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::StickyNote)
                                    .label("Note")
                                    .tooltip("Highlight with Note")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.note_editor.update(cx, |ed, cx| {
                                            ed.set_value("", window, cx);
                                        });
                                        if let Some(ActiveHighlightPopup::Selection {
                                            note_mode,
                                            ..
                                        }) = &mut this.popup
                                        {
                                            *note_mode = true;
                                        }
                                        this.note_needs_focus = true;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("epub-hl-copy-btn")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Copy)
                                    .tooltip("Copy Selected Passage")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            quote_copy.clone(),
                                        ));
                                        this.clear_selection_and_popup();
                                        this.focus_handle.focus(window, cx);
                                        cx.notify();
                                    })),
                            )
                            .into_any_element(),
                    );
                }

                if self.note_needs_focus {
                    self.note_needs_focus = false;
                    self.note_editor.focus_handle(cx).focus(window, cx);
                }

                let accent_border = hex_color(selected_color.rgb_hex());
                Some(
                    v_flex()
                        .id("epub-hl-note-card")
                        .absolute()
                        .left(px(left))
                        .top(px(top))
                        .w(px(pop_w))
                        .p_3()
                        .gap_2p5()
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(border)
                        .shadow_md()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_scroll(|_, _, cx| cx.stop_propagation())
                        .occlude()
                        .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                            if ev.keystroke.key.as_str() == "escape" {
                                cx.stop_propagation();
                                this.clear_selection_and_popup();
                                this.focus_handle.focus(window, cx);
                                cx.notify();
                            }
                        }))
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    h_flex().gap_1p5().items_center().children(
                                        HighlightColor::all().iter().copied().map(|col| {
                                            let is_cur = col == selected_color;
                                            div()
                                                .id(("epub-note-col", col as usize))
                                                .w(px(18.))
                                                .h(px(18.))
                                                .rounded_full()
                                                .bg(hex_color(col.rgb_hex()))
                                                .border_2()
                                                .border_color(if is_cur { fg } else { deep_bg })
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if let Some(ActiveHighlightPopup::Selection {
                                                        selected_color: sc,
                                                        ..
                                                    }) = &mut this.popup
                                                    {
                                                        *sc = col;
                                                    }
                                                    cx.notify();
                                                }))
                                        }),
                                    ),
                                )
                                .child(
                                    h_flex().gap_0p5().items_center().children(
                                        HighlightDecoration::all().iter().copied().map(|dec| {
                                            let is_cur = dec == selected_style;
                                            Button::new(SharedString::from(format!(
                                                "epub-note-dec-{}",
                                                dec.label()
                                            )))
                                            .with_variant(if is_cur {
                                                ButtonVariant::Secondary
                                            } else {
                                                ButtonVariant::Ghost
                                            })
                                            .xsmall()
                                            .tooltip(dec.label())
                                            .child(Self::render_style_icon(dec))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(ActiveHighlightPopup::Selection {
                                                    selected_style: ss,
                                                    ..
                                                }) = &mut this.popup
                                                {
                                                    *ss = dec;
                                                }
                                                cx.notify();
                                            }))
                                        }),
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .pl_2()
                                .py_1()
                                .border_l_2()
                                .border_color(accent_border)
                                .text_xs()
                                .italic()
                                .text_color(muted)
                                .max_h(px(40.))
                                .overflow_hidden()
                                .child(quote),
                        )
                        .child(
                            div()
                                .h(px(92.))
                                .w_full()
                                .rounded(px(6.))
                                .bg(deep_bg)
                                .border_1()
                                .border_color(border)
                                .child(
                                    Editor::new(&self.note_editor)
                                        .size_full()
                                        .appearance(false)
                                        .bordered(false),
                                ),
                        )
                        .child(
                            h_flex()
                                .justify_between()
                                .gap_2()
                                .child(
                                    Button::new("epub-note-cancel")
                                        .ghost()
                                        .xsmall()
                                        .label("Cancel")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.clear_selection_and_popup();
                                            this.focus_handle.focus(window, cx);
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("epub-note-save")
                                        .primary()
                                        .xsmall()
                                        .icon(IconName::Check)
                                        .label("Save Highlight & Note")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            let note =
                                                this.note_editor.read(cx).value().to_string();
                                            this.commit_selection_highlight(
                                                selected_color,
                                                selected_style,
                                                note,
                                                window,
                                                cx,
                                            );
                                        })),
                                ),
                        )
                        .into_any_element(),
                )
            }
            ActiveHighlightPopup::Inspect {
                highlight_id,
                anchor_pos,
            } => {
                let hl = self.highlights.iter().find(|h| h.id == highlight_id)?.clone();
                let (pop_w, pop_h) = (360.0, 260.0);
                let lx = f32::from(anchor_pos.x - vp.origin.x);
                let ly = f32::from(anchor_pos.y - vp.origin.y);
                let left = (lx - pop_w * 0.5).clamp(12.0, (vp_w - pop_w - 12.0).max(12.0));
                let top = if ly > pop_h + 20.0 {
                    ly - pop_h - 10.0
                } else {
                    ly + 24.0
                }
                .clamp(10.0, (vp_h - pop_h - 10.0).max(10.0));

                if self.note_needs_focus {
                    self.note_needs_focus = false;
                    self.note_editor.focus_handle(cx).focus(window, cx);
                }

                let hl_id_copy = hl.id.clone();
                let hl_id_del = hl.id.clone();
                let hl_id_save = hl.id.clone();
                let quote_copy = hl.quote.clone();

                Some(
                    v_flex()
                        .id("epub-hl-inspect-card")
                        .absolute()
                        .left(px(left))
                        .top(px(top))
                        .w(px(pop_w))
                        .p_3()
                        .gap_2p5()
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(border)
                        .shadow_md()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_scroll(|_, _, cx| cx.stop_propagation())
                        .occlude()
                        .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                            if ev.keystroke.key.as_str() == "escape" {
                                cx.stop_propagation();
                                this.clear_selection_and_popup();
                                this.focus_handle.focus(window, cx);
                                cx.notify();
                            }
                        }))
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    h_flex().gap_1p5().items_center().children(
                                        HighlightColor::all().iter().copied().map(|col| {
                                            let is_cur = col == hl.color;
                                            let target_id = hl_id_copy.clone();
                                            div()
                                                .id(("epub-insp-col", col as usize))
                                                .w(px(18.))
                                                .h(px(18.))
                                                .rounded_full()
                                                .bg(hex_color(col.rgb_hex()))
                                                .border_2()
                                                .border_color(if is_cur { fg } else { deep_bg })
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.update_highlight_appearance(
                                                        &target_id,
                                                        Some(col),
                                                        None,
                                                        cx,
                                                    );
                                                }))
                                        }),
                                    ),
                                )
                                .child(
                                    h_flex().gap_0p5().items_center().children(
                                        HighlightDecoration::all().iter().copied().map(|dec| {
                                            let is_cur = dec == hl.style;
                                            let target_id = hl_id_copy.clone();
                                            Button::new(SharedString::from(format!(
                                                "epub-insp-dec-{}",
                                                dec.label()
                                            )))
                                            .with_variant(if is_cur {
                                                ButtonVariant::Secondary
                                            } else {
                                                ButtonVariant::Ghost
                                            })
                                            .xsmall()
                                            .tooltip(dec.label())
                                            .child(Self::render_style_icon(dec))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.update_highlight_appearance(
                                                    &target_id,
                                                    None,
                                                    Some(dec),
                                                    cx,
                                                );
                                            }))
                                        }),
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .pl_2()
                                .py_1()
                                .border_l_2()
                                .border_color(hex_color(hl.color.rgb_hex()))
                                .rounded_r(px(4.))
                                .bg(panel_bg)
                                .text_xs()
                                .italic()
                                .text_color(fg)
                                .max_h(px(44.))
                                .overflow_hidden()
                                .child(hl.quote.clone()),
                        )
                        .child(
                            div()
                                .h(px(92.))
                                .w_full()
                                .rounded(px(6.))
                                .bg(deep_bg)
                                .border_1()
                                .border_color(border)
                                .child(
                                    Editor::new(&self.note_editor)
                                        .size_full()
                                        .appearance(false)
                                        .bordered(false),
                                ),
                        )
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("epub-insp-copy")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Copy)
                                                .tooltip("Copy Highlight Quote")
                                                .on_click(cx.listener(move |_, _, _, cx| {
                                                    cx.write_to_clipboard(
                                                        ClipboardItem::new_string(quote_copy.clone()),
                                                    );
                                                })),
                                        )
                                        .child(
                                            Button::new("epub-insp-del")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Trash)
                                                .label("Delete")
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.delete_highlight(
                                                        &hl_id_del,
                                                        Some(window),
                                                        cx,
                                                    );
                                                })),
                                        ),
                                )
                                .child(
                                    Button::new("epub-insp-save")
                                        .primary()
                                        .xsmall()
                                        .icon(IconName::Check)
                                        .label("Save Note")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.save_inspected_highlight_note(
                                                &hl_id_save,
                                                window,
                                                cx,
                                            );
                                        })),
                                ),
                        )
                        .into_any_element(),
                )
            }
        }
    }

    fn render_annotations_popover(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let card_bg = hex_color(kd::BG_SIDEBAR);
        let panel_bg = hex_color(kd::BG_PANEL);
        let deep_bg = hex_color(kd::BG_DEEP);
        let border = hex_color(kd::BORDER);
        let fg = hex_color(kd::FG);
        let muted = hex_color(kd::FG_MUTED);
        let accent = hex_color(kd::ACCENT);

        let body: AnyElement = match self.annotations_tab {
            AnnotationsTab::Highlights => {
                let mut filtered: Vec<&BookHighlight> = self
                    .highlights
                    .iter()
                    .filter(|h| {
                        if let Some(cf) = self.annotations_color_filter
                            && h.color != cf
                        {
                            return false;
                        }
                        if self.annotations_notes_only && h.note.trim().is_empty() {
                            return false;
                        }
                        true
                    })
                    .collect();
                filtered.sort_by_key(|h| (h.chapter_idx, h.start_block, h.start_offset));

                let filter_bar = h_flex()
                    .gap_1()
                    .items_center()
                    .flex_wrap()
                    .child(
                        Button::new("epub-ann-flt-all")
                            .with_variant(if self.annotations_color_filter.is_none() {
                                ButtonVariant::Secondary
                            } else {
                                ButtonVariant::Ghost
                            })
                            .xsmall()
                            .label("All")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.annotations_color_filter = None;
                                this.annotations_hl_scroll.set_offset(point(px(0.), px(0.)));
                                cx.notify();
                            })),
                    )
                    .children(HighlightColor::all().iter().copied().map(|col| {
                        let active = self.annotations_color_filter == Some(col);
                        div()
                            .id(("epub-ann-col", col as usize))
                            .w(px(16.))
                            .h(px(16.))
                            .rounded_full()
                            .bg(hex_color(col.rgb_hex()))
                            .border_2()
                            .border_color(if active { fg } else { deep_bg })
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.annotations_color_filter = if active { None } else { Some(col) };
                                this.annotations_hl_scroll.set_offset(point(px(0.), px(0.)));
                                cx.notify();
                            }))
                    }))
                    .child(
                        Button::new("epub-ann-flt-notes")
                            .with_variant(if self.annotations_notes_only {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Ghost
                            })
                            .xsmall()
                            .icon(IconName::StickyNote)
                            .label("Notes")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.annotations_notes_only = !this.annotations_notes_only;
                                this.annotations_hl_scroll.set_offset(point(px(0.), px(0.)));
                                cx.notify();
                            })),
                    );

                let rows: Vec<AnyElement> = filtered
                    .into_iter()
                    .enumerate()
                    .map(|(ix, hl)| {
                        let hl_jump = hl.clone();
                        let hl_del_id = hl.id.clone();
                        let col_bar = hex_color(hl.color.rgb_hex());
                        let header = if hl.chapter_title.trim().is_empty() {
                            format!("Ch {} · {}", hl.chapter_idx + 1, hl.style.label())
                        } else {
                            format!(
                                "Ch {} · {} · {}",
                                hl.chapter_idx + 1,
                                hl.chapter_title.trim(),
                                hl.style.label()
                            )
                        };
                        let quote = hl.quote.clone();
                        let note = hl.note.trim().to_string();

                        h_flex()
                            .id(("epub-hl-row", ix))
                            .w_full()
                            .px_2p5()
                            .py_2()
                            .gap_2()
                            .items_start()
                            .rounded(px(6.))
                            .bg(panel_bg)
                            .border_l_4()
                            .border_color(col_bar)
                            .cursor_pointer()
                            .hover(|s| s.bg(hex_alpha(0x8ba4b026)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.jump_to_highlight(&hl_jump, cx);
                            }))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(col_bar)
                                            .truncate()
                                            .child(header),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .italic()
                                            .text_color(fg)
                                            .max_h(px(36.))
                                            .overflow_hidden()
                                            .child(quote),
                                    )
                                    .when(!note.is_empty(), |col| {
                                        col.child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .rounded(px(4.))
                                                .bg(deep_bg)
                                                .text_xs()
                                                .text_color(accent)
                                                .child(note),
                                        )
                                    }),
                            )
                            .child(
                                Button::new(SharedString::from(format!("epub-hl-row-del-{ix}")))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Close)
                                    .tooltip("Delete Highlight")
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.delete_highlight(&hl_del_id, Some(window), cx);
                                    })),
                            )
                            .into_any_element()
                    })
                    .collect();

                v_flex()
                    .gap_2()
                    .child(filter_bar)
                    .child(if rows.is_empty() {
                        div()
                            .py_4()
                            .text_xs()
                            .text_color(muted)
                            .child(if self.highlights.is_empty() {
                                "Select any text in the book to highlight or add a note."
                            } else {
                                "No highlights match this filter."
                            })
                            .into_any_element()
                    } else {
                        v_flex()
                            .relative()
                            .w_full()
                            .max_h(px(290.))
                            .overflow_hidden()
                            .child(
                                v_flex()
                                    .id(SharedString::from(format!(
                                        "epub-hl-list-{:?}",
                                        cx.entity_id()
                                    )))
                                    .w_full()
                                    .max_h(px(290.))
                                    .overflow_y_scroll()
                                    .track_scroll(&self.annotations_hl_scroll)
                                    .child(v_flex().flex_none().w_full().gap_1p5().pr_2().children(rows)),
                            )
                            .vertical_scrollbar(&self.annotations_hl_scroll)
                            .into_any_element()
                    })
                    .into_any_element()
            }
            AnnotationsTab::Bookmarks => {
                let items: Vec<AnyElement> = self
                    .bookmarks
                    .iter()
                    .enumerate()
                    .map(|(ix, bm)| {
                        let bm_jump = bm.clone();
                        let bm_del_id = bm.id.clone();
                        let pct = (bm.scroll_pct.clamp(0.0, 1.0) * 100.0).round() as u8;
                        let loc =
                            format!("Ch {} · {} · {pct}%", bm.chapter_idx + 1, bm.chapter_title);
                        let note = bm.note.clone();

                        h_flex()
                            .id(("epub-bm-row", ix))
                            .w_full()
                            .px_2p5()
                            .py_1p5()
                            .gap_2()
                            .items_center()
                            .justify_between()
                            .rounded(px(6.))
                            .bg(panel_bg)
                            .cursor_pointer()
                            .hover(|s| s.bg(hex_alpha(0x8ba4b026)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.jump_to_bookmark(&bm_jump, cx);
                            }))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(accent)
                                            .truncate()
                                            .child(loc),
                                    )
                                    .when(!note.is_empty(), |col| {
                                        col.child(
                                            div()
                                                .text_xs()
                                                .text_color(fg)
                                                .truncate()
                                                .child(note),
                                        )
                                    }),
                            )
                            .child(
                                Button::new(SharedString::from(format!("epub-bm-del-{ix}")))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Close)
                                    .tooltip("Delete Bookmark")
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.remove_bookmark(&bm_del_id, cx);
                                    })),
                            )
                            .into_any_element()
                    })
                    .collect();

                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.bookmark_note_input).xsmall()),
                            )
                            .child(
                                Button::new("epub-bm-add")
                                    .primary()
                                    .xsmall()
                                    .icon(IconName::BookmarkPlus)
                                    .label("Add")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_bookmark(window, cx);
                                    })),
                            ),
                    )
                    .child(if items.is_empty() {
                        div()
                            .py_3()
                            .text_xs()
                            .text_color(muted)
                            .child("No bookmarks in this book yet.")
                            .into_any_element()
                    } else {
                        v_flex()
                            .relative()
                            .w_full()
                            .max_h(px(250.))
                            .overflow_hidden()
                            .child(
                                v_flex()
                                    .id(SharedString::from(format!(
                                        "epub-bm-list-{:?}",
                                        cx.entity_id()
                                    )))
                                    .w_full()
                                    .max_h(px(250.))
                                    .overflow_y_scroll()
                                    .track_scroll(&self.annotations_bm_scroll)
                                    .child(v_flex().flex_none().w_full().gap_1p5().pr_2().children(items)),
                            )
                            .vertical_scrollbar(&self.annotations_bm_scroll)
                            .into_any_element()
                    })
                    .into_any_element()
            }
        };

        div()
            .id(SharedString::from(format!(
                "epub-ann-popover-{:?}",
                cx.entity_id()
            )))
            .absolute()
            .top(px(10.))
            .right(px(16.))
            .w(px(364.))
            .max_h(px(460.))
            .p_3()
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(border)
            .shadow_md()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll(|_, _, cx| cx.stop_propagation())
            .occlude()
            .child(
                v_flex()
                    .gap_2p5()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .gap_1p5()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(
                                        Button::new("epub-ann-tab-hl")
                                            .with_variant(
                                                if self.annotations_tab == AnnotationsTab::Highlights
                                                {
                                                    ButtonVariant::Primary
                                                } else {
                                                    ButtonVariant::Ghost
                                                },
                                            )
                                            .xsmall()
                                            .icon(IconName::Highlighter)
                                            .label(format!("Highlights ({})", self.highlights.len()))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.annotations_tab = AnnotationsTab::Highlights;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("epub-ann-tab-bm")
                                            .with_variant(
                                                if self.annotations_tab == AnnotationsTab::Bookmarks
                                                {
                                                    ButtonVariant::Primary
                                                } else {
                                                    ButtonVariant::Ghost
                                                },
                                            )
                                            .xsmall()
                                            .icon(IconName::Bookmark)
                                            .label(format!("Bookmarks ({})", self.bookmarks.len()))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.annotations_tab = AnnotationsTab::Bookmarks;
                                                this.bookmark_note_input
                                                    .focus_handle(cx)
                                                    .focus(window, cx);
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                Button::new("epub-ann-close")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.bookmarks_open = false;
                                        this.focus_handle.focus(window, cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(body),
            )
            .into_any_element()
    }

    pub fn go_to_chapter(&mut self, idx: usize, cx: &mut Context<Self>) {
        if self.book.chapters.is_empty() {
            return;
        }
        self.rendered_layouts.borrow_mut().clear();
        self.clear_selection_and_popup();
        self.chapter_idx = idx.min(self.book.chapters.len() - 1);
        self.current_page = 0;
        self.needs_page_recompute = true;
        *self.page_offsets.borrow_mut() = vec![(0.0, 600.0)];
        self.pending_scroll_block = None;
        self.pending_scroll_fraction = None;
        self.scroll_jump_gen = self.scroll_jump_gen.wrapping_add(1);
        self.scroll_handle.set_offset(point(px(0.), px(0.)));
        self.last_reported_scroll_pct = 0.0;
        self.emit_progress(false, cx);
        cx.notify();
    }

    pub fn next_chapter(&mut self, cx: &mut Context<Self>) {
        if self.chapter_idx + 1 < self.book.chapters.len() {
            self.go_to_chapter(self.chapter_idx + 1, cx);
        }
    }

    pub fn prev_chapter(&mut self, cx: &mut Context<Self>) {
        if self.chapter_idx > 0 {
            self.go_to_chapter(self.chapter_idx - 1, cx);
        }
    }

    pub fn outline_headings(&self) -> Vec<OutlineEntry> {
        self.book.outline_headings()
    }

    pub fn estimated_minutes_left(&self) -> usize {
        if self.is_finished || self.book.chapters.is_empty() {
            return 0;
        }
        let frac_done = self.overall_pct();
        let rem_words = (self.book.total_words as f32 * (1.0 - frac_done)).max(0.0);
        if rem_words < 10.0 {
            0
        } else {
            (rem_words / 230.0).ceil() as usize
        }
    }

    pub fn summary(&self) -> String {
        let pct = (self.overall_pct() * 100.0).round() as u8;
        let mins = self.estimated_minutes_left();
        if self.is_finished {
            format!(
                "Ch {}/{} · Finished · {} words",
                self.chapter_idx + 1,
                self.book.chapters.len().max(1),
                self.book.total_words
            )
        } else if mins > 0 {
            format!(
                "Ch {}/{} ({pct}%) · ~{mins}m left · {} words",
                self.chapter_idx + 1,
                self.book.chapters.len().max(1),
                self.book.total_words
            )
        } else {
            format!(
                "Ch {}/{} ({pct}%) · {} words",
                self.chapter_idx + 1,
                self.book.chapters.len().max(1),
                self.book.total_words
            )
        }
    }
}

impl Render for EpubView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.prefs.pagination == ReaderPaginationPref::Page
            && (self.sync_page_layout() || self.needs_page_recompute)
        {
            let entity = cx.entity().clone();
            window.on_next_frame(move |_, cx| {
                let _ = entity.update(cx, |_, cx| cx.notify());
            });
        }
        let cur_scroll = self.current_scroll_pct();
        if !self.scroll_save_scheduled
            && self.pending_scroll_block.is_none()
            && self.pending_scroll_fraction.is_none()
            && (cur_scroll - self.last_reported_scroll_pct).abs() >= 0.03
        {
            self.scroll_save_scheduled = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(450))
                    .await;
                let _ = this.update(cx, |v, cx| {
                    v.scroll_save_scheduled = false;
                    v.emit_progress(false, cx);
                });
            })
            .detach();
        }

        let (
            outer_bg,
            page_bg,
            text_fg,
            muted_fg,
            heading_fg,
            border_c,
            code_bg,
            code_fg,
            sel_bg,
            light_paper,
        ) = match self.prefs.theme {
            ReaderThemePref::Dark => (
                hex_color(kd::BG_DEEP),
                hex_color(kd::BG_MAIN),
                hex_color(kd::FG),
                hex_color(kd::FG_MUTED),
                hex_color(kd::ACCENT),
                hex_color(kd::BORDER_SUBTLE),
                hex_color(kd::BG_PANEL),
                hex_color(kd::ORANGE),
                hex_alpha(0x7e9cd855),
                false,
            ),
            ReaderThemePref::Sepia => (
                hex_color(0x1f1c18),
                hex_color(0xf4ecd8),
                hex_color(0x3b2f1e),
                hex_color(0x73634e),
                hex_color(0x8a4b25),
                hex_color(0xd8cbb2),
                hex_color(0xe7dcc4),
                hex_color(0x7a3e1b),
                hex_alpha(0x3b82f648),
                true,
            ),
            ReaderThemePref::Paper => (
                hex_color(0xe5e3dc),
                hex_color(0xfaf9f5),
                hex_color(0x1f2328),
                hex_color(0x656d76),
                hex_color(0x2d5a7b),
                hex_color(0xd8d5cc),
                hex_color(0xeeede6),
                hex_color(0x9a3412),
                hex_alpha(0x3b82f648),
                true,
            ),
            ReaderThemePref::Amoled => (
                hex_color(0x000000),
                hex_color(0x000000),
                hex_color(0xdcd8cc),
                hex_color(0x8a877d),
                hex_color(0xc4b28a),
                hex_color(0x1e1e24),
                hex_color(0x121216),
                hex_color(0xe69875),
                hex_alpha(0x7e9cd855),
                false,
            ),
        };

        let ch_count = self.book.chapters.len();
        let cur_ix = self.chapter_idx.min(ch_count.saturating_sub(1));
        let fs = self.prefs.font_size;
        let line_mult = self.prefs.spacing.multiplier();
        let reader_font: SharedString = match self.prefs.font {
            ReaderFontPref::Serif => "serif".into(),
            ReaderFontPref::Sans => cx.theme().font_family.clone(),
        };

        self.rendered_layouts.borrow_mut().clear();

        let mut chapter_blocks: Vec<AnyElement> = if let Some(ch) = self.book.chapters.get(cur_ix) {
            ch.blocks
                .iter()
                .enumerate()
                .map(|(blk_ix, blk)| match blk {
                    EpubBlock::Heading { level, text } => {
                        let scale = match level {
                            1 => 1.55,
                            2 => 1.32,
                            3 => 1.16,
                            _ => 1.05,
                        };
                        div()
                            .pt_3()
                            .cursor(CursorStyle::IBeam)
                            .text_size(px(fs * scale))
                            .font_weight(FontWeight::BOLD)
                            .text_color(heading_fg)
                            .child(self.render_block_text(
                                text,
                                &[],
                                cur_ix,
                                blk_ix,
                                code_bg,
                                code_fg,
                                sel_bg,
                                light_paper,
                            ))
                            .into_any_element()
                    }
                    EpubBlock::Paragraph {
                        text,
                        runs,
                        is_quote,
                    } => {
                        let p = div()
                            .cursor(CursorStyle::IBeam)
                            .text_size(px(fs))
                            .line_height(px(fs * line_mult))
                            .text_color(if *is_quote { muted_fg } else { text_fg })
                            .child(self.render_block_text(
                                text,
                                runs,
                                cur_ix,
                                blk_ix,
                                code_bg,
                                code_fg,
                                sel_bg,
                                light_paper,
                            ));
                        if *is_quote {
                            div()
                                .pl_4()
                                .py_1()
                                .border_l_2()
                                .border_color(heading_fg)
                                .italic()
                                .child(p)
                                .into_any_element()
                        } else {
                            p.into_any_element()
                        }
                    }
                    EpubBlock::ListItem { text, runs } => h_flex()
                        .pl_4()
                        .gap_2()
                        .items_start()
                        .child(
                            div()
                                .text_size(px(fs))
                                .text_color(heading_fg)
                                .child("•"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .cursor(CursorStyle::IBeam)
                                .text_size(px(fs))
                                .line_height(px(fs * (line_mult - 0.05)))
                                .text_color(text_fg)
                                .child(self.render_block_text(
                                    text,
                                    runs,
                                    cur_ix,
                                    blk_ix,
                                    code_bg,
                                    code_fg,
                                    sel_bg,
                                    light_paper,
                                )),
                        )
                        .into_any_element(),
                    EpubBlock::Image {
                        alt,
                        media_type,
                        zip_path,
                        bytes,
                    } => {
                        let decoded = if let Some(cached) =
                            self.decoded_images.borrow().get(&(cur_ix, blk_ix)).cloned()
                        {
                            Some(cached)
                        } else if let Some(fresh) = decode_bytes_image(media_type, zip_path, bytes) {
                            self.decoded_images
                                .borrow_mut()
                                .insert((cur_ix, blk_ix), Arc::clone(&fresh));
                            Some(fresh)
                        } else {
                            None
                        };
                        v_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .py_2()
                            .when_some(decoded, |col, image| {
                                col.child(
                                    img(image)
                                        .max_w(px(616.))
                                        .max_h(px(460.))
                                        .object_fit(ObjectFit::Contain)
                                        .rounded(px(6.)),
                                )
                            })
                            .when(!alt.is_empty(), |col| {
                                col.child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child(alt.clone()),
                                )
                            })
                            .into_any_element()
                    }
                    EpubBlock::Rule => div()
                        .w_full()
                        .h(px(1.))
                        .my_3()
                        .bg(border_c)
                        .into_any_element(),
                })
                .collect()
        } else {
            vec![
                div()
                    .text_sm()
                    .text_color(muted_fg)
                    .child("Empty EPUB book.")
                    .into_any_element(),
            ]
        };

        let ch_title = self
            .book
            .chapters
            .get(cur_ix)
            .map(|c| c.title.clone())
            .unwrap_or_default();
        let base_author_line = self
            .book
            .author
            .as_ref()
            .map(|a| format!("{} — {a}", self.book.title))
            .unwrap_or_else(|| self.book.title.clone());
        let author_line = match self.book.metadata.series.as_deref() {
            Some(s) if !s.trim().is_empty() => format!("{} · {base_author_line}", s.trim()),
            _ => base_author_line,
        };

        let find_bar = if self.find_open {
            if self.find_needs_focus {
                self.find_needs_focus = false;
                self.find_input.focus_handle(cx).focus(window, cx);
            }
            Some(self.render_find_bar(cx))
        } else {
            None
        };

        let annotations_popover = if self.bookmarks_open {
            Some(self.render_annotations_popover(cx))
        } else {
            None
        };

        let hl_popup = self.render_highlight_popup(window, cx);

        let body_element: AnyElement = if self.prefs.width == ReaderWidthPref::TwoColumn
            && chapter_blocks.len() >= 4
        {
            let mid = chapter_blocks.len().div_ceil(2);
            let right_col = chapter_blocks.split_off(mid);
            let left_col = chapter_blocks;
            h_flex()
                .w_full()
                .gap_8()
                .items_start()
                .child(v_flex().flex_1().min_w_0().gap_3p5().children(left_col))
                .child(div().w(px(1.)).self_stretch().bg(border_c))
                .child(v_flex().flex_1().min_w_0().gap_3p5().children(right_col))
                .into_any_element()
        } else {
            v_flex()
                .w_full()
                .gap_3p5()
                .children(chapter_blocks)
                .into_any_element()
        };

        let is_page_mode = self.prefs.pagination == ReaderPaginationPref::Page;
        let (page_idx, total_pages, page_mask_h) = if is_page_mode {
            let pages = self.page_offsets.borrow();
            let total_p = pages.len().max(1);
            let cur_p = self.current_page.min(total_p - 1);
            let vp_h = f32::from(self.scroll_handle.bounds().size.height);
            let clip_h = pages.get(cur_p).map(|p| p.1).unwrap_or(vp_h);
            let mask_h = if vp_h > 40.0 && clip_h + 1.0 < vp_h {
                (vp_h - clip_h).max(0.0)
            } else {
                0.0
            };
            (cur_p, total_p, mask_h)
        } else {
            (0, 1, 0.0)
        };

        let is_last_chapter = cur_ix + 1 >= ch_count;
        let at_book_end = is_last_chapter && (!is_page_mode || page_idx + 1 >= total_pages);
        let at_book_start = cur_ix == 0 && (!is_page_mode || page_idx == 0);
        let prev_label = if is_page_mode {
            "Previous Page"
        } else {
            "Previous Chapter"
        };
        let next_label = if is_page_mode && page_idx + 1 < total_pages {
            "Next Page"
        } else {
            "Next Chapter"
        };
        let footer_right = if at_book_end {
            Button::new("epub-foot-finish")
                .with_variant(if self.is_finished {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Primary
                })
                .small()
                .icon(IconName::BookCheck)
                .label(if self.is_finished {
                    "Finished"
                } else {
                    "Mark Finished"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.mark_finished(cx);
                }))
                .into_any_element()
        } else {
            Button::new("epub-foot-next")
                .ghost()
                .small()
                .icon(IconName::ChevronRight)
                .label(next_label)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.next_page_or_chapter(cx);
                }))
                .into_any_element()
        };

        let overall_pct = self.overall_pct();
        let pct_label = (overall_pct * 100.0).round() as u8;
        let mins_left = self.estimated_minutes_left();
        let page_prefix = if is_page_mode {
            format!("Page {}/{} · ", page_idx + 1, total_pages)
        } else {
            String::new()
        };
        let footer_meta = if self.is_finished {
            format!("{page_prefix}Ch {}/{} · 100% Finished", cur_ix + 1, ch_count.max(1))
        } else if mins_left > 0 {
            format!(
                "{page_prefix}Ch {}/{} · {pct_label}% · ~{mins_left} min left",
                cur_ix + 1,
                ch_count.max(1)
            )
        } else {
            format!("{page_prefix}Ch {}/{} · {pct_label}%", cur_ix + 1, ch_count.max(1))
        };

        let vp_slot = self.viewport_bounds.clone();

        v_flex()
            .id(SharedString::from(format!("epub-view-{:?}", cx.entity_id())))
            .relative()
            .size_full()
            .bg(outer_bg)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                match ev.keystroke.key.as_str() {
                    "escape" if this.find_open || this.bookmarks_open || this.popup.is_some() => {
                        cx.stop_propagation();
                        this.close_find(Some(window), cx);
                    }
                    "f3" if this.find_open => {
                        cx.stop_propagation();
                        this.step_find(!ev.keystroke.modifiers.shift, cx);
                        cx.notify();
                    }
                    "c" if ev.keystroke.modifiers.secondary(cx)
                        && !this.note_editor.focus_handle(cx).is_focused(window)
                        && this.normalized_selection().is_some() =>
                    {
                        if let Some((start, end)) = this.normalized_selection() {
                            let quote = this.extract_text_for_range(start, end);
                            if !quote.trim().is_empty() {
                                cx.stop_propagation();
                                cx.write_to_clipboard(ClipboardItem::new_string(quote));
                            }
                        }
                    }
                    "left" | "pageup"
                        if this.popup.is_none() && !this.find_open && !this.bookmarks_open =>
                    {
                        cx.stop_propagation();
                        this.prev_page_or_chapter(cx);
                    }
                    "right" | "pagedown"
                        if this.popup.is_none() && !this.find_open && !this.bookmarks_open =>
                    {
                        cx.stop_propagation();
                        this.next_page_or_chapter(cx);
                    }
                    "space"
                        if is_page_mode
                            && this.popup.is_none()
                            && !this.find_open
                            && !this.bookmarks_open =>
                    {
                        cx.stop_propagation();
                        if ev.keystroke.modifiers.shift {
                            this.prev_page_or_chapter(cx);
                        } else {
                            this.next_page_or_chapter(cx);
                        }
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .relative()
                    .child(
                        canvas(
                            move |bounds, _, _| vp_slot.set(bounds),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .relative()
                            .size_full()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "epub-ch-scroll-{:?}",
                                        cx.entity_id()
                                    )))
                                    .size_full()
                                    .flex()
                                    .flex_col()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.scroll_handle)
                                    .when(is_page_mode, |el| {
                                        el.on_scroll(cx.listener(|this, ev: &gpui_kit::ScrollEvent, _, cx| {
                                            cx.stop_propagation();
                                            let dy = f32::from(ev.delta.pixel_delta(px(24.)).y);
                                            if dy.abs() < 6.0 {
                                                return;
                                            }
                                            let ready = this.last_wheel_page_at.is_none_or(|t| {
                                                t.elapsed() >= std::time::Duration::from_millis(240)
                                            });
                                            if ready {
                                                this.last_wheel_page_at = Some(std::time::Instant::now());
                                                if dy < 0.0 {
                                                    this.next_page_or_chapter(cx);
                                                } else {
                                                    this.prev_page_or_chapter(cx);
                                                }
                                            }
                                        }))
                                    })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                                    this.focus_handle.focus(window, cx);
                                    if this.bookmarks_open {
                                        this.bookmarks_open = false;
                                    }
                                    let Some(hit) = this.hit_test_position(ev.position) else {
                                        this.clear_selection_and_popup();
                                        cx.notify();
                                        return;
                                    };
                                    if ev.click_count >= 3 {
                                        if let Some(txt) =
                                            this.current_chapter_block_text(hit.block_idx)
                                            && !txt.is_empty()
                                        {
                                            let start = TextPos {
                                                block_idx: hit.block_idx,
                                                byte_offset: 0,
                                            };
                                            let end = TextPos {
                                                block_idx: hit.block_idx,
                                                byte_offset: txt.len(),
                                            };
                                            this.selection_anchor = Some(start);
                                            this.selection_head = Some(end);
                                            this.is_dragging_selection = false;
                                            let anchor_pos =
                                                this.anchor_for_pos(start, ev.position);
                                            this.popup = Some(ActiveHighlightPopup::Selection {
                                                anchor_pos,
                                                note_mode: false,
                                                selected_color: this.prefs.last_highlight_color,
                                                selected_style: this.prefs.last_highlight_style,
                                            });
                                            cx.notify();
                                        }
                                        return;
                                    }
                                    if ev.click_count == 2 {
                                        if let Some(txt) =
                                            this.current_chapter_block_text(hit.block_idx)
                                        {
                                            let r = word_bounds_at(txt, hit.byte_offset);
                                            if r.start < r.end {
                                                let start = TextPos {
                                                    block_idx: hit.block_idx,
                                                    byte_offset: r.start,
                                                };
                                                let end = TextPos {
                                                    block_idx: hit.block_idx,
                                                    byte_offset: r.end,
                                                };
                                                this.selection_anchor = Some(start);
                                                this.selection_head = Some(end);
                                                this.is_dragging_selection = false;
                                                let anchor_pos =
                                                    this.anchor_for_pos(start, ev.position);
                                                this.popup =
                                                    Some(ActiveHighlightPopup::Selection {
                                                        anchor_pos,
                                                        note_mode: false,
                                                        selected_color: this
                                                            .prefs
                                                            .last_highlight_color,
                                                        selected_style: this
                                                            .prefs
                                                            .last_highlight_style,
                                                    });
                                                cx.notify();
                                                return;
                                            }
                                        }
                                    }
                                    this.pending_click_highlight_id =
                                        this.highlight_at_pos(hit).map(|h| h.id.clone());
                                    this.selection_anchor = Some(hit);
                                    this.selection_head = Some(hit);
                                    this.is_dragging_selection = true;
                                    this.drag_start_window_pos = Some(ev.position);
                                    this.popup = None;
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(
                                |this, ev: &MouseMoveEvent, _window, cx| {
                                    if !this.is_dragging_selection
                                        || ev.pressed_button != Some(MouseButton::Left)
                                    {
                                        return;
                                    }
                                    if let Some(hit) = this.hit_test_position(ev.position) {
                                        let moved_far = this.drag_start_window_pos.is_some_and(|p| {
                                            let dx = f32::from(ev.position.x - p.x);
                                            let dy = f32::from(ev.position.y - p.y);
                                            dx * dx + dy * dy > 16.0
                                        });
                                        if moved_far || this.selection_anchor != Some(hit) {
                                            this.pending_click_highlight_id = None;
                                        }
                                        if this.selection_head != Some(hit) {
                                            this.selection_head = Some(hit);
                                            cx.notify();
                                        }
                                    }
                                },
                            ))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, ev: &MouseUpEvent, window, cx| {
                                    if !this.is_dragging_selection {
                                        return;
                                    }
                                    this.is_dragging_selection = false;
                                    if let Some((start, _end)) = this.normalized_selection() {
                                        let anchor_pos = this.anchor_for_pos(start, ev.position);
                                        this.popup = Some(ActiveHighlightPopup::Selection {
                                            anchor_pos,
                                            note_mode: false,
                                            selected_color: this.prefs.last_highlight_color,
                                            selected_style: this.prefs.last_highlight_style,
                                        });
                                    } else if let Some(hl_id) =
                                        this.pending_click_highlight_id.take()
                                    {
                                        this.open_highlight_inspector(
                                            &hl_id,
                                            ev.position,
                                            window,
                                            cx,
                                        );
                                    } else {
                                        this.clear_selection_and_popup();
                                    }
                                    cx.notify();
                                }),
                            )
                            .on_mouse_up_out(
                                MouseButton::Left,
                                cx.listener(|this, ev: &MouseUpEvent, _window, cx| {
                                    if !this.is_dragging_selection {
                                        return;
                                    }
                                    this.is_dragging_selection = false;
                                    if let Some((start, _end)) = this.normalized_selection() {
                                        let anchor_pos = this.anchor_for_pos(start, ev.position);
                                        this.popup = Some(ActiveHighlightPopup::Selection {
                                            anchor_pos,
                                            note_mode: false,
                                            selected_color: this.prefs.last_highlight_color,
                                            selected_style: this.prefs.last_highlight_style,
                                        });
                                    } else {
                                        this.clear_selection_and_popup();
                                    }
                                    cx.notify();
                                }),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .w_full()
                                    .min_h_full()
                                    .py_6()
                                    .px_6()
                                    .flex()
                                    .justify_center()
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .when_some(self.prefs.width.max_width_px_opt(), |d, w| {
                                                d.max_w(px(w))
                                            })
                                    .font_family(reader_font)
                                    .p_8()
                                    .gap_4()
                                    .rounded(px(10.))
                                    .bg(page_bg)
                                    .border_1()
                                    .border_color(border_c)
                                    .child(
                                        h_flex()
                                            .pb_3()
                                            .border_b_1()
                                            .border_color(border_c)
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted_fg)
                                                    .truncate()
                                                    .child(author_line),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .italic()
                                                    .text_color(muted_fg)
                                                    .child(ch_title),
                                            ),
                                    )
                                    .child(body_element)
                                    .child(
                                        h_flex()
                                            .pt_5()
                                            .mt_4()
                                            .border_t_1()
                                            .border_color(border_c)
                                            .justify_between()
                                            .child(
                                                Button::new("epub-foot-prev")
                                                    .ghost()
                                                    .small()
                                                    .icon(IconName::ChevronLeft)
                                                    .label(prev_label)
                                                    .disabled(at_book_start)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.prev_page_or_chapter(cx);
                                                    })),
                                            )
                                            .child(footer_right),
                                    ),
                            ),
                                    ),
                            )
                            .when(!is_page_mode, |el| el.vertical_scrollbar(&self.scroll_handle))
                            .when(is_page_mode && page_mask_h > 0.5, |el| {
                                el.child(
                                    div()
                                        .absolute()
                                        .bottom_0()
                                        .left_0()
                                        .right_0()
                                        .h(px(page_mask_h))
                                        .bg(outer_bg),
                                )
                            })
                    )
                    .when_some(find_bar, |root, bar| root.child(bar))
                    .when_some(annotations_popover, |root, pop| root.child(pop))
                    .when_some(hl_popup, |root, pop| root.child(pop)),
            )
            .child(
                h_flex()
                    .h(px(26.))
                    .w_full()
                    .px_4()
                    .gap_3()
                    .items_center()
                    .bg(outer_bg)
                    .border_t_1()
                    .border_color(border_c)
                    .child(
                        div()
                            .flex_1()
                            .h(px(3.))
                            .rounded_full()
                            .bg(border_c)
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(overall_pct.clamp(0.0, 1.0)))
                                    .bg(if self.is_finished {
                                        hex_color(kd::GREEN)
                                    } else {
                                        hex_color(kd::VIOLET)
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child(footer_meta),
                    ),
            )
            .into_any_element()
    }
}

pub fn render_epub_tab(
    _app: &BookwormApp,
    tab: &BookTab,
    cx: &mut Context<BookwormApp>,
) -> impl IntoElement {
    let Some(view) = tab.view.as_ref() else {
        return div().into_any_element();
    };

    let file_size_bytes = &tab.file_size_bytes;
    let tab_id = tab.id;
    let rel_path = tab.rel_path.clone();
    let book_path = tab.path.clone();
    let kb = *file_size_bytes as f64 / 1024.0;
    let ev = view.read(cx);
    let summary = ev.summary();
    let theme_label = ev.prefs.theme.label();
    let font_label = ev.prefs.font.label();
    let width_label = ev.prefs.width.label();
    let page_mode = ev.prefs.pagination == ReaderPaginationPref::Page;
    let page_mode_label = ev.prefs.pagination.label();
    let spacing_label = ev.prefs.spacing.label();
    let ann_count = ev.highlights.len() + ev.bookmarks.len();
    let bm_open = ev.bookmarks_open;
    let cur_ch = ev.chapter_idx;
    let total_ch = ev.book.chapters.len();
    let find_open = ev.find_open;

    let v_find = view.clone();
    let v_prev = view.clone();
    let v_next = view.clone();
    let v_sm = view.clone();
    let v_lg = view.clone();
    let v_font = view.clone();
    let v_width = view.clone();
    let v_page = view.clone();
    let v_spacing = view.clone();
    let v_thm = view.clone();
    let v_bm = view.clone();
    let v_lib = view.clone();

    let header = h_flex()
        .h(px(36.))
        .w_full()
        .px_3()
        .gap_2()
        .items_center()
        .justify_between()
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_b_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(kd::FG_MUTED))
                        .truncate()
                        .child(rel_path),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(kd::VIOLET))
                        .truncate()
                        .child(format!("{summary} · {kb:.0} KB")),
                ),
        )
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child(
                    Button::new(SharedString::from(format!("epub-find-{tab_id}")))
                        .with_variant(if find_open {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .xsmall()
                        .icon(IconName::Search)
                        .tooltip("Find in Book (Ctrl+F)")
                        .on_click(cx.listener(move |_, _, window, cx| {
                            v_find.update(cx, |v, cx| v.open_find(window, cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-prev-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::ChevronLeft)
                        .disabled(cur_ch == 0)
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_prev.update(cx, |v, cx| v.prev_chapter(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-next-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::ChevronRight)
                        .disabled(cur_ch + 1 >= total_ch)
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_next.update(cx, |v, cx| v.next_chapter(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-sm-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .label("A-")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_sm.update(cx, |v, cx| {
                                let next = v.prefs.font_size - 1.0;
                                v.set_font_size(next, cx);
                            });
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-lg-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .label("A+")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_lg.update(cx, |v, cx| {
                                let next = v.prefs.font_size + 1.0;
                                v.set_font_size(next, cx);
                            });
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-font-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Type)
                        .label(font_label)
                        .tooltip("Switch Font Family (Serif / Sans)")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_font.update(cx, |v, cx| v.toggle_font(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-width-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Columns2)
                        .label(width_label)
                        .tooltip("Cycle Page Width / 2-Column Spread")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_width.update(cx, |v, cx| v.cycle_width(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-page-mode-{tab_id}")))
                        .with_variant(if page_mode {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .xsmall()
                        .icon(IconName::FileText)
                        .label(page_mode_label)
                        .tooltip("Toggle Paginated Page View vs Continuous Scroll")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_page.update(cx, |v, cx| v.toggle_pagination(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-space-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .label(spacing_label)
                        .tooltip("Cycle Line Spacing")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_spacing.update(cx, |v, cx| v.cycle_spacing(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-theme-{tab_id}")))
                        .secondary()
                        .xsmall()
                        .icon(IconName::BookOpen)
                        .label(theme_label)
                        .tooltip("Cycle Reader Theme (Dark / Sepia / Paper / AMOLED)")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            v_thm.update(cx, |v, cx| v.cycle_theme(cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-bm-{tab_id}")))
                        .with_variant(if bm_open {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .xsmall()
                        .icon(IconName::Highlighter)
                        .label(if ann_count > 0 {
                            format!("Notes ({ann_count})")
                        } else {
                            "Notes".to_string()
                        })
                        .tooltip("Highlights, Inline Notes & Bookmarks")
                        .on_click(cx.listener(move |_, _, window, cx| {
                            v_bm.update(cx, |v, cx| v.toggle_bookmarks(window, cx));
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("epub-tags-{tab_id}")))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Tag)
                        .label("Tags")
                        .tooltip("Edit Book Metadata & Cover in Library")
                        .on_click({
                            let path = book_path.clone();
                            cx.listener(move |_, _, _, cx| {
                                v_lib.update(cx, |_, cx| {
                                    cx.emit(EpubViewEvent::OpenInLibrary { path: path.clone() });
                                });
                            })
                        }),
                ),
        );

    v_flex()
        .size_full()
        .bg(hex_color(kd::BG_DEEP))
        .child(header)
        .child(div().flex_1().min_h_0().w_full().child(view.clone()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::EpubView;

    #[test]
    fn compute_block_scroll_target_y_handles_scrolled_and_unscrolled_coords() {
        // Viewport top at y=100, currently unscrolled (offset.y=0), block at win_y=900, margin=50 -> 750.
        let target = EpubView::compute_block_scroll_target_y(900.0, 100.0, 0.0, 2000.0, 50.0);
        assert!((target - 750.0).abs() < 1e-4);

        // Same block when already scrolled down 300px (offset.y=-300 => painted win_y=600) must yield identical 750.
        let target_scrolled =
            EpubView::compute_block_scroll_target_y(600.0, 100.0, -300.0, 2000.0, 50.0);
        assert!((target_scrolled - 750.0).abs() < 1e-4);

        // Jumping UP to an earlier block scrolled offscreen above viewport (offset.y=-1200, win_y=-800 -> content_y=300 -> 250).
        let target_up =
            EpubView::compute_block_scroll_target_y(-800.0, 100.0, -1200.0, 2000.0, 50.0);
        assert!((target_up - 250.0).abs() < 1e-4);

        // Clamps to [0, max_scroll_y] and returns 0 when non-scrollable.
        assert_eq!(
            EpubView::compute_block_scroll_target_y(120.0, 100.0, 0.0, 2000.0, 50.0),
            0.0
        );
        assert_eq!(
            EpubView::compute_block_scroll_target_y(3500.0, 100.0, 0.0, 2000.0, 50.0),
            2000.0
        );
        assert_eq!(
            EpubView::compute_block_scroll_target_y(500.0, 100.0, 0.0, 0.0, 50.0),
            0.0
        );
    }
}
