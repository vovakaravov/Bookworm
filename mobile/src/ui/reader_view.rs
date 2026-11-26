use super::image_util::decode_bytes_image;
use super::{BookwormMobileApp, PageTouchMode, ReaderState, SelectionHandleKind};
use crate::theme::{ReaderPalette, ReaderTheme, hex_alpha, hex_color};
use bookworm_core::epub::{EpubBlock, EpubRun};
use bookworm_core::library::{
    BookHighlight, ChapterPage, HighlightColor, HighlightDecoration, PageSlice,
    ReaderPaginationPref, ReaderWidthPref, clamp_to_char_boundary,
};
use gpui::{prelude::*, *};
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn highlight_fill_alpha(color: HighlightColor, light_paper: bool) -> Hsla {
    let alpha = if light_paper { 0x5c } else { 0x48 };
    hex_alpha((color.rgb_hex() << 8) | alpha)
}

#[allow(clippy::too_many_arguments)]
pub fn flatten_block_highlights(
    text: &str,
    runs: &[EpubRun],
    block_hls: &[(&BookHighlight, Range<usize>)],
    sel_range: Option<Range<usize>>,
    code_bg: Hsla,
    code_fg: Hsla,
    sel_bg: Hsla,
    light_paper: bool,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut pts = Vec::with_capacity((runs.len() + block_hls.len() + 1) * 2);
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

        let in_sel = sel_range.as_ref().is_some_and(|sr| {
            let rs = clamp_to_char_boundary(text, sr.start);
            let re = clamp_to_char_boundary(text, sr.end);
            rs <= a && re >= b
        });

        if base_run.is_none() && matching_hls.is_empty() && !in_sel {
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

        if in_sel {
            style.background_color = Some(sel_bg);
        }

        out.push((a..b, style));
    }

    out
}

fn block_visual_weight(blk: &EpubBlock) -> usize {
    match blk {
        EpubBlock::Heading { text, .. }
        | EpubBlock::Paragraph { text, .. }
        | EpubBlock::ListItem { text, .. } => text.len() + 64,
        EpubBlock::Image { .. } => 420,
        EpubBlock::Rule => 24,
    }
}

pub fn split_blocks_two_columns(blocks: &[EpubBlock]) -> usize {
    if blocks.len() < 2 {
        return blocks.len();
    }
    let total: usize = blocks.iter().map(block_visual_weight).sum();
    let half = total / 2;
    let mut acc = 0usize;
    let mut split = 1usize;
    for (ix, blk) in blocks.iter().enumerate() {
        acc += block_visual_weight(blk);
        split = ix + 1;
        if acc >= half {
            break;
        }
    }
    split.clamp(1, blocks.len() - 1)
}

#[allow(clippy::too_many_arguments)]
fn render_paragraph_text(
    blk_idx: usize,
    text: String,
    runs: &[EpubRun],
    slice: Option<&PageSlice>,
    highlights: &[BookHighlight],
    sel_range: Option<Range<usize>>,
    chapter_idx: usize,
    code_bg: Hsla,
    code_fg: Hsla,
    sel_bg: Hsla,
    light_paper: bool,
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
) -> AnyElement {
    let full_len = text.len();
    let (base_off, disp_text, shifted_runs, shifted_sel): (
        usize,
        String,
        Vec<EpubRun>,
        Option<Range<usize>>,
    ) = if let Some(sl) = slice {
        let a = clamp_to_char_boundary(&text, sl.byte_range.start);
        let b = clamp_to_char_boundary(&text, sl.byte_range.end).max(a);
        let sub_text = text[a..b].to_string();
        let sub_runs = runs
            .iter()
            .filter_map(|r| {
                let rs = clamp_to_char_boundary(&text, r.range.start).max(a);
                let re = clamp_to_char_boundary(&text, r.range.end).min(b);
                (rs < re).then(|| EpubRun {
                    range: (rs - a)..(re - a),
                    bold: r.bold,
                    italic: r.italic,
                    underline: r.underline,
                    code: r.code,
                })
            })
            .collect();
        let sub_sel = sel_range.and_then(|sr| {
            let ss = clamp_to_char_boundary(&text, sr.start).max(a);
            let se = clamp_to_char_boundary(&text, sr.end).min(b);
            (ss < se).then_some((ss - a)..(se - a))
        });
        (a, sub_text, sub_runs, sub_sel)
    } else {
        (0, text, runs.to_vec(), sel_range)
    };

    let disp_len = disp_text.len();
    let end_off = base_off + disp_len;
    let block_hls: Vec<(&BookHighlight, Range<usize>)> = highlights
        .iter()
        .filter_map(|hl| {
            let r = hl.block_byte_range(chapter_idx, blk_idx, full_len)?;
            let rs = r.start.max(base_off);
            let re = r.end.min(end_off);
            (rs < re).then_some((hl, (rs - base_off)..(re - base_off)))
        })
        .collect();
    let flat = flatten_block_highlights(
        &disp_text,
        &shifted_runs,
        &block_hls,
        shifted_sel,
        code_bg,
        code_fg,
        sel_bg,
        light_paper,
    );
    let st = if flat.is_empty() {
        StyledText::new(disp_text)
    } else {
        StyledText::new(disp_text).with_highlights(flat)
    };
    if let Some(slot) = rendered_layouts {
        slot.borrow_mut()
            .push((blk_idx, base_off, disp_len, st.layout().clone()));
    }
    st.into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_chapter_block(
    b_ix: usize,
    blk: &EpubBlock,
    slice: Option<&PageSlice>,
    sel_range: Option<Range<usize>>,
    ch_idx: usize,
    font_size: f32,
    max_img_h: f32,
    pal: ReaderPalette,
    light_paper: bool,
    highlights: &[BookHighlight],
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
    decoded_images: &RefCell<HashMap<(usize, usize), Arc<Image>>>,
) -> AnyElement {
    let code_bg = hex_color(pal.surface);
    let code_fg = hex_color(pal.accent);
    let sel_bg = if light_paper {
        hex_alpha(0x3b82f64c)
    } else {
        hex_alpha(0x7e9cd858)
    };
    let is_cont = slice.is_some_and(|s| s.is_continuation);
    let cont_next = slice.is_some_and(|s| s.continues_next);
    match blk {
        EpubBlock::Heading { level, text } => {
            let size = match level {
                1 => (font_size + 7.0).min(34.0),
                2 => (font_size + 4.5).min(30.0),
                _ => (font_size + 2.0).min(26.0),
            };
            let lh = (size * 1.28).ceil();
            let rendered = render_paragraph_text(
                b_ix,
                text.clone(),
                &[],
                slice,
                highlights,
                sel_range,
                ch_idx,
                code_bg,
                code_fg,
                sel_bg,
                light_paper,
                rendered_layouts,
            );
            div()
                .id(SharedString::from(format!("blk-{b_ix}-{}", is_cont as u8)))
                .when(!is_cont, |d| d.pt_2())
                .when(!cont_next, |d| d.pb_1())
                .text_size(px(size))
                .line_height(px(lh))
                .font_weight(FontWeight::BOLD)
                .text_color(hex_color(pal.fg))
                .child(rendered)
                .into_any_element()
        }
        EpubBlock::Paragraph {
            text,
            runs,
            is_quote,
        } => {
            let lh = (font_size * 1.62).ceil();
            let rendered = render_paragraph_text(
                b_ix,
                text.clone(),
                runs,
                slice,
                highlights,
                sel_range,
                ch_idx,
                code_bg,
                code_fg,
                sel_bg,
                light_paper,
                rendered_layouts,
            );
            div()
                .id(SharedString::from(format!("blk-{b_ix}-{}", is_cont as u8)))
                .w_full()
                .px_1p5()
                .when(!is_cont, |d| d.pt_1())
                .when(!cont_next, |d| d.pb_1())
                .rounded(px(6.))
                .when(*is_quote, |el| {
                    el.pl_3()
                        .border_l_2()
                        .border_color(hex_color(pal.accent))
                        .bg(hex_color(pal.quote_bg))
                })
                .text_size(px(font_size))
                .line_height(px(lh))
                .text_color(hex_color(if *is_quote { pal.muted } else { pal.fg }))
                .child(rendered)
                .into_any_element()
        }
        EpubBlock::ListItem { text, runs } => {
            let lh = (font_size * 1.58).ceil();
            let rendered = render_paragraph_text(
                b_ix,
                text.clone(),
                runs,
                slice,
                highlights,
                sel_range,
                ch_idx,
                code_bg,
                code_fg,
                sel_bg,
                light_paper,
                rendered_layouts,
            );
            h_flex()
                .id(SharedString::from(format!("blk-{b_ix}-{}", is_cont as u8)))
                .w_full()
                .items_start()
                .gap_2()
                .pl_2()
                .when(!is_cont, |d| d.pt_0p5())
                .when(!cont_next, |d| d.pb_0p5())
                .rounded(px(6.))
                .text_size(px(font_size))
                .line_height(px(lh))
                .text_color(hex_color(pal.fg))
                .child(
                    div()
                        .w(px(10.))
                        .text_color(hex_color(pal.accent))
                        .child(if is_cont { " " } else { "•" }),
                )
                .child(div().flex_1().min_w_0().child(rendered))
                .into_any_element()
        }
        EpubBlock::Image {
            media_type,
            zip_path,
            bytes,
            ..
        } => {
            let decoded = if let Some(cached) = decoded_images.borrow().get(&(ch_idx, b_ix)).cloned()
            {
                Some(cached)
            } else if let Some(fresh) = decode_bytes_image(media_type, zip_path, bytes) {
                decoded_images
                    .borrow_mut()
                    .insert((ch_idx, b_ix), Arc::clone(&fresh));
                Some(fresh)
            } else {
                None
            };
            let img_h = 220.0_f32.min(max_img_h.max(120.0));
            div()
                .w_full()
                .h(px(img_h))
                .my_2()
                .flex()
                .items_center()
                .justify_center()
                .when_some(decoded, |box_el, image| {
                    box_el.child(img(image).size_full().object_fit(ObjectFit::Contain))
                })
                .into_any_element()
        }
        EpubBlock::Rule => div()
            .w_full()
            .h(px(1.))
            .my_2()
            .bg(hex_color(pal.border))
            .into_any_element(),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_page_spread_canvas(
    page: Option<&ChapterPage>,
    right_page: Option<&ChapterPage>,
    ch_blocks: &[EpubBlock],
    ch_idx: usize,
    use_two_col: bool,
    col_max_w: Option<f32>,
    max_img_h: f32,
    font_size: f32,
    pal: ReaderPalette,
    light_paper: bool,
    highlights: &[BookHighlight],
    selection_bounds: &[Option<Range<usize>>],
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
    decoded_images: &RefCell<HashMap<(usize, usize), Arc<Image>>>,
) -> AnyElement {
    let build_col = |pg_opt: Option<&ChapterPage>| {
        let mut col = v_flex().w_full().gap_3p5();
        if let Some(pg) = pg_opt {
            for sl in &pg.slices {
                if let Some(blk) = ch_blocks.get(sl.block_idx) {
                    let sel_r = if rendered_layouts.is_some() {
                        selection_bounds.get(sl.block_idx).cloned().flatten()
                    } else {
                        None
                    };
                    col = col.child(render_chapter_block(
                        sl.block_idx,
                        blk,
                        Some(sl),
                        sel_r,
                        ch_idx,
                        font_size,
                        max_img_h,
                        pal,
                        light_paper,
                        highlights,
                        rendered_layouts,
                        decoded_images,
                    ));
                }
            }
        }
        col
    };

    let body_inner = if use_two_col {
        h_flex()
            .size_full()
            .px_6()
            .py_4()
            .gap_6()
            .items_start()
            .child(v_flex().flex_1().min_w_0().child(build_col(page)))
            .child(div().w(px(1.)).self_stretch().bg(hex_color(pal.border)))
            .child(v_flex().flex_1().min_w_0().child(build_col(right_page)))
            .into_any_element()
    } else {
        v_flex()
            .size_full()
            .px_5()
            .py_4()
            .child(build_col(page))
            .into_any_element()
    };

    div()
        .size_full()
        .flex()
        .justify_center()
        .child(
            div()
                .w_full()
                .h_full()
                .when_some(col_max_w, |d, w| d.max_w(px(w)))
                .child(body_inner),
        )
        .into_any_element()
}

pub fn render_reader_view(
    app: &mut BookwormMobileApp,
    window: &mut Window,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let wide_viewport = f32::from(window.viewport_size().width) >= 700.0;
    let use_two_col = app.two_column_spread && wide_viewport;
    let is_page_mode = app.reader_pagination == ReaderPaginationPref::Page;
    let width_pref = app.reader_width;

    if is_page_mode {
        let should_pag = app
            .reader
            .as_ref()
            .is_some_and(|r| !r.show_toc && !r.show_notes);
        if should_pag {
            let vp_b = app
                .reader
                .as_ref()
                .map(|r| r.viewport_bounds.get())
                .unwrap_or_default();
            let vp_w = f32::from(vp_b.size.width);
            let vp_h = f32::from(vp_b.size.height);
            if vp_w > 100.0 && vp_h > 100.0 {
                app.ensure_reader_pages_and_sync(vp_w, vp_h, use_two_col, window);
            } else {
                let entity = cx.entity().clone();
                window.on_next_frame(move |_, cx| {
                    let _ = entity.update(cx, |_, cx| cx.notify());
                });
            }
        }
    }
    app.maybe_schedule_reader_scroll_save(cx);

    let col_max_w = if use_two_col {
        Some(1140.0)
    } else {
        width_pref.max_width_px_opt()
    };

    let Some(r) = &app.reader else {
        return div().size_full().into_any_element();
    };
    let book_rel = &r.book_rel;
    let book_title = &r.book_title;
    let book = &r.book;
    let ch_idx = r.chapter_idx.min(book.chapters.len().max(1).saturating_sub(1));
    let show_settings = &r.show_settings;
    let show_toc = &r.show_toc;
    let show_notes = &r.show_notes;
    let notes_color_filter = &r.notes_color_filter;
    let notes_only_filter = &r.notes_only_filter;
    let draft = &r.draft;
    let rendered_layouts = &r.rendered_layouts;
    let decoded_images = &r.decoded_images;
    let vp_slot = r.viewport_bounds.clone();
    let vp_bounds = r.viewport_bounds.get();
    let visual_swipe_x = r.visual_swipe_offset_x();
    let is_long_pressing = r.is_long_press_selecting;
    let handle_points = if !*show_toc && !*show_notes && !is_long_pressing {
        r.selection_handle_points(vp_bounds)
    } else {
        (None, None)
    };
    let (page_idx, total_pages) = if is_page_mode {
        let tp = r.chapter_pages.len().max(1);
        let cp = r.current_page.min(tp - 1);
        (cp, tp)
    } else {
        (0, 1)
    };
    let selection_bounds: Vec<Option<Range<usize>>> = book
        .chapters
        .get(ch_idx)
        .map(|c| {
            c.blocks
                .iter()
                .enumerate()
                .map(|(ix, b)| {
                    super::block_plain_text(b)
                        .and_then(|t| r.block_selection_range(ix, t.len()))
                })
                .collect()
        })
        .unwrap_or_default();

    let pal = app.reader_theme.palette();
    let light_paper = matches!(app.reader_theme, ReaderTheme::Sepia | ReaderTheme::Paper);
    let user_state = app
        .library_store
        .books
        .get(book_rel)
        .cloned()
        .unwrap_or_default();
    let total_ch = book.chapters.len().max(1);
    let chapter = book.chapters.get(ch_idx);
    let ch_title = chapter
        .map(|c| c.title.clone())
        .unwrap_or_else(|| book_title.clone());
    let is_bookmarked = user_state.bookmarks.iter().any(|b| b.chapter_idx == ch_idx);
    let ann_count = user_state.highlights.len() + user_state.bookmarks.len();

    let top_bar = h_flex()
        .w_full()
        .px_3()
        .py_2()
        .bg(hex_color(pal.surface))
        .border_b_1()
        .border_color(hex_color(pal.border))
        .items_center()
        .justify_between()
        .gap_1p5()
        .child(
            Button::new("rdr-back")
                .ghost()
                .small()
                .label("← Library")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.close_reader_to_library(window, cx);
                })),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(hex_color(pal.fg))
                .truncate()
                .child(ch_title.clone()),
        )
        .child(
            h_flex()
                .gap_1()
                .flex_shrink_0()
                .child(
                    Button::new("rdr-page-toggle")
                        .with_variant(if is_page_mode {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Secondary
                        })
                        .small()
                        .label(if is_page_mode { "Page" } else { "Scroll" })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_reader_pagination(cx);
                        })),
                )
                .when(wide_viewport, |row| {
                    row.child(
                        Button::new("rdr-two-col")
                            .with_variant(if app.two_column_spread {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Secondary
                            })
                            .small()
                            .label("2-Col")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_two_column_spread(cx);
                            })),
                    )
                })
                .child(
                    Button::new("rdr-toc")
                        .secondary()
                        .small()
                        .label("TOC")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(r) = &mut this.reader {
                                r.show_toc = !r.show_toc;
                                r.show_notes = false;
                                r.show_settings = false;
                            }
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("rdr-notes")
                        .secondary()
                        .small()
                        .label(format!("★{ann_count}"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(r) = &mut this.reader {
                                r.show_notes = !r.show_notes;
                                r.show_toc = false;
                                r.show_settings = false;
                            }
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("rdr-aa")
                        .secondary()
                        .small()
                        .label("Aa")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(r) = &mut this.reader {
                                r.show_settings = !r.show_settings;
                            }
                            cx.notify();
                        })),
                ),
        );

    let settings_strip = v_flex()
        .w_full()
        .px_3p5()
        .py_2p5()
        .gap_2()
        .bg(hex_color(pal.surface))
        .border_b_1()
        .border_color(hex_color(pal.border))
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .gap_2()
                .child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .child(
                            Button::new("rdr-font-dec")
                                .secondary()
                                .small()
                                .label("A-")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.adjust_reader_font_size(-1.5, cx);
                                })),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(hex_color(pal.fg))
                                .child(format!("{:.1}px", app.reader_font_size)),
                        )
                        .child(
                            Button::new("rdr-font-inc")
                                .secondary()
                                .small()
                                .label("A+")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.adjust_reader_font_size(1.5, cx);
                                })),
                        ),
                )
                .child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .child(
                            Button::new("rdr-mode-pill")
                                .with_variant(if is_page_mode {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .small()
                                .label(if is_page_mode { "Mode: Page" } else { "Mode: Scroll" })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_reader_pagination(cx);
                                })),
                        )
                        .child(
                            Button::new("rdr-bm-btn")
                                .with_variant(if is_bookmarked {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .small()
                                .label(if is_bookmarked { "🔖 Saved" } else { "🔖 Mark" })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.reader_toggle_bookmark(cx);
                                })),
                        ),
                ),
        )
        .child(
            h_flex()
                .w_full()
                .gap_1p5()
                .children(ReaderWidthPref::all_single_column().iter().copied().map(|rw| {
                    let active = width_pref == rw && !app.two_column_spread;
                    div()
                        .id(SharedString::from(format!("rw-{}", rw.label())))
                        .flex_1()
                        .py_1()
                        .rounded(px(7.))
                        .cursor_pointer()
                        .bg(hex_color(if active { pal.accent } else { pal.bg }))
                        .border_1()
                        .border_color(hex_color(if active { pal.accent } else { pal.border }))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(if active { 0x181619 } else { pal.fg }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.two_column_spread {
                                this.two_column_spread = false;
                                this.persist_mobile_config(cx);
                            }
                            this.set_reader_width(rw, cx);
                        }))
                        .child(rw.label())
                })),
        )
        .child(
            h_flex()
                .w_full()
                .gap_1p5()
                .children(ReaderTheme::all().iter().copied().map(|rt| {
                    let active = app.reader_theme == rt;
                    let rp = rt.palette();
                    div()
                        .id(SharedString::from(format!("rt-{}", rt.label())))
                        .flex_1()
                        .py_1p5()
                        .rounded(px(8.))
                        .cursor_pointer()
                        .bg(hex_color(rp.bg))
                        .border_1()
                        .border_color(hex_color(if active { pal.accent } else { rp.border }))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(rp.fg))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_reader_theme(rt, cx);
                        }))
                        .child(rt.label())
                })),
        );

    let main_area = if *show_toc {
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                v_flex()
                    .id("rdr-toc-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&app.toc_scroll)
                    .child(
                        v_flex()
                            .flex_none()
                            .w_full()
                            .p_3p5()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(hex_color(pal.fg))
                                    .child(format!("Table of Contents · {}", book.toc.len())),
                            )
                            .children(book.toc.iter().enumerate().map(|(ix, item)| {
                                let target_ch = item.chapter_idx;
                                let is_cur = target_ch == ch_idx;
                                let indent = (item.depth.saturating_sub(1) as f32) * 14.0;
                                div()
                                    .id(SharedString::from(format!("toc-row-{ix}")))
                                    .w_full()
                                    .min_h(px(46.))
                                    .pl(px(14.0 + indent))
                                    .pr_3()
                                    .py_2()
                                    .rounded(px(8.))
                                    .bg(hex_color(if is_cur {
                                        pal.quote_bg
                                    } else {
                                        pal.surface
                                    }))
                                    .border_1()
                                    .border_color(hex_color(if is_cur {
                                        pal.accent
                                    } else {
                                        pal.border
                                    }))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.reader_go_chapter(target_ch, window, cx);
                                    }))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(if is_cur {
                                                FontWeight::BOLD
                                            } else {
                                                FontWeight::NORMAL
                                            })
                                            .text_color(hex_color(pal.fg))
                                            .child(item.title.clone()),
                                    )
                            })),
                    ),
            )
            .child(Scrollbar::vertical(&app.toc_scroll))
            .into_any_element()
    } else if *show_notes {
        let active_color_filter = *notes_color_filter;
        let active_notes_only = *notes_only_filter;
        let mut filtered_hls: Vec<BookHighlight> = user_state
            .highlights
            .iter()
            .filter(|h| {
                if let Some(col) = active_color_filter
                    && h.color != col
                {
                    return false;
                }
                if active_notes_only && h.note.trim().is_empty() {
                    return false;
                }
                true
            })
            .cloned()
            .collect();
        filtered_hls.sort_by_key(|h| (h.chapter_idx, h.start_block, h.start_offset));

        let filter_bar = h_flex()
            .w_full()
            .gap_1p5()
            .items_center()
            .flex_wrap()
            .child(
                Button::new("rdr-notes-flt-all")
                    .with_variant(if active_color_filter.is_none() {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .xsmall()
                    .label("All")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(r) = &mut this.reader {
                            r.notes_color_filter = None;
                        }
                        this.notes_scroll.set_offset(point(px(0.), px(0.)));
                        cx.notify();
                    })),
            )
            .children(HighlightColor::all().iter().copied().map(|col| {
                let active = active_color_filter == Some(col);
                div()
                    .id(("rdr-notes-col", col as usize))
                    .w(px(22.))
                    .h(px(22.))
                    .rounded(px(999.))
                    .bg(hex_color(col.rgb_hex()))
                    .border_2()
                    .border_color(if active {
                        hex_color(pal.fg)
                    } else {
                        hex_color(pal.border)
                    })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(r) = &mut this.reader {
                            r.notes_color_filter = if active { None } else { Some(col) };
                        }
                        this.notes_scroll.set_offset(point(px(0.), px(0.)));
                        cx.notify();
                    }))
            }))
            .child(
                Button::new("rdr-notes-flt-notes")
                    .with_variant(if active_notes_only {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .xsmall()
                    .label("Notes Only")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(r) = &mut this.reader {
                            r.notes_only_filter = !r.notes_only_filter;
                        }
                        this.notes_scroll.set_offset(point(px(0.), px(0.)));
                        cx.notify();
                    })),
            );

        div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                v_flex()
                    .id("rdr-notes-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&app.notes_scroll)
                    .child(
                        v_flex()
                            .flex_none()
                            .w_full()
                            .p_3p5()
                            .gap_2p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(hex_color(pal.fg))
                                    .child(format!("Highlights, Notes & Bookmarks · {ann_count}")),
                            )
                            .when(!user_state.highlights.is_empty(), |col| {
                                col.child(filter_bar)
                            })
                            .when(
                                user_state.highlights.is_empty() && user_state.bookmarks.is_empty(),
                                |col| {
                                    col.child(
                                        div()
                                            .p_4()
                                            .rounded(px(10.))
                                            .bg(hex_color(pal.surface))
                                            .text_xs()
                                            .text_color(hex_color(pal.muted))
                                            .child("Hold and drag across text while reading to highlight passages in 5 Moon+ Reader colors and attach margin notes."),
                                    )
                                },
                            )
                            .when(
                                !user_state.highlights.is_empty()
                                    && filtered_hls.is_empty()
                                    && user_state.bookmarks.is_empty(),
                                |col| {
                                    col.child(
                                        div()
                                            .p_4()
                                            .rounded(px(10.))
                                            .bg(hex_color(pal.surface))
                                            .text_xs()
                                            .text_color(hex_color(pal.muted))
                                            .child("No highlights match this filter."),
                                    )
                                },
                            )
                            .children(user_state.bookmarks.iter().enumerate().map(|(ix, bm)| {
                                let bm_jump = bm.clone();
                                let bm_del_id = bm.id.clone();
                                let bm_rel_key = book_rel.clone();
                                let pct = (bm.scroll_pct.clamp(0.0, 1.0) * 100.0).round() as u8;
                                let bm_note = bm.note.trim().to_string();
                                h_flex()
                                    .id(SharedString::from(format!("bm-row-{ix}")))
                                    .w_full()
                                    .p_3()
                                    .gap_2()
                                    .items_center()
                                    .justify_between()
                                    .rounded(px(8.))
                                    .bg(hex_color(pal.surface))
                                    .border_1()
                                    .border_color(hex_color(pal.border))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.reader_jump_to_bookmark(&bm_jump, window, cx);
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
                                                    .text_color(hex_color(pal.fg))
                                                    .truncate()
                                                    .child(format!(
                                                        "🔖 Ch {} · {pct}% · {}",
                                                        bm.chapter_idx + 1,
                                                        bm.chapter_title
                                                    )),
                                            )
                                            .when(!bm_note.is_empty(), |col| {
                                                col.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(hex_color(pal.muted))
                                                        .truncate()
                                                        .child(bm_note),
                                                )
                                            }),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!("bm-del-{ix}")))
                                            .ghost()
                                            .xsmall()
                                            .label("✕")
                                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                cx.stop_propagation();
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.library_store
                                                    .delete_bookmark(&bm_rel_key, &bm_del_id);
                                                let _ = this
                                                    .library_store
                                                    .sync_save(&this.library_state_root);
                                                cx.notify();
                                            })),
                                    )
                            }))
                            .children(filtered_hls.into_iter().enumerate().map(|(ix, hl)| {
                                let hl_jump = hl.clone();
                                let color_hex = hl.color.rgb_hex();
                                let hl_id = hl.id.clone();
                                let rel_key = book_rel.clone();
                                v_flex()
                                    .id(SharedString::from(format!("hl-item-{ix}")))
                                    .w_full()
                                    .p_3()
                                    .gap_1p5()
                                    .rounded(px(10.))
                                    .bg(hex_color(pal.surface))
                                    .border_l_4()
                                    .border_color(hex_color(color_hex))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.reader_jump_to_highlight(&hl_jump, window, cx);
                                    }))
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_size(px(10.5))
                                                    .text_color(hex_color(pal.muted))
                                                    .child(format!(
                                                        "Ch {} · {} · {}",
                                                        hl.chapter_idx + 1,
                                                        hl.color.label(),
                                                        hl.style.label()
                                                    )),
                                            )
                                            .child(
                                                Button::new(SharedString::from(format!(
                                                    "hl-del-{ix}"
                                                )))
                                                .ghost()
                                                .xsmall()
                                                .label("✕")
                                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                    cx.stop_propagation();
                                                })
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    cx.stop_propagation();
                                                    this.library_store
                                                        .delete_highlight(&rel_key, &hl_id);
                                                    let _ = this
                                                        .library_store
                                                        .sync_save(&this.library_state_root);
                                                    cx.notify();
                                                })),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(hex_color(pal.fg))
                                            .child(hl.quote.clone()),
                                    )
                                    .when(!hl.note.trim().is_empty(), |card| {
                                        card.child(
                                            div()
                                                .p_2()
                                                .rounded(px(6.))
                                                .bg(hex_color(pal.quote_bg))
                                                .text_xs()
                                                .text_color(hex_color(color_hex))
                                                .child(hl.note.clone()),
                                        )
                                    })
                            })),
                    ),
            )
            .child(Scrollbar::vertical(&app.notes_scroll))
            .into_any_element()
    } else {
        rendered_layouts.borrow_mut().clear();
        let vp_w_cur = f32::from(vp_bounds.size.width).max(300.0);
        let vp_h_cur = f32::from(vp_bounds.size.height).max(200.0);
        let max_img_h = (vp_h_cur - 32.0) * 0.68;

        if is_page_mode {
            let step = if use_two_col { 2usize } else { 1usize };
            let cur_left_pg = r.chapter_pages.get(page_idx);
            let cur_right_pg = use_two_col
                .then(|| r.chapter_pages.get(page_idx + 1))
                .flatten();
            let cur_blocks: &[EpubBlock] = chapter.map(|c| c.blocks.as_slice()).unwrap_or(&[]);

            let cur_page_el = render_page_spread_canvas(
                cur_left_pg,
                cur_right_pg,
                cur_blocks,
                ch_idx,
                use_two_col,
                col_max_w,
                max_img_h,
                app.reader_font_size,
                pal,
                light_paper,
                &user_state.highlights,
                &selection_bounds,
                Some(rendered_layouts),
                decoded_images,
            );

            // Adjacent real page preview during active swipe or flip animation
            let adj_layer: Option<(bool, AnyElement)> = if visual_swipe_x < -0.5 {
                let next_el = if page_idx + step < total_pages {
                    Some(render_page_spread_canvas(
                        r.chapter_pages.get(page_idx + step),
                        use_two_col
                            .then(|| r.chapter_pages.get(page_idx + step + 1))
                            .flatten(),
                        cur_blocks,
                        ch_idx,
                        use_two_col,
                        col_max_w,
                        max_img_h,
                        app.reader_font_size,
                        pal,
                        light_paper,
                        &user_state.highlights,
                        &[],
                        None,
                        decoded_images,
                    ))
                } else if ch_idx + 1 < book.chapters.len() {
                    let nch = ch_idx + 1;
                    let nblocks = book
                        .chapters
                        .get(nch)
                        .map(|c| c.blocks.as_slice())
                        .unwrap_or(&[]);
                    let npages = r.adjacent_pages_cache.get(&nch);
                    Some(render_page_spread_canvas(
                        npages.and_then(|v| v.first()),
                        use_two_col
                            .then(|| npages.and_then(|v| v.get(1)))
                            .flatten(),
                        nblocks,
                        nch,
                        use_two_col,
                        col_max_w,
                        max_img_h,
                        app.reader_font_size,
                        pal,
                        light_paper,
                        &user_state.highlights,
                        &[],
                        None,
                        decoded_images,
                    ))
                } else {
                    None
                };
                next_el.map(|el| (true, el))
            } else if visual_swipe_x > 0.5 {
                let prev_el = if page_idx >= step {
                    let p_ix = page_idx - step;
                    Some(render_page_spread_canvas(
                        r.chapter_pages.get(p_ix),
                        use_two_col.then(|| r.chapter_pages.get(p_ix + 1)).flatten(),
                        cur_blocks,
                        ch_idx,
                        use_two_col,
                        col_max_w,
                        max_img_h,
                        app.reader_font_size,
                        pal,
                        light_paper,
                        &user_state.highlights,
                        &[],
                        None,
                        decoded_images,
                    ))
                } else if ch_idx > 0 {
                    let pch = ch_idx - 1;
                    let pblocks = book
                        .chapters
                        .get(pch)
                        .map(|c| c.blocks.as_slice())
                        .unwrap_or(&[]);
                    let ppages = r.adjacent_pages_cache.get(&pch);
                    let last_ix = ppages.map(|v| {
                        let raw = v.len().saturating_sub(1);
                        if step == 2 { raw - (raw % 2) } else { raw }
                    });
                    Some(render_page_spread_canvas(
                        ppages.and_then(|v| last_ix.and_then(|ix| v.get(ix))),
                        use_two_col
                            .then(|| ppages.and_then(|v| last_ix.and_then(|ix| v.get(ix + 1))))
                            .flatten(),
                        pblocks,
                        pch,
                        use_two_col,
                        col_max_w,
                        max_img_h,
                        app.reader_font_size,
                        pal,
                        light_paper,
                        &user_state.highlights,
                        &[],
                        None,
                        decoded_images,
                    ))
                } else {
                    None
                };
                prev_el.map(|el| (false, el))
            } else {
                None
            };

            let app_weak_page = cx.entity().downgrade();
            let vp_measure = vp_slot.clone();
            let page_touch_canvas = canvas(
                move |bounds, _, _| vp_measure.set(bounds),
                move |bounds, _, window, _cx| {
                    let weak_td = app_weak_page.clone();
                    window.on_mouse_event(move |ev: &TouchDragEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        if ev.phase == TouchPhase::Started && !bounds.contains(&ev.start_position) {
                            return;
                        }
                        let _ = weak_td.update(cx, |app, cx| {
                            let is_reading = app
                                .reader
                                .as_ref()
                                .is_some_and(|r| !r.show_toc && !r.show_notes);
                            if !is_reading || app.reader_pagination != ReaderPaginationPref::Page {
                                return;
                            }
                            window.prevent_default();
                            cx.stop_propagation();

                            let last_c = app.library_store.reader_prefs.last_highlight_color;
                            let last_s = app.library_store.reader_prefs.last_highlight_style;
                            let vp_w = f32::from(bounds.size.width).max(300.0);

                            match ev.phase {
                                TouchPhase::Started => {
                                    // Flush any in-flight page commit if user starts a rapid next swipe
                                    if app.reader.as_ref().is_some_and(|r| r.flip_anim.is_some()) {
                                        app.flush_pending_page_flip(window, cx);
                                    }
                                    let Some(r) = &mut app.reader else {
                                        return;
                                    };
                                    let (s_pt, e_pt) = r.selection_handle_points(bounds);
                                    let hit_radius_sq = 36.0_f32 * 36.0;
                                    let near = |pt_opt: Option<Point<Pixels>>| -> Option<f32> {
                                        let p = pt_opt?;
                                        let dx = f32::from(ev.start_position.x - p.x);
                                        let dy = f32::from(ev.start_position.y - p.y);
                                        let d2 = dx * dx + dy * dy;
                                        (d2 <= hit_radius_sq).then_some(d2)
                                    };
                                    let chosen_handle = match (near(s_pt), near(e_pt)) {
                                        (Some(a), Some(b)) => Some(if a <= b {
                                            SelectionHandleKind::Start
                                        } else {
                                            SelectionHandleKind::End
                                        }),
                                        (Some(_), None) => Some(SelectionHandleKind::Start),
                                        (None, Some(_)) => Some(SelectionHandleKind::End),
                                        (None, None) => None,
                                    };
                                    if let Some(kind) = chosen_handle {
                                        r.dragging_handle = Some(kind);
                                        r.page_touch_mode = Some(PageTouchMode::HandleDrag(kind));
                                        r.page_touch_gen = r.page_touch_gen.wrapping_add(1);
                                        r._page_lp_task = None;
                                        cx.notify();
                                        return;
                                    }

                                    r.page_touch_mode = Some(PageTouchMode::Pending);
                                    r.page_touch_gen = r.page_touch_gen.wrapping_add(1);
                                    let touch_gen = r.page_touch_gen;
                                    let start_pos = ev.start_position;
                                    r.page_touch_start_x = f32::from(start_pos.x);
                                    r.page_touch_last_x = f32::from(start_pos.x);
                                    r.page_touch_last_at = Some(Instant::now());
                                    r.page_touch_vel_x = 0.0;
                                    r.swipe_offset_x = 0.0;
                                    r.swipe_active = false;
                                    r.flip_anim = None;
                                    r.mouse_down_pos = Some(start_pos);

                                    let book_rel = r.book_rel.clone();
                                    let lp_bounds = bounds;
                                    r._page_lp_task = Some(cx.spawn(async move |this, cx| {
                                        cx.background_executor()
                                            .timer(Duration::from_millis(410))
                                            .await;
                                        let _ = this.update(cx, |app, cx| {
                                            let Some(r) = &mut app.reader else {
                                                return;
                                            };
                                            if r.book_rel != book_rel
                                                || r.page_touch_gen != touch_gen
                                                || r.page_touch_mode != Some(PageTouchMode::Pending)
                                            {
                                                return;
                                            }
                                            if r.select_word_at_point(start_pos, Some(lp_bounds)) {
                                                r.page_touch_mode =
                                                    Some(PageTouchMode::LongPressSelecting);
                                                r.is_long_press_selecting = true;
                                                r.swipe_active = false;
                                                r.swipe_offset_x = 0.0;
                                                cx.notify();
                                            }
                                        });
                                    }));
                                }
                                TouchPhase::Moved => {
                                    let Some(r) = &mut app.reader else {
                                        return;
                                    };
                                    match r.page_touch_mode {
                                        Some(PageTouchMode::HandleDrag(kind)) => {
                                            if r.update_handle_drag_at(
                                                kind,
                                                ev.position,
                                                Some(bounds),
                                                last_c,
                                                last_s,
                                            ) {
                                                cx.notify();
                                            }
                                        }
                                        Some(PageTouchMode::LongPressSelecting) => {
                                            if let Some(hit) =
                                                r.hit_test_position(ev.position, Some(bounds))
                                                && r.selection_head != Some(hit)
                                            {
                                                r.selection_head = Some(hit);
                                                cx.notify();
                                            }
                                        }
                                        Some(PageTouchMode::Pending) => {
                                            let dx = f32::from(ev.position.x - ev.start_position.x);
                                            let dy = f32::from(ev.position.y - ev.start_position.y);
                                            if dx.abs() >= 9.0 && dx.abs() >= dy.abs() * 0.55 {
                                                r.page_touch_mode =
                                                    Some(PageTouchMode::HorizontalSwipe);
                                                r.page_touch_gen = r.page_touch_gen.wrapping_add(1);
                                                r._page_lp_task = None;
                                                r.swipe_active = true;
                                                let can_prev =
                                                    r.current_page > 0 || r.chapter_idx > 0;
                                                let can_next = r.current_page + step
                                                    < r.chapter_pages.len().max(1)
                                                    || r.chapter_idx + 1 < r.book.chapters.len();
                                                let eff = if (dx > 0.0 && !can_prev)
                                                    || (dx < 0.0 && !can_next)
                                                {
                                                    dx * 0.25
                                                } else {
                                                    dx
                                                };
                                                r.swipe_offset_x =
                                                    eff.clamp(-vp_w * 0.96, vp_w * 0.96);
                                                r.page_touch_last_x = f32::from(ev.position.x);
                                                r.page_touch_last_at = Some(Instant::now());
                                                cx.notify();
                                            } else if dy.abs() >= 18.0 && dx.abs() < 6.0 {
                                                // Finger drifted vertically without intending long-press
                                                r.page_touch_gen = r.page_touch_gen.wrapping_add(1);
                                                r._page_lp_task = None;
                                            }
                                        }
                                        Some(PageTouchMode::HorizontalSwipe) => {
                                            let now = Instant::now();
                                            let cur_x = f32::from(ev.position.x);
                                            let step_dx = cur_x - r.page_touch_last_x;
                                            if let Some(prev_t) = r.page_touch_last_at {
                                                let dt = now
                                                    .duration_since(prev_t)
                                                    .as_secs_f32()
                                                    .max(0.004);
                                                let inst_v = step_dx / dt;
                                                r.page_touch_vel_x =
                                                    r.page_touch_vel_x * 0.55 + inst_v * 0.45;
                                            }
                                            r.page_touch_last_x = cur_x;
                                            r.page_touch_last_at = Some(now);

                                            let can_prev =
                                                r.current_page > 0 || r.chapter_idx > 0;
                                            let can_next = r.current_page + step
                                                < r.chapter_pages.len().max(1)
                                                || r.chapter_idx + 1 < r.book.chapters.len();
                                            let next_raw = r.swipe_offset_x + step_dx;
                                            let eff_dx = if (next_raw > 0.0 && !can_prev)
                                                || (next_raw < 0.0 && !can_next)
                                            {
                                                step_dx * 0.25
                                            } else {
                                                step_dx
                                            };
                                            r.swipe_offset_x = (r.swipe_offset_x + eff_dx)
                                                .clamp(-vp_w * 0.96, vp_w * 0.96);
                                            cx.notify();
                                        }
                                        None => {}
                                    }
                                }
                                TouchPhase::Ended | TouchPhase::Cancelled => {
                                    let is_ended = ev.phase == TouchPhase::Ended;
                                    let (mode, final_off, vel_x) = {
                                        let Some(r) = &mut app.reader else {
                                            return;
                                        };
                                        r.page_touch_gen = r.page_touch_gen.wrapping_add(1);
                                        r._page_lp_task = None;
                                        let mode = r.page_touch_mode.take();
                                        let off = r.swipe_offset_x;
                                        let v = r.page_touch_vel_x;
                                        r.swipe_offset_x = 0.0;
                                        r.swipe_active = false;
                                        (mode, off, v)
                                    };
                                    match mode {
                                        Some(PageTouchMode::HandleDrag(_)) => {
                                            if let Some(r) = &mut app.reader {
                                                r.dragging_handle = None;
                                            }
                                            cx.notify();
                                        }
                                        Some(PageTouchMode::LongPressSelecting) => {
                                            if let Some(r) = &mut app.reader {
                                                r.is_long_press_selecting = false;
                                            }
                                            app.reader_finalize_selection_draft(window, cx);
                                        }
                                        Some(PageTouchMode::HorizontalSwipe) => {
                                            let dist_thresh = (vp_w * 0.16).clamp(44.0, 92.0);
                                            let commit_next = is_ended
                                                && (final_off <= -dist_thresh
                                                    || (vel_x <= -360.0 && final_off <= -14.0));
                                            let commit_prev = is_ended
                                                && (final_off >= dist_thresh
                                                    || (vel_x >= 360.0 && final_off >= 14.0));
                                            if commit_next {
                                                app.reader_next_page_or_chapter(
                                                    Some(final_off),
                                                    use_two_col,
                                                    window,
                                                    cx,
                                                );
                                            } else if commit_prev {
                                                app.reader_prev_page_or_chapter(
                                                    Some(final_off),
                                                    use_two_col,
                                                    window,
                                                    cx,
                                                );
                                            } else if final_off.abs() > 1.0 {
                                                app.reader_start_page_flip_anim(
                                                    final_off, 0.0, 0, 0, 160, cx,
                                                );
                                            } else {
                                                cx.notify();
                                            }
                                        }
                                        Some(PageTouchMode::Pending) if is_ended => {
                                            if let Some(r) = &mut app.reader {
                                                let down = r.mouse_down_pos.take();
                                                let moved_far = down.is_some_and(|p| {
                                                    let dx = f32::from(ev.position.x - p.x);
                                                    let dy = f32::from(ev.position.y - p.y);
                                                    dx * dx + dy * dy > 64.0
                                                });
                                                if !moved_far {
                                                    let hit_pos =
                                                        r.hit_test_position(ev.position, Some(bounds));
                                                    let existing_hl = hit_pos.and_then(|pos| {
                                                        let u = app
                                                            .library_store
                                                            .books
                                                            .get(&r.book_rel)?;
                                                        r.highlight_at_pos(&u.highlights, pos)
                                                            .cloned()
                                                    });
                                                    if let Some(hl) = existing_hl {
                                                        app.reader_open_existing_highlight(
                                                            &hl, window, cx,
                                                        );
                                                        return;
                                                    }
                                                    if r.draft.is_some()
                                                        || r.normalized_selection().is_some()
                                                    {
                                                        r.clear_selection_and_draft();
                                                        window.blur(cx);
                                                        gpui_mobile::hide_keyboard();
                                                        cx.notify();
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        });
                    });
                },
            )
            .size_full();

            let (start_handle_pt, end_handle_pt) = handle_points;
            let handle_overlay = |pt_opt: Option<Point<Pixels>>, is_start: bool| -> Option<AnyElement> {
                let pt = pt_opt?;
                let rel_x = (f32::from(pt.x - vp_bounds.origin.x) - 8.0)
                    .clamp(2.0, (f32::from(vp_bounds.size.width) - 18.0).max(2.0));
                let rel_y = (f32::from(pt.y - vp_bounds.origin.y) + 2.0)
                    .clamp(2.0, (f32::from(vp_bounds.size.height) - 24.0).max(2.0));
                Some(
                    div()
                        .absolute()
                        .left(px(rel_x))
                        .top(px(rel_y))
                        .w(px(16.))
                        .h(px(22.))
                        .rounded(px(8.))
                        .bg(hex_color(pal.accent))
                        .border_2()
                        .border_color(hex_color(0xffffff))
                        .when(is_start, |d| d.rounded_tr(px(2.)))
                        .when(!is_start, |d| d.rounded_tl(px(2.)))
                        .into_any_element(),
                )
            };

            let progress = (visual_swipe_x.abs() / vp_w_cur).clamp(0.0, 1.0);
            let spine_overlay = |edge_x: f32, veil_alpha: u32| -> AnyElement {
                div()
                    .id("rdr-flip-shadow")
                    .absolute()
                    .inset_0()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(edge_x.clamp(0.0, vp_w_cur)))
                            .w(px(22.))
                            .bg(hex_alpha(veil_alpha)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(edge_x.clamp(0.0, vp_w_cur)))
                            .w(px(1.5))
                            .bg(hex_color(pal.border)),
                    )
                    .into_any_element()
            };

            let layered_pages = match adj_layer {
                // Turning forward: next page rests beneath while current page peels left
                Some((true, next_el)) => {
                    let under_dim = (((1.0 - progress) * 0.14) * 255.0) as u32;
                    let edge_x = (vp_w_cur + visual_swipe_x).clamp(0.0, vp_w_cur);
                    let sh_alpha = (((0.26 - progress * 0.14).max(0.06)) * 255.0) as u32;
                    div()
                        .relative()
                        .size_full()
                        .child(div().absolute().inset_0().bg(hex_color(pal.bg)).child(next_el))
                        .when(under_dim > 2, |d| {
                            d.child(div().absolute().inset_0().bg(hex_alpha(0x00000000 | under_dim)))
                        })
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(px(visual_swipe_x))
                                .w(px(vp_w_cur))
                                .bg(hex_color(pal.bg))
                                .child(cur_page_el),
                        )
                        .child(spine_overlay(edge_x, sh_alpha))
                        .into_any_element()
                }
                // Turning backward: current page rests beneath while previous page slides in from left
                Some((false, prev_el)) => {
                    let under_dim = ((progress * 0.14) * 255.0) as u32;
                    let slide_x = visual_swipe_x - vp_w_cur;
                    let edge_x = visual_swipe_x.clamp(0.0, vp_w_cur);
                    let sh_alpha = (((0.26 - progress * 0.14).max(0.06)) * 255.0) as u32;
                    div()
                        .relative()
                        .size_full()
                        .child(div().absolute().inset_0().bg(hex_color(pal.bg)).child(cur_page_el))
                        .when(under_dim > 2, |d| {
                            d.child(div().absolute().inset_0().bg(hex_alpha(0x00000000 | under_dim)))
                        })
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(px(slide_x))
                                .w(px(vp_w_cur))
                                .bg(hex_color(pal.bg))
                                .child(prev_el),
                        )
                        .child(spine_overlay(edge_x, sh_alpha))
                        .into_any_element()
                }
                None => div()
                    .relative()
                    .size_full()
                    .bg(hex_color(pal.bg))
                    .child(cur_page_el)
                    .into_any_element(),
            };

            div()
                .id("rdr-viewport-shell")
                .relative()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                        if let Some(r) = &mut this.reader {
                            if r.show_toc || r.show_notes || r.page_touch_mode.is_some() {
                                return;
                            }
                            r.mouse_down_pos = Some(ev.position);
                            if ev.click_count == 2 {
                                let b = r.viewport_bounds.get();
                                if r.select_word_at_point(ev.position, Some(b)) {
                                    r.is_mouse_selecting = true;
                                    cx.notify();
                                }
                            }
                        }
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, ev: &MouseUpEvent, window, cx| {
                        if let Some(r) = &mut this.reader {
                            if r.show_toc
                                || r.show_notes
                                || r.is_long_press_selecting
                                || r.page_touch_mode.is_some()
                            {
                                return;
                            }
                            if r.is_mouse_selecting {
                                r.is_mouse_selecting = false;
                                this.reader_finalize_selection_draft(window, cx);
                                return;
                            }
                            let b = r.viewport_bounds.get();
                            let hit_pos = r.hit_test_position(ev.position, Some(b));
                            let existing_hl = hit_pos.and_then(|pos| {
                                let user = this.library_store.books.get(&r.book_rel)?;
                                r.highlight_at_pos(&user.highlights, pos).cloned()
                            });
                            if let Some(hl) = existing_hl {
                                this.reader_open_existing_highlight(&hl, window, cx);
                                return;
                            }
                            if r.draft.is_some() || r.normalized_selection().is_some() {
                                r.clear_selection_and_draft();
                                window.blur(cx);
                                gpui_mobile::hide_keyboard();
                                cx.notify();
                            }
                        }
                    }),
                )
                .child(layered_pages)
                .child(div().absolute().inset_0().child(page_touch_canvas))
                .when_some(handle_overlay(start_handle_pt, true), |el, h| el.child(h))
                .when_some(handle_overlay(end_handle_pt, false), |el, h| el.child(h))
                .into_any_element()
        } else {
            let body_inner = if let Some(ch) = chapter {
                if use_two_col && ch.blocks.len() >= 2 {
                    let split_idx = split_blocks_two_columns(&ch.blocks);
                    let mut left_col = v_flex().flex_1().min_w_0().gap_3p5();
                    let mut right_col = v_flex().flex_1().min_w_0().gap_3p5();
                    for (b_ix, blk) in ch.blocks.iter().enumerate() {
                        let sel_r = selection_bounds.get(b_ix).cloned().flatten();
                        let el = render_chapter_block(
                            b_ix,
                            blk,
                            None,
                            sel_r,
                            ch_idx,
                            app.reader_font_size,
                            220.0,
                            pal,
                            light_paper,
                            &user_state.highlights,
                            Some(rendered_layouts),
                            decoded_images,
                        );
                        if b_ix < split_idx {
                            left_col = left_col.child(el);
                        } else {
                            right_col = right_col.child(el);
                        }
                    }
                    h_flex()
                        .flex_none()
                        .w_full()
                        .px_6()
                        .py_4()
                        .gap_6()
                        .items_start()
                        .child(left_col)
                        .child(div().w(px(1.)).min_h(px(240.)).bg(hex_color(pal.border)))
                        .child(right_col)
                        .into_any_element()
                } else {
                    let mut col = v_flex().flex_none().w_full().px_5().py_4().gap_3p5();
                    for (b_ix, blk) in ch.blocks.iter().enumerate() {
                        let sel_r = selection_bounds.get(b_ix).cloned().flatten();
                        col = col.child(render_chapter_block(
                            b_ix,
                            blk,
                            None,
                            sel_r,
                            ch_idx,
                            app.reader_font_size,
                            220.0,
                            pal,
                            light_paper,
                            &user_state.highlights,
                            Some(rendered_layouts),
                            decoded_images,
                        ));
                    }
                    col.into_any_element()
                }
            } else {
                v_flex().flex_none().w_full().px_5().py_4().into_any_element()
            };

            let body_el = div()
                .flex_none()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .when_some(col_max_w, |d, w| d.max_w(px(w)))
                        .child(body_inner),
                )
                .into_any_element();

            let app_weak_gesture = cx.entity().downgrade();
            let vp_measure = vp_slot.clone();
            let entity_for_cap = cx.entity().clone();
            let gesture_canvas = canvas(
                move |bounds, _, _| vp_measure.set(bounds),
                move |bounds, _, window, _cx| {
                    let weak_lp = app_weak_gesture.clone();
                    let cap_ent = entity_for_cap.clone();
                    window.on_mouse_event(move |ev: &LongPressEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        let _ = weak_lp.update(cx, |app, cx| {
                            let is_reading = app
                                .reader
                                .as_ref()
                                .is_some_and(|r| !r.show_toc && !r.show_notes);
                            if !is_reading {
                                return;
                            }
                            match ev.phase {
                                TouchPhase::Started => {
                                    if !bounds.contains(&ev.start_position) {
                                        return;
                                    }
                                    let Some(r) = &mut app.reader else {
                                        return;
                                    };
                                    if !r.select_word_at_point(ev.start_position, Some(bounds)) {
                                        return;
                                    }
                                    r.is_long_press_selecting = true;
                                    r.swipe_active = false;
                                    r.swipe_offset_x = 0.0;
                                    window.capture_long_press(&cap_ent);
                                    window.prevent_default();
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                                TouchPhase::Moved => {
                                    let Some(r) = &mut app.reader else {
                                        return;
                                    };
                                    if !r.is_long_press_selecting {
                                        return;
                                    }
                                    window.prevent_default();
                                    cx.stop_propagation();
                                    if let Some(hit) = r.hit_test_position(ev.position, Some(bounds))
                                        && r.selection_head != Some(hit)
                                    {
                                        r.selection_head = Some(hit);
                                        cx.notify();
                                    }
                                }
                                TouchPhase::Ended | TouchPhase::Cancelled => {
                                    let Some(r) = &mut app.reader else {
                                        return;
                                    };
                                    if !r.is_long_press_selecting {
                                        return;
                                    }
                                    r.is_long_press_selecting = false;
                                    window.prevent_default();
                                    cx.stop_propagation();
                                    app.reader_finalize_selection_draft(window, cx);
                                }
                            }
                        });
                    });

                    let weak_td = app_weak_gesture.clone();
                    window.on_mouse_event(move |ev: &TouchDragEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        let _ = weak_td.update(cx, |app, cx| {
                            let last_c = app.library_store.reader_prefs.last_highlight_color;
                            let last_s = app.library_store.reader_prefs.last_highlight_style;
                            let Some(r) = &mut app.reader else {
                                return;
                            };
                            if r.show_toc || r.show_notes {
                                return;
                            }
                            match ev.phase {
                                TouchPhase::Started => {
                                    let (s_pt, e_pt) = r.selection_handle_points(bounds);
                                    let hit_radius_sq = 32.0_f32 * 32.0;
                                    let near = |pt_opt: Option<Point<Pixels>>| -> Option<f32> {
                                        let p = pt_opt?;
                                        let dx = f32::from(ev.start_position.x - p.x);
                                        let dy = f32::from(ev.start_position.y - p.y);
                                        let d2 = dx * dx + dy * dy;
                                        (d2 <= hit_radius_sq).then_some(d2)
                                    };
                                    let chosen = match (near(s_pt), near(e_pt)) {
                                        (Some(a), Some(b)) => Some(if a <= b {
                                            SelectionHandleKind::Start
                                        } else {
                                            SelectionHandleKind::End
                                        }),
                                        (Some(_), None) => Some(SelectionHandleKind::Start),
                                        (None, Some(_)) => Some(SelectionHandleKind::End),
                                        (None, None) => None,
                                    };
                                    if let Some(kind) = chosen {
                                        r.dragging_handle = Some(kind);
                                        window.prevent_default();
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                }
                                TouchPhase::Moved => {
                                    if let Some(kind) = r.dragging_handle {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                        if r.update_handle_drag_at(kind, ev.position, Some(bounds), last_c, last_s) {
                                            cx.notify();
                                        }
                                    }
                                }
                                TouchPhase::Ended | TouchPhase::Cancelled => {
                                    if r.dragging_handle.take().is_some() {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                }
                            }
                        });
                    });
                },
            )
            .size_full();

            let (start_handle_pt, end_handle_pt) = handle_points;
            let handle_overlay = |pt_opt: Option<Point<Pixels>>, is_start: bool| -> Option<AnyElement> {
                let pt = pt_opt?;
                let rel_x = (f32::from(pt.x - vp_bounds.origin.x) - 8.0)
                    .clamp(2.0, (f32::from(vp_bounds.size.width) - 18.0).max(2.0));
                let rel_y = (f32::from(pt.y - vp_bounds.origin.y) + 2.0)
                    .clamp(2.0, (f32::from(vp_bounds.size.height) - 24.0).max(2.0));
                Some(
                    div()
                        .absolute()
                        .left(px(rel_x))
                        .top(px(rel_y))
                        .w(px(16.))
                        .h(px(22.))
                        .rounded(px(8.))
                        .bg(hex_color(pal.accent))
                        .border_2()
                        .border_color(hex_color(0xffffff))
                        .when(is_start, |d| d.rounded_tr(px(2.)))
                        .when(!is_start, |d| d.rounded_tl(px(2.)))
                        .into_any_element(),
                )
            };

            let flip_shadow_overlay = if visual_swipe_x.abs() > 2.0 && vp_bounds.size.width > px(40.) {
                let vp_w = f32::from(vp_bounds.size.width);
                let progress = (visual_swipe_x.abs() / vp_w).clamp(0.0, 1.0);
                let edge_x = if visual_swipe_x < 0.0 {
                    (vp_w + visual_swipe_x).clamp(0.0, vp_w)
                } else {
                    visual_swipe_x.clamp(0.0, vp_w)
                };
                let shadow_alpha = ((0.42 - progress * 0.18).max(0.10) * 255.0) as u32;
                Some(
                    div()
                        .id("rdr-flip-shadow")
                        .absolute()
                        .inset_0()
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(px((edge_x - 22.0).max(0.0)))
                                .w(px(26.))
                                .bg(hex_alpha(0x00000000 | shadow_alpha)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(px(edge_x))
                                .w(px(2.))
                                .bg(hex_color(pal.border)),
                        ),
                )
            } else {
                None
            };

            div()
                .id("rdr-viewport-shell")
                .relative()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .child(
                    div()
                        .relative()
                        .size_full()
                        .left(px(visual_swipe_x))
                        .on_scroll(cx.listener(move |this, ev: &ScrollEvent, window, cx| {
                            let delta = ev.delta.pixel_delta(px(24.));
                            let dx = f32::from(delta.x);
                            let dy = f32::from(delta.y);

                            let blocked_by_sel = this.reader.as_ref().is_some_and(|r| {
                                r.is_long_press_selecting
                                    || r.dragging_handle.is_some()
                                    || r.draft.is_some()
                                    || r.show_toc
                                    || r.show_notes
                            });
                            if blocked_by_sel {
                                cx.stop_propagation();
                                return;
                            }

                            if ev.touch_phase == TouchPhase::Started {
                                if let Some(r) = &mut this.reader {
                                    r.swipe_ignore_momentum = false;
                                }
                            }

                            let already_swiping =
                                this.reader.as_ref().is_some_and(|r| r.swipe_active);
                            let ignore_mom =
                                this.reader.as_ref().is_some_and(|r| r.swipe_ignore_momentum);
                            let is_horizontal = !ignore_mom
                                && (already_swiping || (dx.abs() > 1.0 && dx.abs() > dy.abs() * 1.15));

                            if is_horizontal {
                                cx.stop_propagation();
                                let mut commit: Option<i8> = None;
                                let mut spring_from: Option<f32> = None;
                                let vp_w = this
                                    .reader
                                    .as_ref()
                                    .map(|r| f32::from(r.viewport_bounds.get().size.width).max(300.0))
                                    .unwrap_or(360.0);

                                if let Some(r) = &mut this.reader {
                                    match ev.touch_phase {
                                        TouchPhase::Started | TouchPhase::Moved => {
                                            r.swipe_active = true;
                                            r.flip_anim = None;
                                            let next_raw = r.swipe_offset_x + dx;
                                            let can_prev = r.chapter_idx > 0;
                                            let can_next = r.chapter_idx + 1 < r.book.chapters.len();
                                            let eff_dx = if (next_raw > 0.0 && !can_prev)
                                                || (next_raw < 0.0 && !can_next)
                                            {
                                                dx * 0.25
                                            } else {
                                                dx
                                            };
                                            r.swipe_offset_x = (r.swipe_offset_x + eff_dx)
                                                .clamp(-vp_w * 0.85, vp_w * 0.85);
                                            cx.notify();
                                        }
                                        TouchPhase::Ended | TouchPhase::Cancelled => {
                                            let final_off = r.swipe_offset_x;
                                            r.swipe_active = false;
                                            r.swipe_ignore_momentum = true;
                                            let threshold = (vp_w * 0.14).clamp(42.0, 90.0);
                                            if final_off <= -threshold {
                                                commit = Some(1);
                                                spring_from = Some(vp_w * 0.45);
                                            } else if final_off >= threshold {
                                                commit = Some(-1);
                                                spring_from = Some(-vp_w * 0.45);
                                            } else if final_off.abs() > 1.0 {
                                                spring_from = Some(final_off);
                                            }
                                            r.swipe_offset_x = 0.0;
                                        }
                                    }
                                }

                                if let Some(dir) = commit {
                                    if dir > 0 {
                                        this.reader_next_page_or_chapter(spring_from, use_two_col, window, cx);
                                    } else {
                                        this.reader_prev_page_or_chapter(spring_from, use_two_col, window, cx);
                                    }
                                } else if let Some(sx) = spring_from {
                                    this.reader_start_page_flip_anim(sx, 0.0, 0, 0, 170, cx);
                                }
                                return;
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                                if let Some(r) = &mut this.reader {
                                    if r.show_toc || r.show_notes {
                                        return;
                                    }
                                    r.mouse_down_pos = Some(ev.position);
                                    r.is_mouse_selecting = false;
                                    if ev.click_count == 2 {
                                        let b = r.viewport_bounds.get();
                                        if r.select_word_at_point(ev.position, Some(b)) {
                                            r.is_mouse_selecting = true;
                                            cx.notify();
                                        }
                                    }
                                }
                            }),
                        )
                        .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _window, cx| {
                            let Some(r) = &mut this.reader else {
                                return;
                            };
                            if !ev.dragging() || r.is_long_press_selecting || r.dragging_handle.is_some() {
                                return;
                            }
                            if r.is_mouse_selecting {
                                let b = r.viewport_bounds.get();
                                if let Some(hit) = r.hit_test_position(ev.position, Some(b))
                                    && r.selection_head != Some(hit)
                                {
                                    r.selection_head = Some(hit);
                                    cx.notify();
                                }
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, ev: &MouseUpEvent, window, cx| {
                                if let Some(r) = &mut this.reader {
                                    if r.show_toc || r.show_notes || r.is_long_press_selecting {
                                        return;
                                    }
                                    if r.is_mouse_selecting {
                                        r.is_mouse_selecting = false;
                                        this.reader_finalize_selection_draft(window, cx);
                                        return;
                                    }
                                    let down_pos = r.mouse_down_pos.take();
                                    let moved_far = down_pos.is_some_and(|p| {
                                        let dx = f32::from(ev.position.x - p.x);
                                        let dy = f32::from(ev.position.y - p.y);
                                        dx * dx + dy * dy > 64.0
                                    });
                                    if moved_far {
                                        return;
                                    }

                                    let b = r.viewport_bounds.get();
                                    let hit_pos = r.hit_test_position(ev.position, Some(b));
                                    let existing_hl = hit_pos.and_then(|pos| {
                                        let user = this.library_store.books.get(&r.book_rel)?;
                                        r.highlight_at_pos(&user.highlights, pos).cloned()
                                    });
                                    if let Some(hl) = existing_hl {
                                        this.reader_open_existing_highlight(&hl, window, cx);
                                        return;
                                    }

                                    if r.draft.is_some() || r.normalized_selection().is_some() {
                                        r.clear_selection_and_draft();
                                        window.blur(cx);
                                        gpui_mobile::hide_keyboard();
                                        cx.notify();
                                    }
                                }
                            }),
                        )
                        .child(
                            v_flex()
                                .id("epub-chapter-scroll")
                                .size_full()
                                .overflow_y_scroll()
                                .track_scroll(&app.reader_scroll)
                                .child(body_el),
                        )
                        .child(div().absolute().inset_0().child(gesture_canvas)),
                )
                .when_some(handle_overlay(start_handle_pt, true), |el, h| el.child(h))
                .when_some(handle_overlay(end_handle_pt, false), |el, h| el.child(h))
                .when_some(flip_shadow_overlay, |el, sh| el.child(sh))
                .child(Scrollbar::vertical(&app.reader_scroll))
                .into_any_element()
        }
    };

    let pct = (((ch_idx + 1) as f32 / total_ch as f32) * 100.0).round() as u8;
    let footer_meta = if is_page_mode {
        format!(
            "Page {} / {} · Ch {} / {} · {}%",
            page_idx + 1,
            total_pages,
            ch_idx + 1,
            total_ch,
            pct
        )
    } else {
        format!("Ch {} / {} · {}%", ch_idx + 1, total_ch, pct)
    };
    let footer = v_flex()
        .w_full()
        .flex_shrink_0()
        .bg(hex_color(pal.surface))
        .border_t_1()
        .border_color(hex_color(pal.border))
        .when_some(draft.clone(), |sheet_col, d| {
            let preview: String = d.quote.chars().take(90).collect();
            let has_existing = d.existing_id.is_some();
            sheet_col.child(
                v_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .bg(hex_color(pal.quote_bg))
                    .border_b_1()
                    .border_color(hex_color(pal.border))
                    .child(
                        div()
                            .text_xs()
                            .text_color(hex_color(pal.muted))
                            .truncate()
                            .child(format!("\"{preview}…\"")),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                h_flex().gap_2().children(HighlightColor::all().iter().copied().map(
                                    |hc| {
                                        let active = d.color == hc;
                                        div()
                                            .id(SharedString::from(format!("hc-{}", hc.label())))
                                            .w(px(30.))
                                            .h(px(30.))
                                            .rounded(px(999.))
                                            .cursor_pointer()
                                            .bg(hex_color(hc.rgb_hex()))
                                            .border_2()
                                            .border_color(if active {
                                                hex_color(pal.fg)
                                            } else {
                                                transparent_black()
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(ReaderState {
                                                    draft: Some(dr), ..
                                                }) = &mut this.reader
                                                {
                                                    dr.color = hc;
                                                }
                                                cx.notify();
                                            }))
                                    },
                                )),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .gap_1p5()
                            .children(HighlightDecoration::all().iter().copied().map(|st| {
                                let active = d.style == st;
                                div()
                                    .id(SharedString::from(format!("hs-{}", st.label())))
                                    .flex_1()
                                    .py_1()
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .bg(hex_color(if active {
                                        pal.accent
                                    } else {
                                        pal.surface
                                    }))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(hex_color(if active {
                                        0x181619
                                    } else {
                                        pal.fg
                                    }))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(ReaderState {
                                            draft: Some(dr), ..
                                        }) = &mut this.reader
                                        {
                                            dr.style = st;
                                        }
                                        cx.notify();
                                    }))
                                    .child(st.label())
                            })),
                    )
                    .child(Input::new(&app.highlight_note_input).small())
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .child(
                                        Button::new("hl-cancel")
                                            .ghost()
                                            .small()
                                            .label("Cancel")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if let Some(r) = &mut this.reader {
                                                    r.clear_selection_and_draft();
                                                }
                                                window.blur(cx);
                                                gpui_mobile::hide_keyboard();
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("hl-copy")
                                            .secondary()
                                            .small()
                                            .label("Copy")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.reader_copy_active_text(window, cx);
                                            })),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .when(has_existing, |r| {
                                        r.child(
                                            Button::new("hl-delete")
                                                .danger()
                                                .small()
                                                .label("Delete")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.reader_delete_draft_highlight(window, cx);
                                                })),
                                        )
                                    })
                                    .child(
                                        Button::new("hl-save")
                                            .primary()
                                            .small()
                                            .label("Save Highlight")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.reader_save_highlight_draft(window, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
        })
        .child(
            h_flex()
                .w_full()
                .h(px(48.))
                .px_3()
                .justify_between()
                .items_center()
                .child(
                    Button::new("rdr-prev")
                        .secondary()
                        .small()
                        .label("← Prev")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.reader_prev_page_or_chapter(None, wide_viewport && this.two_column_spread, window, cx);
                        })),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(pal.muted))
                        .child(footer_meta),
                )
                .child(
                    Button::new("rdr-next")
                        .secondary()
                        .small()
                        .label("Next →")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.reader_next_page_or_chapter(None, wide_viewport && this.two_column_spread, window, cx);
                        })),
                ),
        )
        .child(div().w_full().h(px(app.safe_area.bottom)));

    v_flex()
        .size_full()
        .bg(hex_color(pal.bg))
        .text_color(hex_color(pal.fg))
        .child(top_bar)
        .when(*show_settings, |col| col.child(settings_strip))
        .child(main_area)
        .child(footer)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        BookHighlight, BookwormMobileApp, EpubBlock, EpubRun, HighlightColor, HighlightDecoration,
        flatten_block_highlights, hex_color, split_blocks_two_columns,
    };

    #[test]
    fn compute_block_scroll_target_y_handles_scrolled_and_unscrolled_coords() {
        let target =
            BookwormMobileApp::compute_block_scroll_target_y(900.0, 100.0, 0.0, 2000.0, 50.0);
        assert!((target - 750.0).abs() < 1e-4);

        let target_scrolled =
            BookwormMobileApp::compute_block_scroll_target_y(600.0, 100.0, -300.0, 2000.0, 50.0);
        assert!((target_scrolled - 750.0).abs() < 1e-4);

        let target_up =
            BookwormMobileApp::compute_block_scroll_target_y(-800.0, 100.0, -1200.0, 2000.0, 50.0);
        assert!((target_up - 250.0).abs() < 1e-4);

        assert_eq!(
            BookwormMobileApp::compute_block_scroll_target_y(120.0, 100.0, 0.0, 2000.0, 50.0),
            0.0
        );
        assert_eq!(
            BookwormMobileApp::compute_block_scroll_target_y(3500.0, 100.0, 0.0, 2000.0, 50.0),
            2000.0
        );
        assert_eq!(
            BookwormMobileApp::compute_block_scroll_target_y(500.0, 100.0, 0.0, 0.0, 50.0),
            0.0
        );
    }

    #[test]
    fn flatten_highlights_handles_unicode_bold_and_moon_styles_safely() {
        let txt = "Hello Привет мир world!";
        let runs = vec![EpubRun {
            range: 6..18,
            bold: true,
            italic: false,
            underline: false,
            code: false,
        }];
        let hl = BookHighlight {
            id: "1".into(),
            chapter_idx: 0,
            chapter_title: "Ch".into(),
            start_block: 0,
            start_offset: 12,
            end_block: 0,
            end_offset: txt.len(),
            quote: "вет мир world!".into(),
            color: HighlightColor::Purple,
            style: HighlightDecoration::Squiggly,
            note: "note".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let range = hl.block_byte_range(0, 0, txt.len()).unwrap();
        let flat = flatten_block_highlights(
            txt,
            &runs,
            &[(&hl, range)],
            Some(0..5),
            hex_color(0x222222),
            hex_color(0xeeeeee),
            hex_color(0x334466),
            false,
        );
        assert!(!flat.is_empty());
        for (r, _) in &flat {
            assert!(txt.is_char_boundary(r.start));
            assert!(txt.is_char_boundary(r.end));
            assert!(r.start < r.end);
        }
    }

    #[test]
    fn two_column_split_balances_blocks() {
        let blocks = vec![
            EpubBlock::Heading {
                level: 1,
                text: "Title".into(),
            },
            EpubBlock::Paragraph {
                text: "A".repeat(300),
                runs: vec![],
                is_quote: false,
            },
            EpubBlock::Paragraph {
                text: "B".repeat(300),
                runs: vec![],
                is_quote: false,
            },
            EpubBlock::Paragraph {
                text: "C".repeat(120),
                runs: vec![],
                is_quote: false,
            },
        ];
        let idx = split_blocks_two_columns(&blocks);
        assert!((1..blocks.len()).contains(&idx));
    }
}
