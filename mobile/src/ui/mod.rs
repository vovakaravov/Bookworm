pub mod image_util;
pub mod library_view;
pub mod reader_view;
pub mod settings_view;

use crate::config::{
    BookwormMobileConfig, LibraryGridSize, LibrarySort, app_cache_dir, app_storage_dir,
    books_root_candidates, default_books_root, has_external_storage_access, state_root_for,
};
use crate::theme::ReaderTheme;
use bookworm_core::dictionary::{
    DEFAULT_DICT_PACK_URL, DictionaryEntry, DictionaryStats, DictionaryStore,
    download_offline_pack, extract_lookup_word, lookup_online_and_cache,
};
use bookworm_core::epub::{EpubBlock, EpubBook, GlobalBookLocation, create_sample_epub};
use bookworm_core::export_md::export_library_notes_to_dir;
use bookworm_core::library::{
    BookBookmark, BookHighlight, BookRecord, ChapterPage, HighlightColor, HighlightDecoration,
    LibraryStore, MeasuredBlock, ReaderPaginationPref, ReaderThemePref, ReaderWidthPref,
    ReadingStatus, clamp_to_char_boundary, page_index_for_block, paginate_measured_blocks,
    scan_books_cached, word_bounds_at,
};
use bookworm_core::{can_read_dir, count_direct_matching_files, list_subdirectories};
use gpui::{prelude::*, *};
use gpui_kit::component::input::{InputEvent, InputState};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPos {
    pub block_idx: usize,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionHandleKind {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTouchMode {
    Pending,
    HorizontalSwipe,
    LongPressSelecting,
    HandleDrag(SelectionHandleKind),
}

pub fn extend_long_press_selection(
    anchor_word: (TextPos, TextPos),
    hit: TextPos,
) -> (TextPos, TextPos) {
    let (w_start, w_end) = anchor_word;
    if hit < w_start {
        (w_end, hit)
    } else if hit > w_end {
        (w_start, hit)
    } else {
        (w_start, w_end)
    }
}

pub fn compute_handle_drag_update(
    handle: SelectionHandleKind,
    cur_start: TextPos,
    cur_end: TextPos,
    hit: TextPos,
) -> Option<(TextPos, TextPos, SelectionHandleKind)> {
    match handle {
        SelectionHandleKind::Start => {
            if hit < cur_end {
                Some((hit, cur_end, SelectionHandleKind::Start))
            } else if hit > cur_end {
                Some((cur_end, hit, SelectionHandleKind::End))
            } else {
                None
            }
        }
        SelectionHandleKind::End => {
            if hit > cur_start {
                Some((cur_start, hit, SelectionHandleKind::End))
            } else if hit < cur_start {
                Some((hit, cur_start, SelectionHandleKind::Start))
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PageFlipAnimation {
    pub dir: i8,
    pub pending_step: isize,
    pub started_at: Instant,
    pub duration_ms: u64,
    pub start_x: f32,
    pub target_x: f32,
}

pub fn block_plain_text(blk: &EpubBlock) -> Option<&str> {
    match blk {
        EpubBlock::Heading { text, .. }
        | EpubBlock::Paragraph { text, .. }
        | EpubBlock::ListItem { text, .. } => Some(text.as_str()),
        EpubBlock::Image { .. } | EpubBlock::Rule => None,
    }
}

#[derive(Debug, Clone)]
pub enum DictLookupState {
    Loading(String),
    Found(Vec<DictionaryEntry>),
    NotFound(String),
}

#[derive(Debug, Clone)]
pub struct HighlightEditorDraft {
    pub start_block: usize,
    pub start_offset: usize,
    pub end_block: usize,
    pub end_offset: usize,
    pub quote: String,
    pub existing_id: Option<String>,
    pub color: HighlightColor,
    pub style: HighlightDecoration,
}

pub type ReaderLayoutKey = (u32, u32, u32, u8, bool);
pub type SliceRenderKey = (usize, usize, usize, usize, bool, u64);
pub type SliceRenderValue = (
    SharedString,
    Vec<(Range<usize>, HighlightStyle)>,
    Vec<(HighlightDecoration, Range<usize>, u32)>,
);

pub struct PendingChapterPagination {
    pub ch_idx: usize,
    pub layout_key: ReaderLayoutKey,
    pub next_block: usize,
    pub measured: Vec<MeasuredBlock>,
}

pub(crate) fn retain_adjacent_chapters<V>(cache: &mut HashMap<usize, V>, ch_idx: usize) {
    let min_keep = ch_idx.saturating_sub(2);
    let max_keep = ch_idx.saturating_add(2);
    cache.retain(|&k, _| (min_keep..=max_keep).contains(&k));
}

pub struct ReaderState {
    pub book_rel: String,
    pub book_title: String,
    pub book: EpubBook,
    pub chapter_cum_weights: Vec<f32>,
    pub chapter_idx: usize,
    pub show_settings: bool,
    pub show_toc: bool,
    pub show_notes: bool,
    pub zen_mode: bool,
    pub notes_color_filter: Option<HighlightColor>,
    pub notes_only_filter: bool,
    pub draft: Option<HighlightEditorDraft>,
    pub draft_dock_top: Cell<bool>,
    pub draft_dock_initialized: Cell<bool>,
    pub dict_lookup: Option<DictLookupState>,
    pub dict_lookup_gen: u64,
    pub draft_sheet_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    pub settings_strip_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    pub viewport_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub chapter_pages: Vec<ChapterPage>,
    pub adjacent_pages_cache: HashMap<usize, Vec<ChapterPage>>,
    pub pages_layout_key: Option<ReaderLayoutKey>,
    pub pages_chapter_idx: Option<usize>,
    pub pending_prepag: Option<PendingChapterPagination>,
    pub prepag_scheduled: bool,
    pub current_page: usize,
    pub pending_last_page: bool,
    pub needs_page_recompute: bool,
    pub selection_anchor: Option<TextPos>,
    pub selection_head: Option<TextPos>,
    pub is_long_press_selecting: bool,
    pub long_press_anchor_word: Option<(TextPos, TextPos)>,
    pub long_press_origin_pos: Option<Point<Pixels>>,
    pub long_press_dragged: bool,
    pub dragging_handle: Option<SelectionHandleKind>,
    pub handle_drag_offset_y: f32,
    #[allow(clippy::type_complexity)]
    pub sel_handle_positions:
        Rc<RefCell<Option<(Option<(Point<Pixels>, f32)>, Option<(Point<Pixels>, f32)>)>>>,
    pub mouse_down_pos: Option<Point<Pixels>>,
    pub is_mouse_selecting: bool,
    pub page_touch_mode: Option<PageTouchMode>,
    pub page_touch_gen: u64,
    pub page_touch_start_x: f32,
    pub page_touch_cur_pos: Point<Pixels>,
    pub page_touch_last_x: f32,
    pub page_touch_last_at: Option<Instant>,
    pub page_touch_vel_x: f32,
    pub _page_lp_task: Option<Task<()>>,
    pub swipe_offset_x: f32,
    pub swipe_active: bool,
    pub swipe_ignore_momentum: bool,
    pub flip_anim: Option<PageFlipAnimation>,
    pub flip_anim_gen: u64,
    /// `(block_idx, byte_base, slice_len, TextLayout)` for currently interactive page/viewport.
    pub rendered_layouts: Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>,
    pub highlights_epoch: u64,
    pub slice_render_cache: RefCell<HashMap<SliceRenderKey, SliceRenderValue>>,
    pub decoded_images: RefCell<HashMap<(usize, usize), Arc<Image>>>,
    pub pending_scroll_block: Option<(usize, usize)>,
    pub pending_scroll_fraction: Option<f32>,
    pub last_saved_scroll_pct: f32,
}

impl ReaderState {
    pub fn global_location(&self, scroll_pct: f32) -> GlobalBookLocation {
        EpubBook::global_location_from_weights(
            &self.chapter_cum_weights,
            self.chapter_idx,
            scroll_pct,
        )
    }

    pub fn normalized_selection(&self) -> Option<(TextPos, TextPos)> {
        let a = self.selection_anchor?;
        let b = self.selection_head?;
        if a == b {
            return None;
        }
        if a <= b { Some((a, b)) } else { Some((b, a)) }
    }

    pub fn block_selection_range(&self, blk_ix: usize, block_len: usize) -> Option<Range<usize>> {
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

    pub fn clear_selection_and_draft(&mut self) {
        self.selection_anchor = None;
        self.selection_head = None;
        self.is_long_press_selecting = false;
        self.long_press_anchor_word = None;
        self.long_press_origin_pos = None;
        self.long_press_dragged = false;
        self.dragging_handle = None;
        self.draft_dock_initialized.set(false);
        *self.sel_handle_positions.borrow_mut() = None;
        self.mouse_down_pos = None;
        self.is_mouse_selecting = false;
        self.draft = None;
        self.dict_lookup = None;
        self.dict_lookup_gen = self.dict_lookup_gen.wrapping_add(1);
    }

    pub fn plain_text(&self, blk_ix: usize) -> Option<&str> {
        self.book
            .chapters
            .get(self.chapter_idx)
            .and_then(|c| c.blocks.get(blk_ix))
            .and_then(block_plain_text)
    }

    pub fn extract_text_for_range(&self, start: TextPos, end: TextPos) -> String {
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

    pub fn hit_test_position(
        &self,
        pos: Point<Pixels>,
        clip_bounds: Option<Bounds<Pixels>>,
    ) -> Option<TextPos> {
        let layouts = self.rendered_layouts.borrow();
        if layouts.is_empty() {
            return None;
        }
        let in_clip = |b: Bounds<Pixels>| {
            clip_bounds.is_none_or(|c| {
                b.bottom() >= c.top() - px(12.) && b.top() <= c.bottom() + px(12.)
            })
        };

        let mut chosen: Option<&(usize, usize, usize, TextLayout)> = None;
        for entry in layouts.iter() {
            let b = entry.3.bounds();
            if !in_clip(b) {
                continue;
            }
            if b.top() <= pos.y
                && pos.y <= b.bottom()
                && b.left() - px(18.) <= pos.x
                && pos.x <= b.right() + px(18.)
            {
                chosen = Some(entry);
                break;
            }
        }

        if chosen.is_none() {
            let mut best_score = f32::MAX;
            for entry in layouts.iter() {
                let b = entry.3.bounds();
                if !in_clip(b) {
                    continue;
                }
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
                let score = dy + dx * 4.0;
                if score < best_score {
                    best_score = score;
                    chosen = Some(entry);
                }
            }
        }

        let (blk_ix, byte_base, slice_len, layout) = chosen?;
        let rel_ix = match layout.index_for_position(pos) {
            Ok(ix) | Err(ix) => ix,
        }
        .min(*slice_len);
        let safe_ix = if let Some(txt) = self.plain_text(*blk_ix) {
            clamp_to_char_boundary(txt, byte_base.saturating_add(rel_ix))
        } else {
            0
        };
        Some(TextPos {
            block_idx: *blk_ix,
            byte_offset: safe_ix,
        })
    }

    pub fn hit_test_strict(
        &self,
        pos: Point<Pixels>,
        clip_bounds: Option<Bounds<Pixels>>,
    ) -> Option<TextPos> {
        let layouts = self.rendered_layouts.borrow();
        if layouts.is_empty() {
            return None;
        }
        let in_clip = |b: Bounds<Pixels>| {
            clip_bounds.is_none_or(|c| {
                b.bottom() >= c.top() - px(12.) && b.top() <= c.bottom() + px(12.)
            })
        };

        for (blk_ix, byte_base, slice_len, layout) in layouts.iter() {
            let b = layout.bounds();
            if !in_clip(b) {
                continue;
            }
            if b.top() - px(4.) <= pos.y
                && pos.y <= b.bottom() + px(4.)
                && b.left() - px(6.) <= pos.x
                && pos.x <= b.right() + px(6.)
                && let Ok(rel_ix) = layout.index_for_position(pos)
            {
                let capped = rel_ix.min(*slice_len);
                let safe_ix = if let Some(txt) = self.plain_text(*blk_ix) {
                    clamp_to_char_boundary(txt, byte_base.saturating_add(capped))
                } else {
                    0
                };
                return Some(TextPos {
                    block_idx: *blk_ix,
                    byte_offset: safe_ix,
                });
            }
        }
        None
    }

    pub fn highlight_at_pos<'a>(
        &self,
        highlights: &'a [BookHighlight],
        pos: TextPos,
    ) -> Option<&'a BookHighlight> {
        highlights.iter().rev().find(|hl| {
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

    pub fn selection_handle_geometries(
        &self,
        clip_bounds: Bounds<Pixels>,
    ) -> (Option<(Point<Pixels>, f32)>, Option<(Point<Pixels>, f32)>) {
        let Some((start, end)) = self.normalized_selection() else {
            return (None, None);
        };
        let layouts = self.rendered_layouts.borrow();
        let find_pt = |tp: TextPos| -> Option<(Point<Pixels>, f32)> {
            let entry = layouts
                .iter()
                .find(|(b, base, slen, _)| {
                    *b == tp.block_idx
                        && tp.byte_offset >= *base
                        && tp.byte_offset <= base.saturating_add(*slen)
                })
                .or_else(|| layouts.iter().find(|(b, _, _, _)| *b == tp.block_idx))?;
            let (_, base, slen, layout) = entry;
            let rel_off = tp.byte_offset.saturating_sub(*base).min(*slen);
            let pt = layout.position_for_index(rel_off)?;
            let lh = f32::from(layout.line_height()).max(14.0);
            if pt.y + px(lh) < clip_bounds.top() - px(16.) || pt.y > clip_bounds.bottom() + px(16.) {
                return None;
            }
            Some((pt, lh))
        };
        (find_pt(start), find_pt(end))
    }

    pub fn select_word_at_point(
        &mut self,
        pos: Point<Pixels>,
        clip_bounds: Option<Bounds<Pixels>>,
    ) -> bool {
        let Some(hit) = self.hit_test_strict(pos, clip_bounds) else {
            return false;
        };
        let Some(txt) = self.plain_text(hit.block_idx) else {
            return false;
        };
        if txt.is_empty() {
            return false;
        }
        let r = word_bounds_at(txt, hit.byte_offset);
        let (a, b) = if r.start < r.end {
            (r.start, r.end)
        } else {
            (
                hit.byte_offset,
                (hit.byte_offset + 1).min(txt.len()),
            )
        };
        self.selection_anchor = Some(TextPos {
            block_idx: hit.block_idx,
            byte_offset: a,
        });
        self.selection_head = Some(TextPos {
            block_idx: hit.block_idx,
            byte_offset: b,
        });
        self.draft_dock_initialized.set(false);
        self.draft = None;
        true
    }

    pub fn update_handle_drag_at(
        &mut self,
        handle: SelectionHandleKind,
        pos: Point<Pixels>,
        clip_bounds: Option<Bounds<Pixels>>,
        last_color: HighlightColor,
        last_style: HighlightDecoration,
    ) -> bool {
        let Some(hit) = self.hit_test_position(pos, clip_bounds) else {
            return false;
        };
        let Some((cur_start, cur_end)) = self.normalized_selection() else {
            return false;
        };
        let Some((new_anchor, new_head, new_handle)) =
            compute_handle_drag_update(handle, cur_start, cur_end, hit)
        else {
            return false;
        };
        self.dragging_handle = Some(new_handle);
        self.page_touch_mode = Some(PageTouchMode::HandleDrag(new_handle));
        if self.selection_anchor == Some(new_anchor) && self.selection_head == Some(new_head) {
            return false;
        }
        self.selection_anchor = Some(new_anchor);
        self.selection_head = Some(new_head);
        self.sync_draft_from_selection(last_color, last_style);
        true
    }

    pub fn sync_draft_from_selection(
        &mut self,
        last_color: HighlightColor,
        last_style: HighlightDecoration,
    ) {
        let Some((start, end)) = self.normalized_selection() else {
            return;
        };
        let quote = self.extract_text_for_range(start, end);
        if quote.trim().is_empty() {
            return;
        }
        if let Some(d) = &mut self.draft {
            d.start_block = start.block_idx;
            d.start_offset = start.byte_offset;
            d.end_block = end.block_idx;
            d.end_offset = end.byte_offset;
            d.quote = quote;
        } else {
            self.draft = Some(HighlightEditorDraft {
                start_block: start.block_idx,
                start_offset: start.byte_offset,
                end_block: end.block_idx,
                end_offset: end.byte_offset,
                quote,
                existing_id: None,
                color: last_color,
                style: last_style,
            });
        }
    }

    pub fn visual_swipe_offset_x(&self) -> f32 {
        let anim_x = if let Some(anim) = &self.flip_anim {
            let elapsed_ms = anim.started_at.elapsed().as_secs_f32() * 1000.0;
            let raw_t = (elapsed_ms / (anim.duration_ms.max(1) as f32)).clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - raw_t).powi(4);
            anim.start_x + (anim.target_x - anim.start_x) * eased
        } else {
            0.0
        };
        self.swipe_offset_x + anim_x
    }
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
    pub library_sort: LibrarySort,
    pub library_shelf_filter: Option<String>,
    pub library_drawer_open: bool,
    pub reader: Option<ReaderState>,
    pub last_opened_rel: Option<String>,
    pub folder_picker: Option<FolderPickerState>,
    pub book_context_menu: Option<String>,
    pub opening_book_title: Option<String>,

    pub reader_theme: ReaderTheme,
    pub reader_font_size: f32,
    pub reader_width: ReaderWidthPref,
    pub two_column_spread: bool,
    pub library_grid_size: LibraryGridSize,
    pub status_banner: Option<String>,
    pub dict_dir: PathBuf,
    pub dict_store: Rc<RefCell<DictionaryStore>>,
    pub dict_stats: DictionaryStats,
    pub dict_downloading: bool,

    pub library_search_input: Entity<InputState>,
    pub highlight_note_input: Entity<InputState>,
    pub folder_picker_input: Entity<InputState>,

    pub library_scroll: ScrollHandle,
    pub library_rows_scroll: UniformListScrollHandle,
    pub library_drawer_scroll: ScrollHandle,
    pub toc_scroll: ScrollHandle,
    pub notes_scroll: ScrollHandle,
    pub settings_scroll: ScrollHandle,
    pub folder_picker_scroll: ScrollHandle,

    last_back_at: Option<Instant>,
    last_resume_epoch: u64,
    open_book_gen: u64,
    sync_save_gen: u64,
    _library_task: Option<Task<()>>,
    _open_book_task: Option<Task<()>>,
    _sync_save_task: Option<Task<()>>,
    _perm_poll_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

pub fn shape_block_line_ranges(
    text: &str,
    font_size: f32,
    wrap_w: f32,
    base_style: &TextStyle,
    all_bold: bool,
    runs: &[bookworm_core::epub::EpubRun],
    window: &Window,
) -> Vec<Range<usize>> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut pts = Vec::with_capacity(runs.len() * 2 + 2);
    pts.push(0usize);
    pts.push(text.len());
    for r in runs {
        let a = clamp_to_char_boundary(text, r.range.start);
        let b = clamp_to_char_boundary(text, r.range.end);
        if a < b {
            pts.push(a);
            pts.push(b);
        }
    }
    pts.sort_unstable();
    pts.dedup();

    let mut text_runs = Vec::with_capacity(pts.len().saturating_sub(1).max(1));
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }
        let matching = runs.iter().find(|r| {
            let rs = clamp_to_char_boundary(text, r.range.start);
            let re = clamp_to_char_boundary(text, r.range.end);
            rs <= a && re >= b
        });
        let mut style = base_style.clone();
        let bold = all_bold || matching.is_some_and(|r| r.bold);
        let italic = matching.is_some_and(|r| r.italic);
        if bold {
            style.font_weight = FontWeight::BOLD;
        }
        if italic {
            style.font_style = FontStyle::Italic;
        }
        text_runs.push(style.to_run(b - a));
    }

    let shaped = match window.text_system().shape_text(
        SharedString::from(text.to_string()),
        px(font_size),
        &text_runs,
        Some(px(wrap_w.max(60.0))),
        None,
    ) {
        Ok(lines) => lines,
        Err(_) => return vec![0..text.len()],
    };

    let mut ranges = Vec::new();
    let mut chunk_start = 0usize;
    for (chunk_str, wrapped) in text.split('\n').zip(shaped.iter()) {
        let chunk_len = chunk_str.len();
        let mut prev = 0usize;
        for wb in wrapped.wrap_boundaries().iter() {
            let split_local = wrapped
                .unwrapped_layout
                .runs
                .get(wb.run_ix)
                .and_then(|r| r.glyphs.get(wb.glyph_ix))
                .map(|g| g.index.min(chunk_len))
                .unwrap_or(prev);
            if split_local > prev {
                let s = clamp_to_char_boundary(text, chunk_start + prev);
                let e = clamp_to_char_boundary(text, chunk_start + split_local);
                if s < e {
                    ranges.push(s..e);
                }
                prev = split_local;
            }
        }
        if prev < chunk_len || ranges.is_empty() {
            let s = clamp_to_char_boundary(text, chunk_start + prev);
            let e = clamp_to_char_boundary(text, chunk_start + chunk_len);
            if s < e || chunk_len == 0 {
                ranges.push(s..e);
            }
        }
        chunk_start = (chunk_start + chunk_len + 1).min(text.len());
    }
    if ranges.is_empty() {
        ranges.push(0..text.len());
    }
    ranges
}

pub fn measure_chapter_block(
    b_ix: usize,
    blk: &EpubBlock,
    col_wrap_base_w: f32,
    page_content_h: f32,
    font_size: f32,
    base_style: &TextStyle,
    window: &Window,
) -> MeasuredBlock {
    match blk {
        EpubBlock::Heading { level, text } => {
            let size = match level {
                1 => (font_size + 7.0).min(34.0),
                2 => (font_size + 4.5).min(30.0),
                _ => (font_size + 2.0).min(26.0),
            };
            let lh = (size * 1.28).ceil();
            let line_ranges = shape_block_line_ranges(
                text,
                size,
                col_wrap_base_w,
                base_style,
                true,
                &[],
                window,
            );
            MeasuredBlock::Text {
                block_idx: b_ix,
                line_ranges,
                line_height: lh,
                pad_top: 8.0,
                pad_bottom: 4.0,
                is_heading: true,
            }
        }
        EpubBlock::Paragraph {
            text,
            runs,
            is_quote,
        } => {
            let lh = (font_size * 1.62).ceil();
            let h_pad = if *is_quote { 20.0 } else { 12.0 };
            let line_ranges = shape_block_line_ranges(
                text,
                font_size,
                (col_wrap_base_w - h_pad).max(60.0),
                base_style,
                false,
                runs,
                window,
            );
            MeasuredBlock::Text {
                block_idx: b_ix,
                line_ranges,
                line_height: lh,
                pad_top: 4.0,
                pad_bottom: 4.0,
                is_heading: false,
            }
        }
        EpubBlock::ListItem { text, runs } => {
            let lh = (font_size * 1.58).ceil();
            let line_ranges = shape_block_line_ranges(
                text,
                font_size,
                (col_wrap_base_w - 28.0).max(60.0),
                base_style,
                false,
                runs,
                window,
            );
            MeasuredBlock::Text {
                block_idx: b_ix,
                line_ranges,
                line_height: lh,
                pad_top: 2.0,
                pad_bottom: 2.0,
                is_heading: false,
            }
        }
        EpubBlock::Image { .. } => MeasuredBlock::Fixed {
            block_idx: b_ix,
            height: 220.0_f32.min((page_content_h * 0.68).max(120.0)) + 16.0,
        },
        EpubBlock::Rule => MeasuredBlock::Fixed {
            block_idx: b_ix,
            height: 17.0,
        },
    }
}

pub fn paginate_chapter_blocks_for_view(
    blocks: &[EpubBlock],
    col_wrap_base_w: f32,
    page_content_h: f32,
    font_size: f32,
    window: &Window,
) -> Vec<ChapterPage> {
    let base_style = window.text_style();
    let mut measured = Vec::with_capacity(blocks.len());
    for (b_ix, blk) in blocks.iter().enumerate() {
        measured.push(measure_chapter_block(
            b_ix,
            blk,
            col_wrap_base_w,
            page_content_h,
            font_size,
            &base_style,
            window,
        ));
    }
    paginate_measured_blocks(&measured, page_content_h, 0.0, 14.0)
}

pub fn reader_column_metrics(
    vp_w: f32,
    vp_h: f32,
    width_pref: ReaderWidthPref,
    two_col: bool,
) -> (Option<f32>, f32, f32) {
    let col_max_w = if two_col {
        Some(1140.0)
    } else {
        width_pref.max_width_px_opt()
    };
    let outer_w = vp_w.max(260.0);
    let inner_w = col_max_w.map_or(outer_w, |m| outer_w.min(m)).max(200.0);
    let col_wrap_base_w = if two_col {
        ((inner_w - 48.0 - 49.0) * 0.5).max(120.0)
    } else {
        (inner_w - 40.0).max(120.0)
    };
    let page_content_h = (vp_h - 32.0).max(120.0);
    (col_max_w, col_wrap_base_w, page_content_h)
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
        let (mut library_store, initial_conflicts) =
            LibraryStore::load_with_conflict_report(&library_state_root);

        let reader_theme = match library_store.reader_prefs.theme {
            ReaderThemePref::Dark => ReaderTheme::DragonDark,
            ReaderThemePref::Amoled => ReaderTheme::Amoled,
            ReaderThemePref::Sepia => ReaderTheme::Sepia,
            ReaderThemePref::Paper => ReaderTheme::Paper,
        };
        let reader_font_size = library_store.reader_prefs.font_size.clamp(13.0, 28.0);
        let reader_width = library_store.reader_prefs.width;
        library_store.reader_prefs.pagination = ReaderPaginationPref::Page;

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

        let dict_dir = app_storage_dir("dictionaries");
        let _ = DictionaryStore::maybe_import_from(
            &books_root.join("Dictionaries").join("en-ru.db"),
            &dict_dir,
        );
        let dict_store = DictionaryStore::open(dict_dir.clone());
        let dict_stats = dict_store.stats();
        let dict_store = Rc::new(RefCell::new(dict_store));

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
            library_sort: cfg.library_sort,
            library_shelf_filter: None,
            library_drawer_open: false,
            reader: None,
            last_opened_rel: None,
            folder_picker: None,
            book_context_menu: None,
            opening_book_title: None,
            reader_theme,
            reader_font_size,
            reader_width,
            two_column_spread: cfg.two_column_spread,
            library_grid_size: cfg.library_grid_size,
            status_banner: initial_banner,
            dict_dir,
            dict_store,
            dict_stats,
            dict_downloading: false,
            library_search_input,
            highlight_note_input,
            folder_picker_input,
            library_scroll: ScrollHandle::new(),
            library_rows_scroll: UniformListScrollHandle::new(),
            library_drawer_scroll: ScrollHandle::new(),
            toc_scroll: ScrollHandle::new(),
            notes_scroll: ScrollHandle::new(),
            settings_scroll: ScrollHandle::new(),
            folder_picker_scroll: ScrollHandle::new(),
            last_back_at: None,
            last_resume_epoch: crate::android_resume_epoch(),
            open_book_gen: 0,
            sync_save_gen: 0,
            _library_task: None,
            _open_book_task: None,
            _sync_save_task: None,
            _perm_poll_task: None,
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
            library_grid_size: self.library_grid_size,
            library_sort: self.library_sort,
        };
        cx.background_executor()
            .spawn(async move {
                let _ = cfg.save();
            })
            .detach();
    }

    pub fn set_library_grid_size(&mut self, size: LibraryGridSize, cx: &mut Context<Self>) {
        if self.library_grid_size != size {
            self.library_grid_size = size;
            self.persist_mobile_config(cx);
            cx.notify();
        }
    }

    pub fn set_library_sort(&mut self, sort: LibrarySort, cx: &mut Context<Self>) {
        if self.library_sort != sort {
            self.library_sort = sort;
            self.persist_mobile_config(cx);
            cx.notify();
        }
    }

    pub fn set_library_shelf_filter(&mut self, shelf: Option<String>, cx: &mut Context<Self>) {
        if self.library_shelf_filter != shelf {
            self.library_shelf_filter = shelf;
            cx.notify();
        }
    }

    pub fn toggle_library_drawer(
        &mut self,
        open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.library_drawer_open != open {
            if open {
                window.blur(cx);
                gpui_mobile::hide_keyboard();
                self.library_drawer_scroll.set_offset(point(px(0.), px(0.)));
            }
            self.library_drawer_open = open;
            cx.notify();
        }
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
        let was_granted = self.has_storage_access;
        let access = has_external_storage_access();
        self.has_storage_access = access;
        let fallback_books = app_storage_dir("books");
        let mut root_changed = false;
        if access && (!can_read_dir(&self.books_root) || self.books_root == fallback_books) {
            let detected = default_books_root();
            self.books_root = detected.clone();
            self.library_state_root = state_root_for(&detected);
            root_changed = true;
        }
        if access && (!was_granted || root_changed) {
            self.library_store = LibraryStore::load(&self.library_state_root);
            self.status_banner =
                Some("All files access granted — scanning library…".to_string());
        }
        self.books_root_candidates = books_root_candidates(&self.books_root);
        self.persist_mobile_config(cx);
        self.refresh_library_async(cx);
    }

    pub fn request_storage_permission(&mut self, cx: &mut Context<Self>) {
        let start_epoch = crate::android_resume_epoch();
        crate::request_all_files_access();
        self.status_banner = Some(
            "Toggle \"Allow access to manage all files\", then press Back.".to_string(),
        );
        cx.notify();

        self._perm_poll_task = Some(cx.spawn(async move |weak_self, cx| {
            for _ in 0..340u32 {
                cx.background_executor()
                    .timer(Duration::from_millis(350))
                    .await;
                if crate::android_resume_epoch() != start_epoch || crate::has_all_files_access() {
                    let _ = weak_self.update(cx, |this, cx| {
                        this.last_resume_epoch = crate::android_resume_epoch();
                        this.recheck_storage_and_refresh(cx);
                    });
                    break;
                }
            }
        }));
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

    pub fn open_book_context_menu(
        &mut self,
        rel_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        self.book_context_menu = Some(rel_path);
        cx.notify();
    }

    pub fn close_book_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.book_context_menu.take().is_some() {
            cx.notify();
        }
    }

    pub fn set_book_status(
        &mut self,
        rel_path: &str,
        status: ReadingStatus,
        cx: &mut Context<Self>,
    ) {
        let entry = self
            .library_store
            .books
            .entry(rel_path.to_string())
            .or_default();
        entry.status = status;
        entry.updated_at = Some(chrono::Utc::now().to_rfc3339());
        let updated = entry.clone();
        if let Some(book_rec) = self.books.iter_mut().find(|b| b.rel_path == rel_path) {
            book_rec.user = updated;
        }
        self.schedule_library_sync_save(true, cx);
        self.status_banner = Some(format!("Status set to {}", status.label()));
        cx.notify();
    }

    pub fn open_book_highlights(
        &mut self,
        rel_path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.book_context_menu = None;
        self.open_book_with_notes(rel_path, true, window, cx);
    }

    pub fn schedule_library_sync_save(&mut self, immediate: bool, cx: &mut Context<Self>) {
        self.sync_save_gen = self.sync_save_gen.wrapping_add(1);
        let seq = self.sync_save_gen;
        let delay = if immediate {
            Duration::from_millis(40)
        } else {
            Duration::from_millis(2500)
        };
        let state_root = self.library_state_root.clone();
        self._sync_save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let snapshot = this
                .update(cx, |app, _| {
                    if app.sync_save_gen != seq {
                        return None;
                    }
                    Some(app.library_store.clone())
                })
                .ok()
                .flatten();
            let Some(mut store) = snapshot else {
                return;
            };
            let res = cx
                .background_executor()
                .spawn(async move {
                    let count = store.sync_save(&state_root)?;
                    Ok::<(LibraryStore, usize), anyhow::Error>((store, count))
                })
                .await;
            if let Ok((merged_store, conflicts)) = res {
                let _ = this.update(cx, |app, cx| {
                    if conflicts > 0 && app.sync_save_gen == seq {
                        app.library_store.merge_from(merged_store);
                        app.status_banner =
                            Some(format!("Merged {conflicts} synced library conflict(s)"));
                        cx.notify();
                    }
                });
            }
        }));
    }

    pub fn export_single_book_notes(&mut self, rel_path: &str, cx: &mut Context<Self>) {
        let Some(book) = self.books.iter().find(|b| b.rel_path == rel_path).cloned() else {
            return;
        };
        let out_dir = self.books_root.join("Bookworm Notes");
        match export_library_notes_to_dir(std::slice::from_ref(&book), &out_dir) {
            Ok(0) => {
                self.status_banner = Some("No highlights or bookmarks to export for this book.".to_string());
            }
            Ok(n) => {
                self.status_banner = Some(format!(
                    "Exported {n} Markdown file to {}",
                    out_dir.display()
                ));
            }
            Err(err) => {
                self.status_banner = Some(format!("Export failed: {err}"));
            }
        }
        self.book_context_menu = None;
        cx.notify();
    }

    pub fn open_surface_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        self.surface = AppSurface::Library;
        cx.notify();
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
        if self.book_context_menu.is_some() {
            self.close_book_context_menu(cx);
            return;
        }

        match self.surface {
            AppSurface::Reader => {
                if let Some(r) = &mut self.reader {
                    if r.draft.is_some() || r.normalized_selection().is_some() {
                        r.clear_selection_and_draft();
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
                    if r.zen_mode {
                        r.zen_mode = false;
                        r.needs_page_recompute = true;
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
                if self.library_drawer_open {
                    self.library_drawer_open = false;
                    cx.notify();
                    return;
                }
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

    pub fn reader_current_scroll_pct(&self) -> f32 {
        let Some(r) = &self.reader else {
            return 0.0;
        };
        let tp = r.chapter_pages.len().max(1);
        if tp <= 1 {
            return if r.chapter_idx + 1 >= r.book.chapters.len() {
                1.0
            } else {
                0.0
            };
        }
        let two_col =
            self.two_column_spread && f32::from(r.viewport_bounds.get().size.width) >= 700.0;
        if two_col && r.current_page + 2 >= tp {
            return 1.0;
        }
        (r.current_page.min(tp - 1) as f32 / (tp - 1) as f32).clamp(0.0, 1.0)
    }

    pub fn ensure_reader_pages_and_sync(
        &mut self,
        vp_w: f32,
        vp_h: f32,
        two_col: bool,
        window: &Window,
    ) {
        if vp_w < 100.0 || vp_h < 100.0 {
            return;
        }
        let width_pref = self.reader_width;
        let font_size = self.reader_font_size;
        let (_, col_wrap_base_w, page_content_h) =
            reader_column_metrics(vp_w, vp_h, width_pref, two_col);

        let Some(r) = &mut self.reader else {
            return;
        };
        let ch_idx = r.chapter_idx.min(r.book.chapters.len().saturating_sub(1));
        let layout_key: ReaderLayoutKey = (
            vp_w.round() as u32,
            vp_h.round() as u32,
            (font_size * 10.0).round() as u32,
            width_pref as u8,
            two_col,
        );
        let same_layout = r.pages_layout_key == Some(layout_key);
        let layout_ready_for_ch =
            same_layout && r.pages_chapter_idx == Some(ch_idx) && !r.needs_page_recompute;

        if r.pages_layout_key.is_some()
            && layout_ready_for_ch
            && (r.draft.is_some()
                || r.is_long_press_selecting
                || r.dragging_handle.is_some()
                || r.page_touch_mode.is_some()
                || r.flip_anim.is_some()
                || r.swipe_active)
        {
            return;
        }
        if r.pages_layout_key.is_some()
            && (r.draft.is_some()
                || r.is_long_press_selecting
                || r.dragging_handle.is_some()
                || r.page_touch_mode.is_some())
        {
            return;
        }

        if !same_layout {
            r.adjacent_pages_cache.clear();
            r.pending_prepag = None;
            r.prepag_scheduled = false;
            r.slice_render_cache.borrow_mut().clear();
        }

        let rebuilt = r.needs_page_recompute || !same_layout || r.pages_chapter_idx != Some(ch_idx);
        let prev_anchor = if rebuilt
            && same_layout
            && r.pages_chapter_idx == Some(ch_idx)
            && r.pending_scroll_block.is_none()
            && r.pending_scroll_fraction.is_none()
            && !r.pending_last_page
        {
            r.chapter_pages
                .get(r.current_page)
                .and_then(|p| p.slices.first())
                .map(|s| (s.block_idx, s.byte_range.start))
        } else {
            None
        };
        if rebuilt {
            if same_layout
                && let Some(old_ch) = r.pages_chapter_idx
                && old_ch != ch_idx
                && !r.chapter_pages.is_empty()
            {
                r.adjacent_pages_cache
                    .insert(old_ch, std::mem::take(&mut r.chapter_pages));
            }
            let pages = if same_layout
                && let Some(cached) = r.adjacent_pages_cache.remove(&ch_idx)
            {
                cached
            } else if let Some(mut pending) = r
                .pending_prepag
                .take()
                .filter(|p| p.ch_idx == ch_idx && p.layout_key == layout_key)
            {
                if let Some(ch) = r.book.chapters.get(ch_idx) {
                    let base_style = window.text_style();
                    for b_ix in pending.next_block..ch.blocks.len() {
                        if let Some(blk) = ch.blocks.get(b_ix) {
                            pending.measured.push(measure_chapter_block(
                                b_ix,
                                blk,
                                col_wrap_base_w,
                                page_content_h,
                                font_size,
                                &base_style,
                                window,
                            ));
                        }
                    }
                    paginate_measured_blocks(&pending.measured, page_content_h, 0.0, 14.0)
                } else {
                    vec![ChapterPage {
                        slices: Vec::new(),
                        show_chapter_header: false,
                    }]
                }
            } else if let Some(ch) = r.book.chapters.get(ch_idx) {
                paginate_chapter_blocks_for_view(
                    &ch.blocks,
                    col_wrap_base_w,
                    page_content_h,
                    font_size,
                    window,
                )
            } else {
                vec![ChapterPage {
                    slices: Vec::new(),
                    show_chapter_header: false,
                }]
            };
            r.chapter_pages = pages;
            r.pages_layout_key = Some(layout_key);
            r.pages_chapter_idx = Some(ch_idx);
            r.needs_page_recompute = false;
        }

        let step = if two_col { 2usize } else { 1usize };
        let count = r.chapter_pages.len().max(1);
        let align_page = |p: usize| -> usize {
            let clamped = p.min(count - 1);
            if step == 2 { clamped - (clamped % 2) } else { clamped }
        };

        if r.pending_last_page {
            r.pending_last_page = false;
            r.pending_scroll_block = None;
            r.pending_scroll_fraction = None;
            r.current_page = align_page(count - 1);
        } else if let Some((blk, byte_off)) = r.pending_scroll_block.take() {
            r.pending_scroll_fraction = None;
            r.current_page = align_page(page_index_for_block(&r.chapter_pages, blk, byte_off));
        } else if let Some(frac) = r.pending_scroll_fraction.take() {
            let target = (frac.clamp(0.0, 1.0) * (count.saturating_sub(1) as f32)).round() as usize;
            r.current_page = align_page(target);
        } else if let Some((blk, byte_off)) = prev_anchor {
            r.current_page = align_page(page_index_for_block(&r.chapter_pages, blk, byte_off));
        } else {
            r.current_page = align_page(r.current_page);
        }

        retain_adjacent_chapters(&mut r.adjacent_pages_cache, ch_idx);
    }

    pub fn maybe_schedule_adjacent_prepag(
        &mut self,
        vp_w: f32,
        vp_h: f32,
        two_col: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if vp_w < 100.0 || vp_h < 100.0 {
            return;
        }
        let width_pref = self.reader_width;
        let font_size = self.reader_font_size;
        let layout_key: ReaderLayoutKey = (
            vp_w.round() as u32,
            vp_h.round() as u32,
            (font_size * 10.0).round() as u32,
            width_pref as u8,
            two_col,
        );
        let Some(r) = &mut self.reader else {
            return;
        };
        if r.flip_anim.is_some()
            || r.swipe_active
            || r.page_touch_mode.is_some()
            || r.show_toc
            || r.show_notes
            || r.prepag_scheduled
            || r.pages_layout_key != Some(layout_key)
        {
            return;
        }
        let ch_idx = r.chapter_idx.min(r.book.chapters.len().saturating_sub(1));
        let count = r.chapter_pages.len().max(1);
        let step = if two_col { 2usize } else { 1usize };
        let near_end = r.current_page + step * 3 >= count;
        let near_start = r.current_page <= step * 2;
        let target_ch = if near_end
            && ch_idx + 1 < r.book.chapters.len()
            && !r.adjacent_pages_cache.contains_key(&(ch_idx + 1))
        {
            Some(ch_idx + 1)
        } else if near_start && ch_idx > 0 && !r.adjacent_pages_cache.contains_key(&(ch_idx - 1)) {
            Some(ch_idx - 1)
        } else {
            None
        };
        let Some(target) = target_ch else {
            return;
        };
        let same_pending = r
            .pending_prepag
            .as_ref()
            .is_some_and(|p| p.ch_idx == target && p.layout_key == layout_key);
        if !same_pending {
            let cap = r
                .book
                .chapters
                .get(target)
                .map(|c| c.blocks.len())
                .unwrap_or(0);
            r.pending_prepag = Some(PendingChapterPagination {
                ch_idx: target,
                layout_key,
                next_block: 0,
                measured: Vec::with_capacity(cap),
            });
        }
        r.prepag_scheduled = true;
        let entity = cx.entity().clone();
        window.on_next_frame(move |window, cx| {
            let _ = entity.update(cx, |this, cx| {
                this.step_adjacent_prepag(window, cx);
            });
        });
    }

    pub fn step_adjacent_prepag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let font_size = self.reader_font_size;
        let width_pref = self.reader_width;
        let Some(r) = &mut self.reader else {
            return;
        };
        r.prepag_scheduled = false;
        if r.flip_anim.is_some()
            || r.swipe_active
            || r.page_touch_mode.is_some()
            || r.show_toc
            || r.show_notes
        {
            return;
        }
        let Some(mut pending) = r.pending_prepag.take() else {
            return;
        };
        if r.pages_layout_key != Some(pending.layout_key) {
            return;
        }
        let (vp_w, vp_h, _, _, two_col) = pending.layout_key;
        let (_, col_wrap_base_w, page_content_h) =
            reader_column_metrics(vp_w as f32, vp_h as f32, width_pref, two_col);
        let Some(ch) = r.book.chapters.get(pending.ch_idx) else {
            return;
        };
        let base_style = window.text_style();
        let started = Instant::now();
        while pending.next_block < ch.blocks.len() {
            if let Some(blk) = ch.blocks.get(pending.next_block) {
                pending.measured.push(measure_chapter_block(
                    pending.next_block,
                    blk,
                    col_wrap_base_w,
                    page_content_h,
                    font_size,
                    &base_style,
                    window,
                ));
            }
            pending.next_block += 1;
            if started.elapsed() >= Duration::from_millis(5) {
                break;
            }
        }
        if pending.next_block >= ch.blocks.len() {
            let pages = paginate_measured_blocks(&pending.measured, page_content_h, 0.0, 14.0);
            r.adjacent_pages_cache.insert(pending.ch_idx, pages);
            retain_adjacent_chapters(&mut r.adjacent_pages_cache, r.chapter_idx);
        } else {
            r.pending_prepag = Some(pending);
            r.prepag_scheduled = true;
            let entity = cx.entity().clone();
            window.on_next_frame(move |window, cx| {
                let _ = entity.update(cx, |this, cx| {
                    this.step_adjacent_prepag(window, cx);
                });
            });
        }
    }

    fn apply_page_step(&mut self, delta_steps: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = &mut self.reader {
            if delta_steps > 0 {
                let next_p = r.current_page + (delta_steps as usize);
                let count = r.chapter_pages.len().max(1);
                if next_p < count {
                    r.current_page = next_p;
                    r.clear_selection_and_draft();
                } else if r.chapter_idx + 1 < r.book.chapters.len() {
                    let next_ch = r.chapter_idx + 1;
                    self.reader_go_chapter(next_ch, window, cx);
                    return;
                }
            } else if delta_steps < 0 {
                let back = (-delta_steps) as usize;
                if r.current_page >= back {
                    r.current_page -= back;
                    r.clear_selection_and_draft();
                } else if r.current_page > 0 {
                    r.current_page = 0;
                    r.clear_selection_and_draft();
                } else if r.chapter_idx > 0 {
                    let prev_ch = r.chapter_idx - 1;
                    self.reader_go_chapter(prev_ch, window, cx);
                    if let Some(r) = &mut self.reader {
                        r.pending_last_page = true;
                        r.needs_page_recompute = true;
                    }
                    return;
                }
            }
        }
        let pct = self.reader_current_scroll_pct();
        self.persist_reader_progress(pct, cx);
    }

    pub fn flush_pending_page_flip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pending = self.reader.as_mut().and_then(|r| {
            let anim = r.flip_anim.take()?;
            r.flip_anim_gen = r.flip_anim_gen.wrapping_add(1);
            Some(anim.pending_step)
        });
        if let Some(step) = pending
            && step != 0
        {
            self.apply_page_step(step, window, cx);
        }
    }

    pub fn finish_page_flip_anim(
        &mut self,
        anim_gen: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let step = if let Some(r) = &mut self.reader {
            if r.flip_anim_gen != anim_gen {
                return;
            }
            let Some(anim) = r.flip_anim.take() else {
                return;
            };
            anim.pending_step
        } else {
            return;
        };
        if step != 0 {
            self.apply_page_step(step, window, cx);
        }
        cx.notify();
    }

    pub fn reader_start_page_flip_anim(
        &mut self,
        start_x: f32,
        target_x: f32,
        dir: i8,
        pending_step: isize,
        duration_ms: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(r) = &mut self.reader else {
            return;
        };
        r.flip_anim_gen = r.flip_anim_gen.wrapping_add(1);
        r.flip_anim = Some(PageFlipAnimation {
            dir,
            pending_step,
            started_at: Instant::now(),
            duration_ms: duration_ms.max(40),
            start_x,
            target_x,
        });
        r.swipe_offset_x = 0.0;
        r.swipe_active = false;
        cx.notify();
    }

    pub fn reader_next_page_or_chapter(
        &mut self,
        start_x: Option<f32>,
        use_two_col: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_pending_page_flip(window, cx);
        let step = if use_two_col { 2isize } else { 1isize };
        let can_advance = self.reader.as_ref().is_some_and(|r| {
            r.current_page + (step as usize) < r.chapter_pages.len().max(1)
                || r.chapter_idx + 1 < r.book.chapters.len()
        });
        let vp_w = self
            .reader
            .as_ref()
            .map(|r| f32::from(r.viewport_bounds.get().size.width).max(320.0))
            .unwrap_or(360.0);
        if can_advance {
            if let Some(sx) = start_x {
                self.reader_start_page_flip_anim(sx, -vp_w, 1, step, 175, cx);
            } else {
                if let Some(r) = &mut self.reader {
                    r.swipe_offset_x = 0.0;
                    r.swipe_active = false;
                }
                self.apply_page_step(step, window, cx);
                cx.notify();
            }
        } else if let Some(sx) = start_x
            && sx.abs() > 1.0
        {
            self.reader_start_page_flip_anim(sx, 0.0, 0, 0, 160, cx);
        }
    }

    pub fn reader_prev_page_or_chapter(
        &mut self,
        start_x: Option<f32>,
        use_two_col: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.flush_pending_page_flip(window, cx);
        let step = if use_two_col { 2isize } else { 1isize };
        let can_retreat = self
            .reader
            .as_ref()
            .is_some_and(|r| r.current_page > 0 || r.chapter_idx > 0);
        let vp_w = self
            .reader
            .as_ref()
            .map(|r| f32::from(r.viewport_bounds.get().size.width).max(320.0))
            .unwrap_or(360.0);
        if can_retreat {
            if let Some(sx) = start_x {
                self.reader_start_page_flip_anim(sx, vp_w, -1, -step, 175, cx);
            } else {
                if let Some(r) = &mut self.reader {
                    r.swipe_offset_x = 0.0;
                    r.swipe_active = false;
                }
                self.apply_page_step(-step, window, cx);
                cx.notify();
            }
        } else if let Some(sx) = start_x
            && sx.abs() > 1.0
        {
            self.reader_start_page_flip_anim(sx, 0.0, 0, 0, 160, cx);
        }
    }

    fn persist_reader_progress(&mut self, scroll_pct: f32, cx: &mut Context<Self>) {
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
        let loc = r.global_location(clamped);
        self.library_store.update_progress_global(
            &r.book_rel,
            r.chapter_idx,
            total,
            clamped,
            loc.progress_frac,
            loc.page,
            loc.total_pages,
            already_finished,
        );
        r.last_saved_scroll_pct = clamped;
        self.schedule_library_sync_save(false, cx);
    }

    pub fn open_book(&mut self, rel_path: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_book_with_notes(rel_path, false, window, cx);
    }

    pub fn open_book_with_notes(
        &mut self,
        rel_path: &str,
        open_notes: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(rec) = self.books.iter().find(|b| b.rel_path == rel_path).cloned() else {
            return;
        };
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        self.book_context_menu = None;
        self.opening_book_title = Some(rec.display_title().to_string());
        self.open_book_gen = self.open_book_gen.wrapping_add(1);
        let seq = self.open_book_gen;
        let state_root = self.library_state_root.clone();
        cx.notify();

        self._open_book_task = Some(cx.spawn_in(window, async move |this, cx| {
            let ep_path = rec.path.clone();
            let loaded = cx
                .background_executor()
                .spawn(async move {
                    let has_state = LibraryStore::state_path(&state_root).is_file();
                    let (loaded_store, merged_conflicts) =
                        LibraryStore::load_with_conflict_report(&state_root);
                    let book_res = EpubBook::load(&ep_path);
                    (has_state, loaded_store, merged_conflicts, book_res)
                })
                .await;

            let _ = this.update_in(cx, |app, window, cx| {
                if app.open_book_gen != seq {
                    return;
                }
                app.opening_book_title = None;
                let (has_state, loaded_store, merged_conflicts, book_res) = loaded;
                if has_state || merged_conflicts > 0 {
                    app.library_store.merge_from(loaded_store);
                }
                let user_state = app
                    .library_store
                    .books
                    .get(&rec.rel_path)
                    .cloned()
                    .unwrap_or_else(|| rec.user.clone());
                let book = match book_res {
                    Ok(b) => b,
                    Err(err) => {
                        app.status_banner = Some(format!("Failed to open EPUB: {err}"));
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
                let cum_weights = book.chapter_cumulative_weights();
                let init_loc =
                    EpubBook::global_location_from_weights(&cum_weights, start_ch, restored_scroll);
                app.library_store.update_progress_global(
                    &rec.rel_path,
                    start_ch,
                    total,
                    restored_scroll,
                    init_loc.progress_frac,
                    init_loc.page,
                    init_loc.total_pages,
                    already_finished,
                );
                if merged_conflicts > 0 {
                    app.status_banner = Some(format!(
                        "Merged {merged_conflicts} synced library conflict(s)"
                    ));
                }

                app.notes_scroll.set_offset(point(px(0.), px(0.)));
                window.blur(cx);
                gpui_mobile::hide_keyboard();
                let pending_scroll_fraction = (restored_scroll > 0.005).then_some(restored_scroll);
                app.last_opened_rel = Some(rec.rel_path.clone());
                app.reader = Some(ReaderState {
                    book_rel: rec.rel_path,
                    book_title,
                    book,
                    chapter_cum_weights: cum_weights,
                    chapter_idx: start_ch,
                    show_settings: false,
                    show_toc: false,
                    show_notes: open_notes,
                    zen_mode: false,
                    notes_color_filter: None,
                    notes_only_filter: false,
                    draft: None,
                    draft_dock_top: Cell::new(false),
                    draft_dock_initialized: Cell::new(false),
                    dict_lookup: None,
                    dict_lookup_gen: 0,
                    draft_sheet_bounds: Rc::new(RefCell::new(None)),
                    settings_strip_bounds: Rc::new(RefCell::new(None)),
                    viewport_bounds: Rc::new(Cell::new(Bounds::default())),
                    chapter_pages: vec![ChapterPage {
                        slices: Vec::new(),
                        show_chapter_header: false,
                    }],
                    adjacent_pages_cache: HashMap::new(),
                    pages_layout_key: None,
                    pages_chapter_idx: None,
                    pending_prepag: None,
                    prepag_scheduled: false,
                    current_page: 0,
                    pending_last_page: false,
                    needs_page_recompute: true,
                    selection_anchor: None,
                    selection_head: None,
                    is_long_press_selecting: false,
                    long_press_anchor_word: None,
                    long_press_origin_pos: None,
                    long_press_dragged: false,
                    dragging_handle: None,
                    handle_drag_offset_y: 16.0,
                    sel_handle_positions: Rc::new(RefCell::new(None)),
                    mouse_down_pos: None,
                    is_mouse_selecting: false,
                    page_touch_mode: None,
                    page_touch_gen: 0,
                    page_touch_start_x: 0.0,
                    page_touch_cur_pos: Point::default(),
                    page_touch_last_x: 0.0,
                    page_touch_last_at: None,
                    page_touch_vel_x: 0.0,
                    _page_lp_task: None,
                    swipe_offset_x: 0.0,
                    swipe_active: false,
                    swipe_ignore_momentum: false,
                    flip_anim: None,
                    flip_anim_gen: 0,
                    rendered_layouts: Rc::new(RefCell::new(Vec::new())),
                    highlights_epoch: 0,
                    slice_render_cache: RefCell::new(HashMap::new()),
                    decoded_images: RefCell::new(HashMap::new()),
                    pending_scroll_block: None,
                    pending_scroll_fraction,
                    last_saved_scroll_pct: restored_scroll,
                });
                app.surface = AppSurface::Reader;
                app.schedule_library_sync_save(false, cx);
                cx.notify();
            });
        }));
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
            r.clear_selection_and_draft();
            r.current_page = 0;
            r.pending_last_page = false;
            r.needs_page_recompute = true;
            r.prepag_scheduled = false;
            r.rendered_layouts.borrow_mut().clear();
            r.pending_scroll_block = None;
            r.pending_scroll_fraction = None;
            self.persist_reader_progress(0.0, cx);
            if gpui_mobile::keyboard_height() > 0.0 {
                window.blur(cx);
                gpui_mobile::hide_keyboard();
            }
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
            r.prepag_scheduled = false;
            r.rendered_layouts.borrow_mut().clear();
            r.pending_scroll_block = Some((hl.start_block, hl.start_offset));
            r.pending_scroll_fraction = Some(frac);
            r.needs_page_recompute = true;
            self.persist_reader_progress(frac, cx);
            if gpui_mobile::keyboard_height() > 0.0 {
                window.blur(cx);
                gpui_mobile::hide_keyboard();
            }
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
            r.prepag_scheduled = false;
            r.rendered_layouts.borrow_mut().clear();
            r.pending_scroll_block = None;
            r.pending_scroll_fraction = Some(frac);
            r.needs_page_recompute = true;
            self.persist_reader_progress(frac, cx);
            if gpui_mobile::keyboard_height() > 0.0 {
                window.blur(cx);
                gpui_mobile::hide_keyboard();
            }
        }
        cx.notify();
    }

    pub fn reader_open_existing_highlight(
        &mut self,
        hl: &BookHighlight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.highlight_note_input
            .update(cx, |inp, cx| inp.set_value(hl.note.clone(), window, cx));
        if let Some(r) = &mut self.reader {
            r.draft_dock_initialized.set(false);
            r.selection_anchor = Some(TextPos {
                block_idx: hl.start_block,
                byte_offset: hl.start_offset,
            });
            r.selection_head = Some(TextPos {
                block_idx: hl.end_block,
                byte_offset: hl.end_offset,
            });
            r.draft = Some(HighlightEditorDraft {
                start_block: hl.start_block,
                start_offset: hl.start_offset,
                end_block: hl.end_block,
                end_offset: hl.end_offset,
                quote: hl.quote.clone(),
                existing_id: Some(hl.id.clone()),
                color: hl.color,
                style: hl.style,
            });
        }
        self.trigger_reader_dict_lookup(&hl.quote, cx);
        cx.notify();
    }

    pub fn reader_finalize_selection_draft(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let last_color = self.library_store.reader_prefs.last_highlight_color;
        let last_style = self.library_store.reader_prefs.last_highlight_style;
        let quote_for_lookup = if let Some(r) = &mut self.reader {
            r.sync_draft_from_selection(last_color, last_style);
            r.draft.as_ref().map(|d| d.quote.clone())
        } else {
            None
        };
        if let Some(quote) = quote_for_lookup {
            let note_val = self
                .reader
                .as_ref()
                .and_then(|r| {
                    let book_rel = &r.book_rel;
                    r.draft.as_ref().and_then(|d| {
                        d.existing_id.as_ref().and_then(|id| {
                            self.library_store
                                .books
                                .get(book_rel)
                                .and_then(|u| u.highlights.iter().find(|h| &h.id == id))
                                .map(|h| h.note.clone())
                        })
                    })
                })
                .unwrap_or_default();
            self.highlight_note_input
                .update(cx, |inp, cx| inp.set_value(note_val, window, cx));
            self.trigger_reader_dict_lookup(&quote, cx);
        }
        cx.notify();
    }

    pub fn trigger_reader_dict_lookup(&mut self, query: &str, cx: &mut Context<Self>) {
        let clean = extract_lookup_word(query);
        let Some(r) = &mut self.reader else {
            return;
        };
        r.dict_lookup_gen = r.dict_lookup_gen.wrapping_add(1);
        let lookup_seq = r.dict_lookup_gen;
        let book_rel = r.book_rel.clone();
        let Some(term) = clean else {
            r.dict_lookup = None;
            return;
        };

        if let Some(hits) = self.dict_store.borrow_mut().lookup_offline(&term) {
            r.dict_lookup = Some(DictLookupState::Found(hits));
            return;
        }

        r.dict_lookup = Some(DictLookupState::Loading(term.clone()));
        let dict_dir = self.dict_dir.clone();
        let raw_trimmed = query.trim().to_string();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { lookup_online_and_cache(&term, &dict_dir) })
                .await;
            let _ = this.update(cx, |app, cx| {
                app.dict_store.borrow_mut().reload();
                app.dict_stats = app.dict_store.borrow().stats();
                if let Some(r) = &mut app.reader
                    && r.book_rel == book_rel
                    && r.dict_lookup_gen == lookup_seq
                {
                    r.dict_lookup = Some(match res {
                        Some(entries) if !entries.is_empty() => DictLookupState::Found(entries),
                        _ => DictLookupState::NotFound(raw_trimmed),
                    });
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn reader_save_dict_as_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(note_line) = self.reader.as_ref().and_then(|r| match &r.dict_lookup {
            Some(DictLookupState::Found(entries)) => entries.first().map(|e| e.format_as_note()),
            _ => None,
        }) else {
            return;
        };
        let existing_val = self.highlight_note_input.read(cx).value().trim().to_string();
        let next_note = if existing_val.is_empty() {
            note_line
        } else if existing_val.contains(&note_line) {
            existing_val
        } else {
            format!("{existing_val}\n{note_line}")
        };
        self.highlight_note_input
            .update(cx, |inp, cx| inp.set_value(next_note, window, cx));
        self.reader_save_highlight_draft(window, cx);
    }

    pub fn download_offline_dictionary(&mut self, cx: &mut Context<Self>) {
        if self.dict_downloading {
            return;
        }
        self.dict_downloading = true;
        self.status_banner = Some("Downloading offline EN/RU dictionary pack…".to_string());
        cx.notify();
        let dict_dir = self.dict_dir.clone();
        cx.spawn(async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { download_offline_pack(&dict_dir, DEFAULT_DICT_PACK_URL) })
                .await;
            let _ = this.update(cx, |app, cx| {
                app.dict_downloading = false;
                match res {
                    Ok(stats) => {
                        app.dict_store.borrow_mut().reload();
                        app.dict_stats = stats.clone();
                        app.status_banner = Some(format!(
                            "Offline EN/RU dictionary ready ({} lemmas, {} forms)",
                            stats.lemma_count, stats.form_count
                        ));
                    }
                    Err(err) => {
                        app.status_banner =
                            Some(format!("Dictionary download failed: {err}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn reader_copy_active_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = &mut self.reader {
            let text = if let Some((s, e)) = r.normalized_selection() {
                r.extract_text_for_range(s, e)
            } else if let Some(d) = &r.draft {
                d.quote.clone()
            } else {
                String::new()
            };
            if !text.trim().is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.status_banner = Some("Copied selection to clipboard".to_string());
            }
            r.clear_selection_and_draft();
        }
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        cx.notify();
    }

    pub fn reader_save_highlight_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let note_text = self.highlight_note_input.read(cx).value().trim().to_string();
        if let Some(r) = &mut self.reader
            && let Some(d) = r.draft.take()
        {
            r.selection_anchor = None;
            r.selection_head = None;
            r.is_long_press_selecting = false;
            r.long_press_anchor_word = None;
            r.long_press_origin_pos = None;
            r.long_press_dragged = false;
            r.dragging_handle = None;
            r.draft_dock_initialized.set(false);
            *r.sel_handle_positions.borrow_mut() = None;
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
                    d.start_block,
                    chrono::Utc::now().timestamp_millis()
                )
            });
            self.library_store.reader_prefs.last_highlight_color = d.color;
            self.library_store.reader_prefs.last_highlight_style = d.style;
            self.library_store.upsert_highlight(
                &r.book_rel,
                BookHighlight {
                    id,
                    chapter_idx: r.chapter_idx,
                    chapter_title: ch_title,
                    start_block: d.start_block,
                    start_offset: d.start_offset,
                    end_block: d.end_block,
                    end_offset: d.end_offset,
                    quote: d.quote,
                    color: d.color,
                    style: d.style,
                    note: note_text,
                    created_at: now.clone(),
                    updated_at: now,
                },
            );
            r.highlights_epoch = r.highlights_epoch.wrapping_add(1);
            r.slice_render_cache.borrow_mut().clear();
            self.schedule_library_sync_save(true, cx);
        }
        window.blur(cx);
        gpui_mobile::hide_keyboard();
        cx.notify();
    }

    pub fn reader_delete_draft_highlight(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(r) = &mut self.reader {
            r.selection_anchor = None;
            r.selection_head = None;
            r.is_long_press_selecting = false;
            r.long_press_anchor_word = None;
            r.long_press_origin_pos = None;
            r.long_press_dragged = false;
            r.dragging_handle = None;
            r.draft_dock_initialized.set(false);
            *r.sel_handle_positions.borrow_mut() = None;
            if let Some(d) = r.draft.take()
                && let Some(id) = d.existing_id
            {
                self.library_store.delete_highlight(&r.book_rel, &id);
                r.highlights_epoch = r.highlights_epoch.wrapping_add(1);
                r.slice_render_cache.borrow_mut().clear();
                self.schedule_library_sync_save(true, cx);
            }
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
            self.schedule_library_sync_save(true, cx);
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
        if let Some(r) = &mut self.reader {
            r.slice_render_cache.borrow_mut().clear();
        }
        self.schedule_library_sync_save(false, cx);
        cx.notify();
    }

    pub fn adjust_reader_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.reader_font_size = (self.reader_font_size + delta).clamp(13.0, 28.0);
        self.library_store.reader_prefs.font_size = self.reader_font_size;
        if let Some(r) = &mut self.reader {
            r.needs_page_recompute = true;
        }
        self.schedule_library_sync_save(false, cx);
        cx.notify();
    }

    pub fn set_reader_width(&mut self, width: ReaderWidthPref, cx: &mut Context<Self>) {
        self.reader_width = width;
        self.library_store.reader_prefs.width = width;
        if let Some(r) = &mut self.reader {
            r.rendered_layouts.borrow_mut().clear();
            r.needs_page_recompute = true;
        }
        self.schedule_library_sync_save(false, cx);
        cx.notify();
    }

    pub fn set_reader_zen_mode(&mut self, zen: bool, cx: &mut Context<Self>) {
        if let Some(r) = &mut self.reader
            && r.zen_mode != zen
        {
            r.zen_mode = zen;
            if zen {
                r.show_settings = false;
                r.show_toc = false;
                r.show_notes = false;
            }
            r.needs_page_recompute = true;
            cx.notify();
        }
    }

    pub fn toggle_two_column_spread(&mut self, cx: &mut Context<Self>) {
        self.two_column_spread = !self.two_column_spread;
        if let Some(r) = &mut self.reader {
            r.rendered_layouts.borrow_mut().clear();
            r.needs_page_recompute = true;
        }
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
            self.persist_reader_progress(pct, cx);
            self.schedule_library_sync_save(true, cx);
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

impl Render for BookwormMobileApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let epoch = crate::android_resume_epoch();
        if epoch != self.last_resume_epoch {
            self.last_resume_epoch = epoch;
            let access = crate::has_all_files_access();
            if access != self.has_storage_access {
                self.recheck_storage_and_refresh(cx);
            }
        }
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
                    .when(self.surface == AppSurface::Library && self.library_drawer_open, |frame| {
                        frame.child(library_view::render_library_drawer(self, window, cx))
                    })
                    .when(self.book_context_menu.is_some(), |frame| {
                        frame.child(library_view::render_book_context_menu(self, cx))
                    })
                    .when_some(self.opening_book_title.clone(), |frame, title| {
                        frame.child(
                            div()
                                .id("lib-opening-pill")
                                .absolute()
                                .inset_0()
                                .bg(crate::theme::hex_alpha(0x0000005a))
                                .occlude()
                                .flex()
                                .items_center()
                                .justify_center()
                                .p_6()
                                .child(
                                    div()
                                        .px_4()
                                        .py_3()
                                        .rounded(px(14.))
                                        .bg(crate::theme::hex_color(crate::theme::kd::BG_SIDEBAR))
                                        .border_1()
                                        .border_color(crate::theme::hex_color(
                                            crate::theme::kd::BORDER,
                                        ))
                                        .shadow_lg()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(crate::theme::hex_color(crate::theme::kd::FG))
                                        .child(format!("Opening {title}…")),
                                )
                                .with_animation(
                                    "lib-opening-fade",
                                    Animation::new(Duration::from_millis(140))
                                        .with_easing(ease_out_quint()),
                                    |el, t| el.opacity(t),
                                ),
                        )
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

#[cfg(test)]
mod tests {
    use super::{
        SelectionHandleKind, TextPos, compute_handle_drag_update, extend_long_press_selection,
    };

    fn pos(block_idx: usize, byte_offset: usize) -> TextPos {
        TextPos {
            block_idx,
            byte_offset,
        }
    }

    #[test]
    fn extend_long_press_selection_preserves_whole_word_and_extends_both_ways() {
        let word = (pos(2, 10), pos(2, 20));
        // Inside-word jitter keeps entire word selected
        assert_eq!(extend_long_press_selection(word, pos(2, 14)), word);
        assert_eq!(extend_long_press_selection(word, pos(2, 10)), word);
        assert_eq!(extend_long_press_selection(word, pos(2, 20)), word);
        // Extending forward anchors at word start
        assert_eq!(
            extend_long_press_selection(word, pos(2, 28)),
            (pos(2, 10), pos(2, 28))
        );
        // Extending backward anchors at word end
        assert_eq!(
            extend_long_press_selection(word, pos(2, 3)),
            (pos(2, 20), pos(2, 3))
        );
    }

    #[test]
    fn compute_handle_drag_update_flips_on_crossover_and_never_collapses() {
        let s = pos(1, 10);
        let e = pos(1, 20);
        // Normal Start/End drags
        assert_eq!(
            compute_handle_drag_update(SelectionHandleKind::Start, s, e, pos(1, 5)),
            Some((pos(1, 5), e, SelectionHandleKind::Start))
        );
        assert_eq!(
            compute_handle_drag_update(SelectionHandleKind::End, s, e, pos(1, 30)),
            Some((s, pos(1, 30), SelectionHandleKind::End))
        );
        // Collapsing to empty is rejected
        assert_eq!(
            compute_handle_drag_update(SelectionHandleKind::Start, s, e, e),
            None
        );
        assert_eq!(
            compute_handle_drag_update(SelectionHandleKind::End, s, e, s),
            None
        );
        // Crossing over flips the active handle
        assert_eq!(
            compute_handle_drag_update(SelectionHandleKind::Start, s, e, pos(1, 25)),
            Some((e, pos(1, 25), SelectionHandleKind::End))
        );
        assert_eq!(
            compute_handle_drag_update(SelectionHandleKind::End, s, e, pos(1, 4)),
            Some((pos(1, 4), s, SelectionHandleKind::Start))
        );
    }
}
