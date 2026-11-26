use crate::epub::{EpubCoverData, EpubMetadata, extension_for_cover, probe_epub_fast};
use crate::{atomic_write_string, can_read_dir, is_ignored_dir};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const LIBRARY_STATE_FILE: &str = ".vkarma-library.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookFormat {
    Epub,
    Pdf,
}

impl BookFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Epub => "EPUB",
            Self::Pdf => "PDF",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Epub => "epub",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    #[default]
    Unread,
    Reading,
    Finished,
    OnHold,
}

impl ReadingStatus {
    pub fn all() -> &'static [ReadingStatus] {
        &[Self::Unread, Self::Reading, Self::Finished, Self::OnHold]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Unread => "Unread",
            Self::Reading => "Reading",
            Self::Finished => "Finished",
            Self::OnHold => "On Hold",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BookBookmark {
    pub id: String,
    pub chapter_idx: usize,
    pub chapter_title: String,
    pub scroll_pct: f32,
    pub note: String,
    pub created_at: String,
}

impl Default for BookBookmark {
    fn default() -> Self {
        Self {
            id: String::new(),
            chapter_idx: 0,
            chapter_title: String::new(),
            scroll_pct: 0.0,
            note: String::new(),
            created_at: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightColor {
    #[default]
    Yellow,
    Green,
    Blue,
    Pink,
    Purple,
}

impl HighlightColor {
    pub fn all() -> &'static [HighlightColor] {
        &[
            Self::Yellow,
            Self::Green,
            Self::Blue,
            Self::Pink,
            Self::Purple,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Pink => "Pink",
            Self::Purple => "Purple",
        }
    }

    pub fn rgb_hex(self) -> u32 {
        match self {
            Self::Yellow => 0xf2c94c,
            Self::Green => 0x6fcf97,
            Self::Blue => 0x56ccf2,
            Self::Pink => 0xf28b82,
            Self::Purple => 0xbb6bd9,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightDecoration {
    #[default]
    Highlight,
    Underline,
    Squiggly,
    Strikethrough,
}

impl HighlightDecoration {
    pub fn all() -> &'static [HighlightDecoration] {
        &[
            Self::Highlight,
            Self::Underline,
            Self::Squiggly,
            Self::Strikethrough,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Highlight => "Highlight",
            Self::Underline => "Underline",
            Self::Squiggly => "Squiggly",
            Self::Strikethrough => "Strike",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BookHighlight {
    pub id: String,
    pub chapter_idx: usize,
    pub chapter_title: String,
    pub start_block: usize,
    pub start_offset: usize,
    pub end_block: usize,
    pub end_offset: usize,
    pub quote: String,
    pub color: HighlightColor,
    pub style: HighlightDecoration,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

impl BookHighlight {
    pub fn block_byte_range(
        &self,
        chapter_idx: usize,
        block_idx: usize,
        block_len: usize,
    ) -> Option<std::ops::Range<usize>> {
        if self.chapter_idx != chapter_idx
            || block_idx < self.start_block
            || block_idx > self.end_block
        {
            return None;
        }
        let start = if block_idx == self.start_block {
            self.start_offset.min(block_len)
        } else {
            0
        };
        let end = if block_idx == self.end_block {
            self.end_offset.min(block_len)
        } else {
            block_len
        };
        (start < end).then_some(start..end)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BookUserState {
    pub status: ReadingStatus,
    pub rating: u8,
    pub shelves: Vec<String>,
    pub chapter_idx: usize,
    pub total_chapters: usize,
    pub scroll_pct: f32,
    pub overall_pct: f32,
    #[serde(default)]
    pub current_page: usize,
    #[serde(default)]
    pub total_pages: usize,
    pub last_opened: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub bookmarks: Vec<BookBookmark>,
    pub highlights: Vec<BookHighlight>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub deleted_highlights: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub deleted_bookmarks: BTreeMap<String, String>,
    pub metadata_override: Option<EpubMetadata>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderThemePref {
    #[default]
    Dark,
    Sepia,
    Paper,
    Amoled,
}

impl ReaderThemePref {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Sepia => "Sepia",
            Self::Paper => "Paper",
            Self::Amoled => "AMOLED",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Sepia,
            Self::Sepia => Self::Paper,
            Self::Paper => Self::Amoled,
            Self::Amoled => Self::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderFontPref {
    #[default]
    Serif,
    Sans,
}

impl ReaderFontPref {
    pub fn label(self) -> &'static str {
        match self {
            Self::Serif => "Serif",
            Self::Sans => "Sans",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Serif => Self::Sans,
            Self::Sans => Self::Serif,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderWidthPref {
    Narrow,
    #[default]
    Standard,
    Wide,
    TwoColumn,
    Full,
}

impl ReaderWidthPref {
    pub fn all_single_column() -> &'static [ReaderWidthPref] {
        &[Self::Narrow, Self::Standard, Self::Wide, Self::Full]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Narrow => "Narrow",
            Self::Standard => "Comfort",
            Self::Wide => "Wide",
            Self::TwoColumn => "2-Col",
            Self::Full => "Full",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Narrow => Self::Standard,
            Self::Standard => Self::Wide,
            Self::Wide => Self::TwoColumn,
            Self::TwoColumn => Self::Full,
            Self::Full => Self::Narrow,
        }
    }

    pub fn max_width_px_opt(self) -> Option<f32> {
        match self {
            Self::Narrow => Some(560.0),
            Self::Standard => Some(700.0),
            Self::Wide => Some(900.0),
            Self::TwoColumn => Some(1140.0),
            Self::Full => None,
        }
    }

    pub fn max_width_px(self) -> f32 {
        self.max_width_px_opt().unwrap_or(100_000.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderPaginationPref {
    #[default]
    Page,
    Scroll,
}

impl ReaderPaginationPref {
    pub fn label(self) -> &'static str {
        match self {
            Self::Scroll => "Scroll",
            Self::Page => "Page",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Scroll => Self::Page,
            Self::Page => Self::Scroll,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSlice {
    pub block_idx: usize,
    pub byte_range: Range<usize>,
    pub is_continuation: bool,
    pub continues_next: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterPage {
    pub slices: Vec<PageSlice>,
    pub show_chapter_header: bool,
}

#[derive(Debug, Clone)]
pub enum MeasuredBlock {
    Text {
        block_idx: usize,
        line_ranges: Vec<Range<usize>>,
        line_height: f32,
        pad_top: f32,
        pad_bottom: f32,
        is_heading: bool,
    },
    Fixed {
        block_idx: usize,
        height: f32,
    },
}

/// Packs shaped visual lines and fixed blocks (images/rules) into exact-height pages
/// without vertical scroll fighting or half-clipped lines.
pub fn paginate_measured_blocks(
    blocks: &[MeasuredBlock],
    page_height: f32,
    chapter_header_height: f32,
    block_gap: f32,
) -> Vec<ChapterPage> {
    let eff_page_h = page_height.max(80.0);
    let hdr_h = chapter_header_height.max(0.0).min(eff_page_h * 0.55);
    let gap = block_gap.max(0.0);

    let mut pages: Vec<ChapterPage> = Vec::new();
    let mut cur = ChapterPage {
        slices: Vec::new(),
        show_chapter_header: hdr_h > 0.5,
    };
    let mut used_h = hdr_h;

    let flush_page = |pages: &mut Vec<ChapterPage>,
                      cur: &mut ChapterPage,
                      used_h: &mut f32| {
        pages.push(std::mem::replace(
            cur,
            ChapterPage {
                slices: Vec::new(),
                show_chapter_header: false,
            },
        ));
        *used_h = 0.0;
    };

    for blk in blocks {
        match blk {
            MeasuredBlock::Fixed { block_idx, height } => {
                let h = height.max(4.0);
                let needed = if cur.slices.is_empty() { h } else { gap + h };
                if !cur.slices.is_empty() && used_h + needed > eff_page_h + 0.5 {
                    flush_page(&mut pages, &mut cur, &mut used_h);
                }
                let add_gap = if cur.slices.is_empty() { 0.0 } else { gap };
                cur.slices.push(PageSlice {
                    block_idx: *block_idx,
                    byte_range: 0..0,
                    is_continuation: false,
                    continues_next: false,
                });
                used_h += add_gap + h;
            }
            MeasuredBlock::Text {
                block_idx,
                line_ranges,
                line_height,
                pad_top,
                pad_bottom,
                is_heading,
            } => {
                let lh = line_height.max(10.0);
                let top_p = pad_top.max(0.0);
                let bot_p = pad_bottom.max(0.0);
                if line_ranges.is_empty() {
                    continue;
                }

                let total_block_h = top_p + bot_p + lh * (line_ranges.len() as f32);
                let lead_gap = if cur.slices.is_empty() { 0.0 } else { gap };

                // Heading orphan guard: move heading + at least ~2 body lines together onto next page.
                if *is_heading
                    && !cur.slices.is_empty()
                    && used_h + lead_gap + total_block_h + gap + 48.0 > eff_page_h + 0.5
                {
                    flush_page(&mut pages, &mut cur, &mut used_h);
                }

                let mut start_line = 0usize;
                let mut is_cont = false;
                while start_line < line_ranges.len() {
                    let add_gap = if cur.slices.is_empty() { 0.0 } else { gap };
                    let cur_top_p = if is_cont { 0.0 } else { top_p };
                    let avail = eff_page_h - used_h - add_gap - cur_top_p;

                    // If not even 1 line fits on a non-empty page, start a fresh page cleanly.
                    if !cur.slices.is_empty() && avail + 0.5 < lh {
                        flush_page(&mut pages, &mut cur, &mut used_h);
                        continue;
                    }

                    let eff_avail = avail.max(lh);
                    let remaining_lines = line_ranges.len() - start_line;
                    let max_lines_without_bottom = ((eff_avail + 0.5) / lh).floor().max(1.0) as usize;

                    let (take_lines, add_bot_p, continues_next) =
                        if remaining_lines <= max_lines_without_bottom {
                            let with_bot = cur_top_p + (remaining_lines as f32) * lh + bot_p;
                            if !cur.slices.is_empty() && used_h + add_gap + with_bot > eff_page_h + 0.5
                                && remaining_lines > 1
                            {
                                (remaining_lines - 1, 0.0, true)
                            } else {
                                (remaining_lines, bot_p, false)
                            }
                        } else {
                            (max_lines_without_bottom.min(remaining_lines), 0.0, true)
                        };

                    let end_line = start_line + take_lines;
                    let byte_start = line_ranges[start_line].start;
                    let byte_end = line_ranges[end_line - 1].end;
                    cur.slices.push(PageSlice {
                        block_idx: *block_idx,
                        byte_range: byte_start..byte_end,
                        is_continuation: is_cont,
                        continues_next,
                    });
                    used_h += add_gap + cur_top_p + (take_lines as f32) * lh + add_bot_p;

                    start_line = end_line;
                    is_cont = true;
                    if continues_next {
                        flush_page(&mut pages, &mut cur, &mut used_h);
                    }
                }
            }
        }
    }

    if !cur.slices.is_empty() || pages.is_empty() {
        pages.push(cur);
    }
    pages
}

pub fn page_index_for_block(
    pages: &[ChapterPage],
    block_idx: usize,
    byte_offset: usize,
) -> usize {
    for (p_ix, page) in pages.iter().enumerate() {
        for slice in &page.slices {
            if slice.block_idx == block_idx {
                if slice.byte_range.is_empty()
                    || (byte_offset >= slice.byte_range.start && byte_offset <= slice.byte_range.end)
                {
                    return p_ix;
                }
            }
        }
    }
    // Fallback to first page containing `block_idx`, or nearest earlier page.
    for (p_ix, page) in pages.iter().enumerate() {
        if page.slices.iter().any(|s| s.block_idx >= block_idx) {
            return p_ix;
        }
    }
    pages.len().saturating_sub(1)
}

/// Computes discrete `(page_start_y, page_clip_h)` slices for a chapter so page breaks
/// snap cleanly between text lines and never horizontally slice a line in half.
/// `line_bands` must contain `(top_y, bottom_y)` spans in content coordinates (origin at 0.0).
pub fn compute_chapter_page_offsets(
    viewport_h: f32,
    total_content_h: f32,
    line_bands: &[(f32, f32)],
) -> Vec<(f32, f32)> {
    if viewport_h <= 1.0 || total_content_h <= viewport_h + 2.0 {
        return vec![(0.0, viewport_h.max(1.0))];
    }

    let usable_h = (viewport_h - 14.0).max(viewport_h * 0.65).max(80.0);
    let mut bands: Vec<(f32, f32)> = line_bands
        .iter()
        .copied()
        .filter(|&(t, b)| b > t + 0.5)
        .collect();
    bands.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));

    let mut pages = Vec::new();
    let mut y = 0.0_f32;
    let max_iterations = ((total_content_h / 24.0).ceil() as usize).clamp(4, 2000);

    for _ in 0..max_iterations {
        let remaining = total_content_h - y;
        if remaining <= viewport_h + 2.0 {
            let clip = remaining.clamp(40.0, viewport_h);
            pages.push((y, clip));
            break;
        }

        let raw_cut = y + usable_h;
        let min_progress = y + usable_h * 0.35;
        let mut cut = raw_cut;

        for &(top, bot) in &bands {
            if top > raw_cut + 1.0 {
                break;
            }
            if top < raw_cut && bot > raw_cut && top > min_progress {
                cut = cut.min(top);
            }
        }

        if cut <= y + 24.0 {
            cut = (y + usable_h).min(total_content_h);
        }

        let clip_h = (cut - y).clamp(40.0, viewport_h);
        pages.push((y, clip_h));
        if cut >= total_content_h - 2.0 {
            break;
        }
        y = cut;
    }

    if pages.is_empty() {
        pages.push((0.0, viewport_h.max(1.0)));
    }
    pages
}

/// Finds the page index in `pages` (`(start_y, clip_h)`) that contains `content_y`.
pub fn page_index_for_content_y(pages: &[(f32, f32)], content_y: f32) -> usize {
    if pages.len() <= 1 {
        return 0;
    }
    let y = content_y.max(0.0);
    let mut best = 0usize;
    for (ix, &(start_y, _)) in pages.iter().enumerate() {
        if start_y <= y + 4.0 {
            best = ix;
        } else {
            break;
        }
    }
    best
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderSpacingPref {
    Compact,
    #[default]
    Normal,
    Relaxed,
}

impl ReaderSpacingPref {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "1.4x",
            Self::Normal => "1.65x",
            Self::Relaxed => "1.9x",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Compact => Self::Normal,
            Self::Normal => Self::Relaxed,
            Self::Relaxed => Self::Compact,
        }
    }

    pub fn multiplier(self) -> f32 {
        match self {
            Self::Compact => 1.4,
            Self::Normal => 1.65,
            Self::Relaxed => 1.9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReaderPrefs {
    pub font_size: f32,
    pub theme: ReaderThemePref,
    pub font: ReaderFontPref,
    pub width: ReaderWidthPref,
    pub spacing: ReaderSpacingPref,
    pub pagination: ReaderPaginationPref,
    pub last_highlight_color: HighlightColor,
    pub last_highlight_style: HighlightDecoration,
}

impl Default for ReaderPrefs {
    fn default() -> Self {
        Self {
            font_size: 16.5,
            theme: ReaderThemePref::Dark,
            font: ReaderFontPref::Serif,
            width: ReaderWidthPref::Standard,
            spacing: ReaderSpacingPref::Normal,
            pagination: ReaderPaginationPref::Page,
            last_highlight_color: HighlightColor::Yellow,
            last_highlight_style: HighlightDecoration::Highlight,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryStore {
    pub version: u32,
    pub reader_prefs: ReaderPrefs,
    pub books: BTreeMap<String, BookUserState>,
}

impl Default for LibraryStore {
    fn default() -> Self {
        Self {
            version: 1,
            reader_prefs: ReaderPrefs::default(),
            books: BTreeMap::new(),
        }
    }
}

pub fn normalize_rel_key(workspace_root: &Path, path: &Path) -> String {
    let raw = path
        .strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    raw.trim_start_matches('/').to_string()
}

pub fn is_library_conflict_filename(name: &str) -> bool {
    name.starts_with(".vkarma-library.sync-conflict-") && name.ends_with(".json")
}

fn parse_rfc3339_ms(ts: &str) -> Option<i64> {
    let trimmed = ts.trim();
    if trimmed.is_empty() {
        return None;
    }
    chrono::DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn ts_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (parse_rfc3339_ms(a), parse_rfc3339_ms(b)) {
        (Some(ma), Some(mb)) => ma.cmp(&mb),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => a.cmp(b),
    }
}

fn opt_ts_max(a: &Option<String>, b: &Option<String>) -> Option<String> {
    match (a, b) {
        (Some(sa), Some(sb)) => {
            if ts_cmp(sb, sa) == std::cmp::Ordering::Greater {
                Some(sb.clone())
            } else {
                Some(sa.clone())
            }
        }
        (Some(sa), None) => Some(sa.clone()),
        (None, Some(sb)) => Some(sb.clone()),
        (None, None) => None,
    }
}

fn book_recency<'a>(s: &'a BookUserState) -> &'a str {
    let upd = s.updated_at.as_deref().unwrap_or("");
    let open = s.last_opened.as_deref().unwrap_or("");
    if ts_cmp(upd, open) != std::cmp::Ordering::Less {
        upd
    } else {
        open
    }
}

fn prune_tombstones(map: &mut BTreeMap<String, String>) {
    const MAX_TOMBSTONES: usize = 500;
    let cutoff_ms = chrono::Utc::now().timestamp_millis() - 180_i64 * 86_400 * 1000;
    map.retain(|_, ts| parse_rfc3339_ms(ts).is_none_or(|ms| ms >= cutoff_ms));
    if map.len() > MAX_TOMBSTONES {
        let mut entries: Vec<(i64, String, String)> = map
            .iter()
            .map(|(k, ts)| (parse_rfc3339_ms(ts).unwrap_or(i64::MIN), ts.clone(), k.clone()))
            .collect();
        entries.sort();
        let drop_count = entries.len() - MAX_TOMBSTONES;
        for (_, _, k) in entries.into_iter().take(drop_count) {
            map.remove(&k);
        }
    }
}

fn merge_tombstone_maps(
    target: &mut BTreeMap<String, String>,
    incoming: BTreeMap<String, String>,
) {
    for (id, ts) in incoming {
        match target.get_mut(&id) {
            Some(cur) => {
                if ts_cmp(&ts, cur) == std::cmp::Ordering::Greater {
                    *cur = ts;
                }
            }
            None => {
                target.insert(id, ts);
            }
        }
    }
    prune_tombstones(target);
}

fn highlight_effective_ts(h: &BookHighlight) -> &str {
    if !h.updated_at.trim().is_empty() {
        &h.updated_at
    } else {
        &h.created_at
    }
}

fn highlight_merge_key(h: &BookHighlight) -> String {
    if !h.id.trim().is_empty() {
        h.id.clone()
    } else {
        format!(
            "pos:{}:{}:{}:{}:{}",
            h.chapter_idx, h.start_block, h.start_offset, h.end_block, h.end_offset
        )
    }
}

fn bookmark_merge_key(bm: &BookBookmark) -> String {
    if !bm.id.trim().is_empty() {
        bm.id.clone()
    } else {
        format!(
            "bm:{}:{}:{}",
            bm.chapter_idx,
            (bm.scroll_pct * 1000.0).round() as i32,
            bm.note
        )
    }
}

fn merge_highlights_into(
    a: &mut BookUserState,
    incoming_highlights: Vec<BookHighlight>,
    incoming_deleted: BTreeMap<String, String>,
) {
    merge_tombstone_maps(&mut a.deleted_highlights, incoming_deleted);

    let mut by_key: BTreeMap<String, BookHighlight> = BTreeMap::new();
    for hl in a.highlights.drain(..).chain(incoming_highlights) {
        let key = highlight_merge_key(&hl);
        match by_key.get_mut(&key) {
            Some(cur) => {
                let cmp = ts_cmp(highlight_effective_ts(&hl), highlight_effective_ts(cur));
                let take_new = match cmp {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Less => false,
                    std::cmp::Ordering::Equal => {
                        let new_note_len = hl.note.trim().len();
                        let cur_note_len = cur.note.trim().len();
                        (new_note_len, &hl.note, &hl.quote) > (cur_note_len, &cur.note, &cur.quote)
                    }
                };
                if take_new {
                    *cur = hl;
                }
            }
            None => {
                by_key.insert(key, hl);
            }
        }
    }

    let mut stale_tombstones = Vec::new();
    a.highlights = by_key
        .into_values()
        .filter(|hl| {
            if hl.id.is_empty() {
                return true;
            }
            if let Some(del_ts) = a.deleted_highlights.get(&hl.id) {
                let eff_ts = highlight_effective_ts(hl);
                if ts_cmp(del_ts, eff_ts) != std::cmp::Ordering::Less {
                    return false;
                }
                stale_tombstones.push(hl.id.clone());
            }
            true
        })
        .collect();
    for stale_id in stale_tombstones {
        a.deleted_highlights.remove(&stale_id);
    }
    a.highlights.sort_by(|x, y| {
        x.chapter_idx
            .cmp(&y.chapter_idx)
            .then(x.start_block.cmp(&y.start_block))
            .then(x.start_offset.cmp(&y.start_offset))
            .then_with(|| ts_cmp(&x.created_at, &y.created_at))
            .then(x.id.cmp(&y.id))
    });
}

fn merge_bookmarks_into(
    a: &mut BookUserState,
    incoming_bookmarks: Vec<BookBookmark>,
    incoming_deleted: BTreeMap<String, String>,
) {
    merge_tombstone_maps(&mut a.deleted_bookmarks, incoming_deleted);

    let mut by_key: BTreeMap<String, BookBookmark> = BTreeMap::new();
    for bm in a.bookmarks.drain(..).chain(incoming_bookmarks) {
        let key = bookmark_merge_key(&bm);
        match by_key.get_mut(&key) {
            Some(cur) => {
                let cmp = ts_cmp(&bm.created_at, &cur.created_at);
                let take_new = match cmp {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Less => false,
                    std::cmp::Ordering::Equal => {
                        (bm.note.trim().len(), &bm.note, &bm.chapter_title)
                            > (cur.note.trim().len(), &cur.note, &cur.chapter_title)
                    }
                };
                if take_new {
                    *cur = bm;
                }
            }
            None => {
                by_key.insert(key, bm);
            }
        }
    }

    let mut stale_tombstones = Vec::new();
    a.bookmarks = by_key
        .into_values()
        .filter(|bm| {
            if bm.id.is_empty() {
                return true;
            }
            if let Some(del_ts) = a.deleted_bookmarks.get(&bm.id) {
                if ts_cmp(del_ts, &bm.created_at) != std::cmp::Ordering::Less {
                    return false;
                }
                stale_tombstones.push(bm.id.clone());
            }
            true
        })
        .collect();
    for stale_id in stale_tombstones {
        a.deleted_bookmarks.remove(&stale_id);
    }
    a.bookmarks.sort_by(|x, y| {
        x.chapter_idx
            .cmp(&y.chapter_idx)
            .then(x.scroll_pct.to_bits().cmp(&y.scroll_pct.to_bits()))
            .then_with(|| ts_cmp(&x.created_at, &y.created_at))
            .then(x.id.cmp(&y.id))
    });
}

fn merge_book_user_state(a: &mut BookUserState, b: BookUserState) {
    let a_rec = book_recency(a).to_string();
    let b_rec = book_recency(&b).to_string();
    let rec_cmp = ts_cmp(&b_rec, &a_rec);
    let b_newer = rec_cmp == std::cmp::Ordering::Greater;
    let same_recency = rec_cmp == std::cmp::Ordering::Equal;

    merge_highlights_into(a, b.highlights, b.deleted_highlights);
    merge_bookmarks_into(a, b.bookmarks, b.deleted_bookmarks);

    a.total_chapters = a.total_chapters.max(b.total_chapters);
    a.total_pages = a.total_pages.max(b.total_pages);
    let a_open = a.last_opened.as_deref().unwrap_or("");
    let b_open = b.last_opened.as_deref().unwrap_or("");
    let take_b_progress = match ts_cmp(b_open, a_open) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            b.overall_pct > a.overall_pct + 1e-4
                || ((b.overall_pct - a.overall_pct).abs() <= 1e-4 && b.chapter_idx > a.chapter_idx)
        }
    };
    if take_b_progress {
        a.chapter_idx = b.chapter_idx;
        a.scroll_pct = b.scroll_pct;
        a.overall_pct = b.overall_pct;
        a.current_page = b.current_page;
        if b.total_pages > 0 {
            a.total_pages = b.total_pages;
        }
        a.last_opened = b.last_opened;
    }

    if a.status == ReadingStatus::Unread && b.status != ReadingStatus::Unread {
        a.status = b.status;
    } else if b.status != ReadingStatus::Unread && b_newer {
        a.status = b.status;
    }

    if a.rating == 0 && b.rating > 0 {
        a.rating = b.rating;
    } else if b_newer {
        a.rating = b.rating;
    }

    if same_recency {
        let mut seen: BTreeSet<String> = a
            .shelves
            .iter()
            .map(|s| s.trim().to_ascii_lowercase())
            .collect();
        for sh in b.shelves {
            let trimmed = sh.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_ascii_lowercase()) {
                a.shelves.push(trimmed.to_string());
            }
        }
    } else if b_newer {
        a.shelves = b.shelves;
    }

    if a.metadata_override.is_none() || (b.metadata_override.is_some() && b_newer) {
        a.metadata_override = b.metadata_override;
    }
    a.updated_at = opt_ts_max(&a.updated_at, &b.updated_at);
}

impl LibraryStore {
    pub fn state_path(workspace_root: &Path) -> PathBuf {
        workspace_root.join(LIBRARY_STATE_FILE)
    }

    pub fn merge_from(&mut self, other: LibraryStore) {
        if self.books.is_empty() && !other.books.is_empty() {
            self.reader_prefs = other.reader_prefs;
        }
        for (key, other_state) in other.books {
            match self.books.get_mut(&key) {
                Some(cur) => merge_book_user_state(cur, other_state),
                None => {
                    self.books.insert(key, other_state);
                }
            }
        }
    }

    fn collect_conflict_paths(workspace_root: &Path) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(workspace_root) else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|ft| ft.is_file()))
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(is_library_conflict_filename)
            })
            .map(|e| e.path())
            .collect();
        paths.sort();
        paths
    }

    fn load_disk_state_and_conflicts(workspace_root: &Path) -> (Self, Vec<PathBuf>) {
        let path = Self::state_path(workspace_root);
        let mut store = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Self>(&raw).ok())
            .unwrap_or_default();

        let mut absorbed = Vec::new();
        for cpath in Self::collect_conflict_paths(workspace_root) {
            let Ok(raw) = fs::read_to_string(&cpath) else {
                continue;
            };
            let Ok(conflict_store) = serde_json::from_str::<Self>(&raw) else {
                continue;
            };
            store.merge_from(conflict_store);
            absorbed.push(cpath);
        }
        (store, absorbed)
    }

    pub fn load_with_conflict_report(workspace_root: &Path) -> (Self, usize) {
        let (store, absorbed) = Self::load_disk_state_and_conflicts(workspace_root);
        if !absorbed.is_empty() {
            if store.save(workspace_root).is_ok() {
                let count = absorbed.len();
                for p in absorbed {
                    let _ = fs::remove_file(p);
                }
                return (store, count);
            }
            return (store, 0);
        }
        (store, 0)
    }

    pub fn load(workspace_root: &Path) -> Self {
        Self::load_with_conflict_report(workspace_root).0
    }

    pub fn save(&self, workspace_root: &Path) -> Result<()> {
        let path = Self::state_path(workspace_root);
        let json = serde_json::to_string_pretty(self)?;
        atomic_write_string(&path, &json)
    }

    pub fn sync_save(&mut self, workspace_root: &Path) -> Result<usize> {
        let (disk_store, absorbed) = Self::load_disk_state_and_conflicts(workspace_root);
        self.merge_from(disk_store);
        self.save(workspace_root)?;
        let count = absorbed.len();
        for p in absorbed {
            let _ = fs::remove_file(p);
        }
        Ok(count)
    }

    pub fn update_progress(
        &mut self,
        rel_key: &str,
        chapter_idx: usize,
        total_chapters: usize,
        scroll_pct: f32,
        mark_finished: bool,
    ) {
        let clamped_scroll = scroll_pct.clamp(0.0, 1.0);
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.chapter_idx = chapter_idx;
        entry.total_chapters = total_chapters.max(1);
        entry.scroll_pct = clamped_scroll;
        entry.overall_pct = if mark_finished {
            1.0
        } else {
            ((chapter_idx as f32 + clamped_scroll) / entry.total_chapters as f32).clamp(0.0, 1.0)
        };
        let now = chrono::Utc::now().to_rfc3339();
        entry.last_opened = Some(now.clone());
        entry.updated_at = Some(now);
        if mark_finished {
            entry.status = ReadingStatus::Finished;
        } else if entry.status == ReadingStatus::Unread {
            entry.status = ReadingStatus::Reading;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_progress_global(
        &mut self,
        rel_key: &str,
        chapter_idx: usize,
        total_chapters: usize,
        scroll_pct: f32,
        overall_pct: f32,
        current_page: usize,
        total_pages: usize,
        mark_finished: bool,
    ) {
        let clamped_scroll = scroll_pct.clamp(0.0, 1.0);
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.chapter_idx = chapter_idx;
        entry.total_chapters = total_chapters.max(1);
        entry.scroll_pct = clamped_scroll;
        entry.overall_pct = if mark_finished {
            1.0
        } else {
            overall_pct.clamp(0.0, 1.0)
        };
        entry.total_pages = total_pages.max(1);
        entry.current_page = if mark_finished {
            entry.total_pages
        } else {
            current_page.clamp(1, entry.total_pages)
        };
        let now = chrono::Utc::now().to_rfc3339();
        entry.last_opened = Some(now.clone());
        entry.updated_at = Some(now);
        if mark_finished {
            entry.status = ReadingStatus::Finished;
        } else if entry.status == ReadingStatus::Unread {
            entry.status = ReadingStatus::Reading;
        }
    }

    pub fn apply_highlights_update(
        &mut self,
        rel_key: &str,
        highlights: &[BookHighlight],
        deleted_id: Option<&str>,
    ) -> Vec<BookHighlight> {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.updated_at = Some(now.clone());
        let mut incoming_deleted = BTreeMap::new();
        if let Some(id) = deleted_id {
            incoming_deleted.insert(id.to_string(), now.clone());
            entry.deleted_highlights.insert(id.to_string(), now);
            entry.highlights.retain(|h| h.id != id);
        }
        merge_highlights_into(entry, highlights.to_vec(), incoming_deleted);
        entry.highlights.clone()
    }

    pub fn apply_bookmarks_update(
        &mut self,
        rel_key: &str,
        bookmarks: &[BookBookmark],
        deleted_id: Option<&str>,
    ) -> Vec<BookBookmark> {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.updated_at = Some(now.clone());
        let mut incoming_deleted = BTreeMap::new();
        if let Some(id) = deleted_id {
            incoming_deleted.insert(id.to_string(), now.clone());
            entry.deleted_bookmarks.insert(id.to_string(), now);
            entry.bookmarks.retain(|b| b.id != id);
        }
        merge_bookmarks_into(entry, bookmarks.to_vec(), incoming_deleted);
        entry.bookmarks.clone()
    }

    pub fn upsert_highlight(&mut self, rel_key: &str, hl: BookHighlight) {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.updated_at = Some(now);
        if !hl.id.is_empty() {
            entry.deleted_highlights.remove(&hl.id);
        }
        if let Some(existing) = entry.highlights.iter_mut().find(|h| h.id == hl.id) {
            *existing = hl;
        } else {
            entry.highlights.push(hl);
        }
    }

    pub fn delete_highlight(&mut self, rel_key: &str, hl_id: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.updated_at = Some(now.clone());
        if !hl_id.is_empty() {
            entry.deleted_highlights.insert(hl_id.to_string(), now);
        }
        entry.highlights.retain(|h| h.id != hl_id);
    }

    pub fn delete_bookmark(&mut self, rel_key: &str, bm_id: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.updated_at = Some(now.clone());
        if !bm_id.is_empty() {
            entry.deleted_bookmarks.insert(bm_id.to_string(), now);
        }
        entry.bookmarks.retain(|b| b.id != bm_id);
    }

    pub fn toggle_bookmark(
        &mut self,
        rel_key: &str,
        chapter_idx: usize,
        chapter_title: &str,
        scroll_pct: f32,
    ) -> bool {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.books.entry(rel_key.to_string()).or_default();
        entry.updated_at = Some(now.clone());
        if let Some(pos) = entry
            .bookmarks
            .iter()
            .position(|b| b.chapter_idx == chapter_idx)
        {
            let removed = entry.bookmarks.remove(pos);
            if !removed.id.is_empty() {
                entry.deleted_bookmarks.insert(removed.id, now);
            }
            false
        } else {
            entry.bookmarks.push(BookBookmark {
                id: format!("bm-{chapter_idx}-{}", chrono::Utc::now().timestamp_millis()),
                chapter_idx,
                chapter_title: chapter_title.to_string(),
                scroll_pct: scroll_pct.clamp(0.0, 1.0),
                note: String::new(),
                created_at: now,
            });
            true
        }
    }

    pub fn relocate_key(
        &mut self,
        workspace_root: &Path,
        old_path: &Path,
        new_path: &Path,
    ) -> bool {
        let old_key = normalize_rel_key(workspace_root, old_path);
        let new_key = normalize_rel_key(workspace_root, new_path);
        if old_key.is_empty() || old_key == new_key {
            return false;
        }
        let old_prefix = format!("{old_key}/");
        let mut changed = false;
        let prev = std::mem::take(&mut self.books);
        for (key, state) in prev {
            let new_book_key = if key == old_key {
                changed = true;
                new_key.clone()
            } else if let Some(rest) = key.strip_prefix(&old_prefix) {
                changed = true;
                format!("{new_key}/{rest}")
            } else {
                key
            };

            self.books.insert(new_book_key, state);
        }
        changed
    }
}

#[derive(Debug, Clone)]
pub struct BookRecord {
    pub path: PathBuf,
    pub rel_path: String,
    pub format: BookFormat,
    pub file_size_bytes: u64,
    pub chapter_count: usize,
    pub metadata: EpubMetadata,
    pub cover: Option<EpubCoverData>,
    pub cover_cache_path: Option<PathBuf>,
    pub user: BookUserState,
}

impl BookRecord {
    pub fn display_title(&self) -> &str {
        if self.metadata.title.trim().is_empty() {
            self.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
        } else {
            self.metadata.title.trim()
        }
    }

    pub fn authors_display(&self) -> String {
        if self.metadata.authors.is_empty() {
            "Unknown Author".to_string()
        } else {
            self.metadata.authors.join(", ")
        }
    }

    pub fn series_badge(&self) -> Option<String> {
        let series = self.metadata.series.as_deref()?.trim();
        if series.is_empty() {
            return None;
        }
        match self.metadata.series_index {
            Some(ix) if (ix.fract()).abs() < 0.001 => Some(format!("{series} #{:.0}", ix)),
            Some(ix) => Some(format!("{series} #{ix}")),
            None => Some(series.to_string()),
        }
    }

    pub fn progress_pct(&self) -> u8 {
        if self.user.status == ReadingStatus::Finished {
            return 100;
        }
        (self.user.overall_pct.clamp(0.0, 1.0) * 100.0).round() as u8
    }

    pub fn all_tags(&self) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for tag in self.metadata.subjects.iter().chain(self.user.shelves.iter()) {
            let trimmed = tag.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_ascii_lowercase()) {
                out.push(trimmed.to_string());
            }
        }
        out
    }
}

fn fnv1a_bytes_hex(key: &str, bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in key.as_bytes().iter().chain(bytes.iter()) {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}-{:x}", bytes.len())
}

fn write_cached_cover(cache_dir: &Path, key: &str, cover: &EpubCoverData) -> Option<PathBuf> {
    if cover.bytes.is_empty() {
        return None;
    }
    let ext = extension_for_cover(&cover.media_type, &cover.zip_path);
    let path = cache_dir.join(format!("{}.{ext}", fnv1a_bytes_hex(key, &cover.bytes)));
    if path.is_file() {
        return Some(path);
    }
    if fs::create_dir_all(cache_dir).is_ok() && fs::write(&path, &cover.bytes).is_ok() {
        Some(path)
    } else {
        None
    }
}

pub fn detect_default_books_root(fallback_books_dir: &Path) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        for cand in [
            PathBuf::from("/sdcard/Documents/Library"),
            PathBuf::from("/sdcard/Books"),
            PathBuf::from("/sdcard/Download"),
        ] {
            if can_read_dir(&cand) {
                return cand;
            }
        }
        let _ = fs::create_dir_all(fallback_books_dir);
        fallback_books_dir.to_path_buf()
    }

    #[cfg(not(target_os = "android"))]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            for cand in [home.join("Documents/Library"), home.join("Books")] {
                if can_read_dir(&cand) {
                    return cand;
                }
            }
        }
        let _ = fs::create_dir_all(fallback_books_dir);
        fallback_books_dir.to_path_buf()
    }
}

pub fn library_state_root_for(books_root: &Path, fallback_books_dir: &Path) -> PathBuf {
    if can_read_dir(books_root) {
        books_root.to_path_buf()
    } else {
        let _ = fs::create_dir_all(fallback_books_dir);
        fallback_books_dir.to_path_buf()
    }
}

pub fn discover_books_root_candidates(
    current_books_root: &Path,
    fallback_books_dir: &Path,
) -> Vec<(String, PathBuf)> {
    let mut standard: Vec<(&'static str, PathBuf)> = Vec::new();
    #[cfg(target_os = "android")]
    {
        standard.push(("Documents / Library", PathBuf::from("/sdcard/Documents/Library")));
        standard.push(("Books", PathBuf::from("/sdcard/Books")));
        standard.push(("Download", PathBuf::from("/sdcard/Download")));
        standard.push(("Documents", PathBuf::from("/sdcard/Documents")));
        standard.push(("Internal Storage", PathBuf::from("/sdcard")));
    }
    #[cfg(not(target_os = "android"))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        standard.push(("Documents / Library", home.join("Documents/Library")));
        standard.push(("Books", home.join("Books")));
    }
    standard.push(("App Books Folder", fallback_books_dir.to_path_buf()));

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if can_read_dir(current_books_root)
        && !standard.iter().any(|(_, p)| p == current_books_root)
        && seen.insert(current_books_root.to_path_buf())
    {
        out.push((
            "Current Books Folder".to_string(),
            current_books_root.to_path_buf(),
        ));
    }

    for (label, path) in standard {
        if can_read_dir(&path) && seen.insert(path.clone()) {
            out.push((label.to_string(), path));
        }
    }

    out
}

fn probe_book_record(
    state_root: &Path,
    path: &Path,
    store: &LibraryStore,
    cover_cache_dir: Option<&Path>,
) -> BookRecord {
    let rel_path = normalize_rel_key(state_root, path);
    let user = store.books.get(&rel_path).cloned().unwrap_or_default();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Book")
        .to_string();

    if ext == "epub" {
        if let Ok(probed) = probe_epub_fast(path) {
            let (cover, cover_cache_path) = if let Some(cache_dir) = cover_cache_dir {
                let cached = probed
                    .cover
                    .as_ref()
                    .and_then(|c| write_cached_cover(cache_dir, &rel_path, c));
                (None, cached)
            } else {
                (probed.cover, None)
            };
            return BookRecord {
                path: path.to_path_buf(),
                rel_path,
                format: BookFormat::Epub,
                file_size_bytes: probed.file_size_bytes,
                chapter_count: probed.chapter_count,
                metadata: probed.metadata,
                cover,
                cover_cache_path,
                user,
            };
        }
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        return BookRecord {
            path: path.to_path_buf(),
            rel_path,
            format: BookFormat::Epub,
            file_size_bytes: size,
            chapter_count: 0,
            metadata: EpubMetadata {
                title: fallback_title,
                ..Default::default()
            },
            cover: None,
            cover_cache_path: None,
            user,
        };
    }

    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let metadata = user.metadata_override.clone().unwrap_or_else(|| EpubMetadata {
        title: fallback_title,
        ..Default::default()
    });
    BookRecord {
        path: path.to_path_buf(),
        rel_path,
        format: BookFormat::Pdf,
        file_size_bytes: size,
        chapter_count: 0,
        metadata,
        cover: None,
        cover_cache_path: None,
        user,
    }
}

fn scan_books_impl(
    search_roots: &[PathBuf],
    state_root: &Path,
    store: &LibraryStore,
    cover_cache_dir: Option<&Path>,
    epubs_only: bool,
) -> Vec<BookRecord> {
    let mut records = Vec::new();
    let mut seen_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for root in search_roots {
        if !root.is_dir() {
            continue;
        }
        let walker = WalkDir::new(root).follow_links(false).into_iter();
        for entry in walker.filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_str().unwrap_or_default();
            if e.file_type().is_dir() {
                return !is_ignored_dir(name);
            }
            !name.starts_with('.')
        }) {
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let matches_ext = if epubs_only {
                ext == "epub"
            } else {
                matches!(ext.as_str(), "epub" | "pdf")
            };
            if !matches_ext {
                continue;
            }
            let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            if !seen_files.insert(canon) {
                continue;
            }
            records.push(probe_book_record(state_root, path, store, cover_cache_dir));
        }
    }
    records.sort_by(|a, b| {
        a.display_title()
            .to_ascii_lowercase()
            .cmp(&b.display_title().to_ascii_lowercase())
    });
    records
}

pub fn scan_library(workspace_root: &Path, store: &LibraryStore) -> Vec<BookRecord> {
    scan_books_impl(
        std::slice::from_ref(&workspace_root.to_path_buf()),
        workspace_root,
        store,
        None,
        false,
    )
}

pub fn scan_books_cached(
    search_roots: &[PathBuf],
    state_root: &Path,
    store: &LibraryStore,
    cover_cache_dir: &Path,
) -> Vec<BookRecord> {
    scan_books_impl(search_roots, state_root, store, Some(cover_cache_dir), true)
}

pub fn sanitize_filename_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.trim().chars() {
        let clean = match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => ' ',
            _ if ch.is_control() => ' ',
            _ => ch,
        };
        if clean == ' ' {
            if !out.is_empty() && !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(clean);
            prev_space = false;
        }
    }
    out.trim().trim_end_matches('.').trim().to_string()
}

pub fn suggested_book_filename(meta: &EpubMetadata, ext: &str) -> String {
    let title = {
        let t = sanitize_filename_segment(&meta.title);
        if t.is_empty() {
            "Untitled".to_string()
        } else {
            t
        }
    };
    let clean_ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    let final_ext = if clean_ext.is_empty() { "epub" } else { &clean_ext };

    let stem = if let Some(series) = meta.series.as_deref() {
        let s = sanitize_filename_segment(series);
        if !s.is_empty() {
            if let Some(ix) = meta.series_index {
                let ix_fmt = if (ix.fract()).abs() < 0.001 && ix >= 0.0 {
                    format!("{:02}", ix.round() as u32)
                } else {
                    format!("{ix}")
                };
                format!("{s} {ix_fmt} - {title}")
            } else {
                format!("{s} - {title}")
            }
        } else if let Some(author) = meta.primary_author() {
            let a = sanitize_filename_segment(author);
            if a.is_empty() {
                title
            } else {
                format!("{a} - {title}")
            }
        } else {
            title
        }
    } else if let Some(author) = meta.primary_author() {
        let a = sanitize_filename_segment(author);
        if a.is_empty() {
            title
        } else {
            format!("{a} - {title}")
        }
    } else {
        title
    };

    format!("{stem}.{final_ext}")
}

pub fn clamp_to_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

pub fn word_bounds_at(s: &str, byte_idx: usize) -> std::ops::Range<usize> {
    let idx = clamp_to_char_boundary(s, byte_idx);
    let is_word = |c: char| c.is_alphanumeric() || matches!(c, '_' | '\'' | '\u{2019}' | '-');

    if let Some(ch) = s[idx..].chars().next() {
        if is_word(ch) {
            let mut start = idx;
            for (rel_ix, c) in s[..idx].char_indices().rev() {
                if is_word(c) {
                    start = rel_ix;
                } else {
                    break;
                }
            }
            let mut end = idx + ch.len_utf8();
            for (rel_ix, c) in s[end..].char_indices() {
                if is_word(c) {
                    end = idx + ch.len_utf8() + rel_ix + c.len_utf8();
                } else {
                    break;
                }
            }
            return start..end;
        }
        if idx > 0
            && let Some(prev) = s[..idx].chars().next_back()
            && is_word(prev)
        {
            let mut start = idx;
            for (rel_ix, c) in s[..idx].char_indices().rev() {
                if is_word(c) {
                    start = rel_ix;
                } else {
                    break;
                }
            }
            return start..idx;
        }
        if !ch.is_whitespace() {
            return idx..(idx + ch.len_utf8());
        }
    } else if idx > 0
        && let Some(prev) = s[..idx].chars().next_back()
        && is_word(prev)
    {
        let mut start = idx;
        for (rel_ix, c) in s[..idx].char_indices().rev() {
            if is_word(c) {
                start = rel_ix;
            } else {
                break;
            }
        }
        return start..idx;
    }
    idx..idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn write_minimal_epub(path: &Path, title: &str, author: &str) {
        let file = fs::File::create(path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        zw.start_file(
            "mimetype",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zw.write_all(b"application/epub+zip").unwrap();
        zw.start_file("META-INF/container.xml", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(
            br#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
                <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
            </container>"#,
        )
        .unwrap();
        zw.start_file("OEBPS/content.opf", SimpleFileOptions::default())
            .unwrap();
        let opf = format!(
            r#"<package xmlns="http://www.idpf.org/2007/opf" version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
                <metadata><dc:title>{title}</dc:title><dc:creator>{author}</dc:creator></metadata>
                <manifest><item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest>
                <spine><itemref idref="c1"/></spine>
            </package>"#
        );
        zw.write_all(opf.as_bytes()).unwrap();
        zw.start_file("OEBPS/ch1.xhtml", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"<html xmlns=\"http://www.w3.org/1999/xhtml\"><body><p>Text.</p></body></html>")
            .unwrap();
        zw.finish().unwrap();
    }

    #[test]
    fn library_store_roundtrip_progress_and_relocate() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let mut store = LibraryStore::default();
        store.reader_prefs.theme = ReaderThemePref::Amoled;
        store.reader_prefs.width = ReaderWidthPref::TwoColumn;
        store.update_progress("SciFi/Dune.epub", 3, 12, 0.5, false);
        assert_eq!(store.books["SciFi/Dune.epub"].status, ReadingStatus::Reading);
        assert!((store.books["SciFi/Dune.epub"].overall_pct - (3.5 / 12.0)).abs() < 0.001);
        store.save(root).unwrap();

        let mut loaded = LibraryStore::load(root);
        assert_eq!(loaded.reader_prefs.theme, ReaderThemePref::Amoled);
        assert_eq!(loaded.reader_prefs.width, ReaderWidthPref::TwoColumn);

        assert!(loaded.relocate_key(
            root,
            &root.join("SciFi/Dune.epub"),
            &root.join("SciFi/Dune 01.epub")
        ));
        assert!(!loaded.books.contains_key("SciFi/Dune.epub"));
        assert!(loaded.books.contains_key("SciFi/Dune 01.epub"));

        loaded.update_progress("SciFi/Dune 01.epub", 11, 12, 1.0, true);
        assert_eq!(
            loaded.books["SciFi/Dune 01.epub"].status,
            ReadingStatus::Finished
        );
        assert_eq!(loaded.books["SciFi/Dune 01.epub"].overall_pct, 1.0);
    }

    #[test]
    fn suggested_filename_handles_series_author_and_sanitization() {
        let s1 = EpubMetadata {
            title: "Children of Time?".into(),
            authors: vec!["Adrian Tchaikovsky".into()],
            series: Some("Children of Time".into()),
            series_index: Some(1.0),
            ..Default::default()
        };
        assert_eq!(
            suggested_book_filename(&s1, "epub"),
            "Children of Time 01 - Children of Time.epub"
        );

        let s2 = EpubMetadata {
            title: "Project Hail Mary".into(),
            authors: vec!["Andy Weir".into()],
            ..Default::default()
        };
        assert_eq!(
            suggested_book_filename(&s2, "epub"),
            "Andy Weir - Project Hail Mary.epub"
        );

        let s3 = EpubMetadata {
            title: "A/B:C*".into(),
            ..Default::default()
        };
        assert_eq!(suggested_book_filename(&s3, "pdf"), "A B C.pdf");
    }

    #[test]
    fn scan_library_discovers_epubs_and_pdfs_with_state() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("Fiction")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();

        write_minimal_epub(&root.join("Fiction/Sol.epub"), "Solaris", "Stanislaw Lem");
        fs::write(root.join("Manual.pdf"), b"%PDF-1.4").unwrap();
        write_minimal_epub(&root.join(".hidden/Ignored.epub"), "Hidden", "Nobody");

        let mut store = LibraryStore::default();
        store.update_progress("Fiction/Sol.epub", 1, 4, 0.25, false);
        store.books.get_mut("Fiction/Sol.epub").unwrap().rating = 5;

        let records = scan_library(root, &store);
        assert_eq!(records.len(), 2);
        let sol = records
            .iter()
            .find(|r| r.rel_path == "Fiction/Sol.epub")
            .expect("Solaris found");
        assert_eq!(sol.format, BookFormat::Epub);
        assert_eq!(sol.display_title(), "Solaris");
        assert_eq!(sol.authors_display(), "Stanislaw Lem");
        assert_eq!(sol.user.rating, 5);
        assert_eq!(sol.progress_pct(), 31);
    }

    #[test]
    fn book_highlight_block_range_and_store_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let hl_multi = BookHighlight {
            id: "hl-1".into(),
            chapter_idx: 2,
            chapter_title: "Part Three".into(),
            start_block: 1,
            start_offset: 5,
            end_block: 3,
            end_offset: 12,
            quote: "Example span across blocks".into(),
            color: HighlightColor::Blue,
            style: HighlightDecoration::Squiggly,
            note: "Important argument".into(),
            created_at: "2026-11-24T00:00:00Z".into(),
            updated_at: "2026-11-24T00:00:00Z".into(),
        };

        assert_eq!(hl_multi.block_byte_range(1, 1, 20), None);
        assert_eq!(hl_multi.block_byte_range(2, 0, 20), None);
        assert_eq!(hl_multi.block_byte_range(2, 1, 20), Some(5..20));
        assert_eq!(hl_multi.block_byte_range(2, 2, 18), Some(0..18));
        assert_eq!(hl_multi.block_byte_range(2, 3, 25), Some(0..12));
        assert_eq!(hl_multi.block_byte_range(2, 4, 25), None);

        let mut store = LibraryStore::default();
        store.reader_prefs.last_highlight_color = HighlightColor::Pink;
        store.reader_prefs.last_highlight_style = HighlightDecoration::Underline;
        store
            .books
            .entry("Books/Philosophy.epub".into())
            .or_default()
            .highlights
            .push(hl_multi.clone());
        store.save(root).unwrap();

        let loaded = LibraryStore::load(root);
        assert_eq!(loaded.reader_prefs.last_highlight_color, HighlightColor::Pink);
        assert_eq!(
            loaded.reader_prefs.last_highlight_style,
            HighlightDecoration::Underline
        );
        assert_eq!(loaded.books["Books/Philosophy.epub"].highlights, vec![hl_multi]);
    }

    #[test]
    fn word_bounds_and_char_boundary_handle_unicode() {
        let sample = "Café — \"quoted\" mind-killer’s test";
        assert_eq!(clamp_to_char_boundary(sample, 4), 3); // mid-é snaps back
        let r1 = word_bounds_at(sample, 2);
        assert_eq!(&sample[r1], "Café");
        let em_idx = sample.find("mind").unwrap() + 5;
        let r2 = word_bounds_at(sample, em_idx);
        assert_eq!(&sample[r2], "mind-killer’s");
    }

    #[test]
    fn syncthing_conflict_merge_unions_annotations_respects_tombstones_and_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let now = chrono::Utc::now();
        let ts_old = (now - chrono::Duration::hours(4)).to_rfc3339();
        let ts_mid = (now - chrono::Duration::hours(2)).to_rfc3339();
        let ts_new = (now - chrono::Duration::hours(1)).to_rfc3339();
        let ts_newer = (now - chrono::Duration::minutes(15)).to_rfc3339();

        let hl = |id: &str, ch: usize, blk: usize, off: usize, note: &str, created: &str, updated: &str| {
            BookHighlight {
                id: id.into(),
                chapter_idx: ch,
                chapter_title: format!("Ch {}", ch + 1),
                start_block: blk,
                start_offset: off,
                end_block: blk,
                end_offset: off + 10,
                quote: format!("quote-{id}"),
                color: HighlightColor::Yellow,
                style: HighlightDecoration::Highlight,
                note: note.into(),
                created_at: created.into(),
                updated_at: updated.into(),
            }
        };
        let bm = |id: &str, ch: usize, pct: f32, note: &str, created: &str| BookBookmark {
            id: id.into(),
            chapter_idx: ch,
            chapter_title: format!("Ch {}", ch + 1),
            scroll_pct: pct,
            note: note.into(),
            created_at: created.into(),
        };

        // Primary file on disk (e.g. desktop): BookA hl-shared (older), hl-desktop, hl-recreated (edited after tombstone),
        // deleted hl-gone (tombstone at ts_mid) and bm-gone (tombstone at ts_mid).
        let mut primary = LibraryStore::default();
        let book_a = primary.books.entry("BookA.epub".into()).or_default();
        book_a.status = ReadingStatus::Reading;
        book_a.chapter_idx = 2;
        book_a.total_chapters = 10;
        book_a.scroll_pct = 0.25;
        book_a.overall_pct = 0.225;
        book_a.last_opened = Some(ts_mid.clone());
        book_a.updated_at = Some(ts_mid.clone());
        book_a.highlights = vec![
            hl("hl-shared", 1, 2, 0, "short", &ts_old, &ts_old),
            hl("hl-desktop", 1, 5, 0, "desktop", &ts_mid, &ts_mid),
            hl("hl-recreated", 2, 1, 0, "resurrected-by-newer-edit", &ts_old, &ts_newer),
        ];
        book_a.deleted_highlights.insert("hl-gone".into(), ts_mid.clone());
        book_a.bookmarks = vec![bm("bm-desk", 2, 0.25, "d", &ts_mid)];
        book_a.deleted_bookmarks.insert("bm-gone".into(), ts_mid.clone());
        primary.save(root).unwrap();

        // Conflict 1 (phone): newer edit on hl-shared, hl-gone & bm-gone from ts_old (must NOT resurrect),
        // hl-phone, newer progress on BookA, plus BookB only on phone.
        let mut c1 = LibraryStore::default();
        let c1_a = c1.books.entry("BookA.epub".into()).or_default();
        c1_a.status = ReadingStatus::Reading;
        c1_a.chapter_idx = 5;
        c1_a.total_chapters = 10;
        c1_a.scroll_pct = 0.5;
        c1_a.overall_pct = 0.55;
        c1_a.last_opened = Some(ts_new.clone());
        c1_a.updated_at = Some(ts_new.clone());
        c1_a.highlights = vec![
            hl("hl-shared", 1, 2, 0, "richer phone note", &ts_old, &ts_new),
            hl("hl-gone", 0, 1, 0, "should stay deleted", &ts_old, &ts_old),
            hl("hl-phone", 4, 2, 0, "phone note", &ts_new, &ts_new),
        ];
        c1_a.bookmarks = vec![
            bm("bm-gone", 0, 0.1, "deleted bm", &ts_old),
            bm("bm-phone", 5, 0.5, "p", &ts_new),
        ];
        let c1_b = c1.books.entry("BookB.epub".into()).or_default();
        c1_b.highlights = vec![hl("hl-b1", 0, 0, 0, "from phone", &ts_new, &ts_new)];
        let c1_path = root.join(".vkarma-library.sync-conflict-20261125-091500-PHONE11.json");
        fs::write(&c1_path, serde_json::to_string_pretty(&c1).unwrap()).unwrap();

        // Conflict 2 (tablet): stale delete tombstone on hl-recreated (ts_mid < ts_newer -> survives!) + hl-tablet.
        let mut c2 = LibraryStore::default();
        let c2_a = c2.books.entry("BookA.epub".into()).or_default();
        c2_a.highlights = vec![hl("hl-tablet", 3, 0, 0, "tablet", &ts_mid, &ts_mid)];
        c2_a.deleted_highlights.insert("hl-recreated".into(), ts_mid.clone());
        let c2_path = root.join(".vkarma-library.sync-conflict-20261125-092000-TAB2222.json");
        fs::write(&c2_path, serde_json::to_string_pretty(&c2).unwrap()).unwrap();

        // Conflict 3: corrupt JSON must NOT abort merge and must remain untouched on disk for safety.
        let corrupt_path = root.join(".vkarma-library.sync-conflict-20261125-092500-CORRUPT.json");
        fs::write(&corrupt_path, b"{ truncated json ...").unwrap();

        let (merged, count) = LibraryStore::load_with_conflict_report(root);
        assert_eq!(count, 2);
        assert!(!c1_path.exists());
        assert!(!c2_path.exists());
        assert!(corrupt_path.exists());

        let merged_a = &merged.books["BookA.epub"];
        assert_eq!(merged_a.chapter_idx, 5);
        let hl_ids: Vec<&str> = merged_a.highlights.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(
            hl_ids,
            vec!["hl-shared", "hl-desktop", "hl-recreated", "hl-tablet", "hl-phone"]
        );
        assert_eq!(merged_a.highlights[0].note, "richer phone note");
        assert!(!merged_a.deleted_highlights.contains_key("hl-recreated"));
        assert!(merged_a.deleted_highlights.contains_key("hl-gone"));
        let bm_ids: Vec<&str> = merged_a.bookmarks.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(bm_ids, vec!["bm-desk", "bm-phone"]);
        assert!(merged.books.contains_key("BookB.epub"));
    }

    #[test]
    fn sync_save_merges_concurrent_disk_changes_without_losing_remote_highlights() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let now = chrono::Utc::now();
        let ts_old = (now - chrono::Duration::hours(2)).to_rfc3339();
        let ts_disk = (now - chrono::Duration::minutes(20)).to_rfc3339();
        let ts_local = (now - chrono::Duration::minutes(5)).to_rfc3339();

        let mut local = LibraryStore::load(root);
        let loc_a = local.books.entry("Dune.epub".into()).or_default();
        loc_a.rating = 2;
        loc_a.updated_at = Some(ts_old.clone());
        local.save(root).unwrap();

        // Simulate Syncthing writing a remote highlight onto disk while desktop holds an older store in memory.
        let mut on_disk = LibraryStore::load(root);
        let disk_a = on_disk.books.entry("Dune.epub".into()).or_default();
        disk_a.rating = 2;
        disk_a.updated_at = Some(ts_disk.clone());
        disk_a.highlights.push(BookHighlight {
            id: "hl-remote".into(),
            chapter_idx: 3,
            chapter_title: "Ch 4".into(),
            start_block: 1,
            start_offset: 0,
            end_block: 1,
            end_offset: 8,
            quote: "Fear is the mind-killer".into(),
            color: HighlightColor::Purple,
            style: HighlightDecoration::Squiggly,
            note: "remote".into(),
            created_at: ts_disk.clone(),
            updated_at: ts_disk,
        });
        on_disk.save(root).unwrap();

        // Local process now updates rating to 5 at ts_local and calls sync_save().
        let entry = local.books.entry("Dune.epub".into()).or_default();
        entry.rating = 5;
        entry.updated_at = Some(ts_local);
        let merged_conflicts = local.sync_save(root).unwrap();
        assert_eq!(merged_conflicts, 0);

        let final_store = LibraryStore::load(root);
        let final_a = &final_store.books["Dune.epub"];
        assert_eq!(final_a.rating, 5);
        assert_eq!(final_a.highlights.len(), 1);
        assert_eq!(final_a.highlights[0].id, "hl-remote");
    }

    #[test]
    fn compute_chapter_page_offsets_snaps_between_lines_and_clamps() {
        // Short chapter fits in 1 page
        let single = compute_chapter_page_offsets(600.0, 500.0, &[(20.0, 48.0)]);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].0, 0.0);

        // Multi-page chapter with a line straddling usable_h (600 - 14 = 586)
        // Band (570.0, 602.0) straddles 586.0 -> cut should snap cleanly to 570.0!
        let bands = vec![
            (20.0, 50.0),
            (540.0, 568.0),
            (570.0, 602.0),
            (1130.0, 1165.0),
        ];
        let pages = compute_chapter_page_offsets(600.0, 1300.0, &bands);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].0, 0.0);
        assert!((pages[0].1 - 570.0).abs() < 1e-3);
        assert!((pages[1].0 - 570.0).abs() < 1e-3);
        assert_eq!(page_index_for_content_y(&pages, 580.0), 1);

        // Zero / tiny viewport still produces 1 safe page
        assert_eq!(compute_chapter_page_offsets(0.0, 500.0, &[]).len(), 1);
    }

    #[test]
    fn reader_prefs_deserializes_legacy_json_without_pagination_or_full_width() {
        let legacy = r#"{"font_size":18.0,"theme":"sepia","width":"standard"}"#;
        let prefs: ReaderPrefs = serde_json::from_str(legacy).unwrap();
        assert_eq!(prefs.width, ReaderWidthPref::Standard);
        assert_eq!(prefs.pagination, ReaderPaginationPref::Page);
        assert_eq!(ReaderWidthPref::Full.max_width_px_opt(), None);
    }

    #[test]
    fn paginate_measured_blocks_splits_paragraphs_and_avoids_heading_orphans() {
        // 5-line paragraph split across pages after a heading + header
        let lines: Vec<Range<usize>> = (0..5).map(|i| (i * 20)..((i + 1) * 20)).collect();
        let blocks = vec![
            MeasuredBlock::Text {
                block_idx: 0,
                line_ranges: vec![0..10],
                line_height: 24.0,
                pad_top: 8.0,
                pad_bottom: 4.0,
                is_heading: true,
            },
            MeasuredBlock::Text {
                block_idx: 1,
                line_ranges: lines.clone(),
                line_height: 24.0,
                pad_top: 4.0,
                pad_bottom: 4.0,
                is_heading: false,
            },
            // Near end of page 1: a heading that shouldn't be orphaned at bottom
            MeasuredBlock::Text {
                block_idx: 2,
                line_ranges: vec![0..12],
                line_height: 28.0,
                pad_top: 8.0,
                pad_bottom: 4.0,
                is_heading: true,
            },
            MeasuredBlock::Fixed {
                block_idx: 3,
                height: 60.0,
            },
        ];

        // Page height 170, header 50, block gap 12:
        // Page 0: header (50) + blk0 (36) + gap (12) -> 72 left -> 2 lines of blk1 fit (4+48 = 52).
        let pages = paginate_measured_blocks(&blocks, 170.0, 50.0, 12.0);
        assert!(pages.len() >= 2);
        assert!(pages[0].show_chapter_header);
        assert!(!pages[1].show_chapter_header);
        // blk1 splits with continues_next -> is_continuation
        let first_blk1 = pages[0].slices.iter().find(|s| s.block_idx == 1).unwrap();
        assert!(!first_blk1.is_continuation);
        assert!(first_blk1.continues_next);
        assert_eq!(first_blk1.byte_range, 0..40);

        let cont_blk1 = pages[1].slices.iter().find(|s| s.block_idx == 1).unwrap();
        assert!(cont_blk1.is_continuation);
        assert_eq!(cont_blk1.byte_range.start, 40);

        // Heading 2 stays with blk 3 on the same page
        let p_h2 = page_index_for_block(&pages, 2, 0);
        let p_b3 = page_index_for_block(&pages, 3, 0);
        assert_eq!(p_h2, p_b3);
        assert_eq!(page_index_for_block(&pages, 1, 55), 1);

        // Empty chapter still produces 1 empty page
        let empty = paginate_measured_blocks(&[], 400.0, 60.0, 12.0);
        assert_eq!(empty.len(), 1);
    }
}
