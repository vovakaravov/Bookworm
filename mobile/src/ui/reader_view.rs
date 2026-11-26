use super::image_util::decode_bytes_image;
use super::{
    BookwormMobileApp, DictLookupState, PageTouchMode, ReaderState, SelectionHandleKind,
    SliceRenderKey, SliceRenderValue,
};
use crate::theme::{ReaderPalette, ReaderTheme, hex_alpha, hex_color};
use bookworm_core::epub::{EpubBlock, EpubRun};
use bookworm_core::library::{
    BookHighlight, ChapterPage, HighlightColor, HighlightDecoration, PageSlice, ReaderWidthPref,
    clamp_to_char_boundary,
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
                }
                HighlightDecoration::Underline => {
                    style.underline = Some(UnderlineStyle {
                        thickness: px(2.0),
                        color: Some(solid),
                        wavy: false,
                    });
                }
                HighlightDecoration::Dotted
                | HighlightDecoration::Dashed
                | HighlightDecoration::Double => {}
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
                HighlightDecoration::Tint => {
                    style.color = Some(solid);
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

fn finish_styled_text_element(
    disp_shared: SharedString,
    flat: Vec<(Range<usize>, HighlightStyle)>,
    custom_lines: Vec<(HighlightDecoration, Range<usize>, u32)>,
    blk_idx: usize,
    base_off: usize,
    disp_len: usize,
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
) -> AnyElement {
    let st = if flat.is_empty() {
        StyledText::new(disp_shared.clone())
    } else {
        StyledText::new(disp_shared.clone()).with_highlights(flat)
    };
    let layout_for_paint = st.layout().clone();
    if let Some(slot) = rendered_layouts {
        slot.borrow_mut()
            .push((blk_idx, base_off, disp_len, layout_for_paint.clone()));
    }
    if custom_lines.is_empty() {
        st.into_any_element()
    } else {
        div()
            .relative()
            .w_full()
            .child(st)
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, _| {
                        paint_custom_decoration_lines(
                            &layout_for_paint,
                            &disp_shared,
                            &custom_lines,
                            window,
                        );
                    },
                )
                .absolute()
                .inset_0(),
            )
            .into_any_element()
    }
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
    highlights_epoch: u64,
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
    slice_cache: &RefCell<HashMap<SliceRenderKey, SliceRenderValue>>,
) -> AnyElement {
    let cache_key = slice.filter(|_| sel_range.is_none()).map(|sl| {
        (
            chapter_idx,
            blk_idx,
            sl.byte_range.start,
            sl.byte_range.end,
            light_paper,
            highlights_epoch,
        )
    });
    if let Some(ck) = cache_key
        && let Some((cached_text, cached_flat, cached_custom)) = slice_cache.borrow().get(&ck)
    {
        let disp_len = cached_text.len();
        return finish_styled_text_element(
            cached_text.clone(),
            cached_flat.clone(),
            cached_custom.clone(),
            blk_idx,
            ck.2,
            disp_len,
            rendered_layouts,
        );
    }

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
    let disp_shared = SharedString::from(disp_text);
    let custom_lines: Vec<(HighlightDecoration, Range<usize>, u32)> = block_hls
        .iter()
        .filter(|(hl, _)| hl.style.is_custom_line())
        .map(|(hl, r)| (hl.style, r.clone(), hl.color.rgb_hex()))
        .collect();
    if let Some(ck) = cache_key {
        let mut cache = slice_cache.borrow_mut();
        if cache.len() >= 128 {
            cache.clear();
        }
        cache.insert(
            ck,
            (disp_shared.clone(), flat.clone(), custom_lines.clone()),
        );
    }
    finish_styled_text_element(
        disp_shared,
        flat,
        custom_lines,
        blk_idx,
        base_off,
        disp_len,
        rendered_layouts,
    )
}

fn paint_custom_decoration_lines(
    layout: &TextLayout,
    disp_text: &str,
    custom_lines: &[(HighlightDecoration, Range<usize>, u32)],
    window: &mut Window,
) {
    let lh = f32::from(layout.line_height()).max(12.0);
    for (deco, range, rgb) in custom_lines {
        let a = clamp_to_char_boundary(disp_text, range.start);
        let b = clamp_to_char_boundary(disp_text, range.end);
        if a >= b {
            continue;
        }
        let color = hex_color(*rgb);
        let mut line_segments: Vec<(f32, f32, f32)> = Vec::new();
        for (rel, ch) in disp_text[a..b].char_indices() {
            let off = a + rel;
            let next = off + ch.len_utf8();
            let Some(p0) = layout.position_for_index(off) else {
                continue;
            };
            let x0 = f32::from(p0.x);
            let y0 = f32::from(p0.y);
            let x1 = match layout.position_for_index(next) {
                Some(p1) if (f32::from(p1.y) - y0).abs() < 1.5 && f32::from(p1.x) > x0 => {
                    f32::from(p1.x)
                }
                _ if ch.is_whitespace() => continue,
                _ => x0 + 6.0,
            };
            if let Some(last) = line_segments.last_mut()
                && (last.0 - y0).abs() < 1.5
            {
                last.1 = last.1.min(x0);
                last.2 = last.2.max(x1);
            } else {
                line_segments.push((y0, x0, x1));
            }
        }
        for (y0, x0, x1) in line_segments {
            if x1 <= x0 {
                continue;
            }
            let base_y = y0 + lh - 2.5;
            match deco {
                HighlightDecoration::Dotted => {
                    let mut x = x0;
                    while x + 2.0 <= x1 {
                        window.paint_quad(
                            fill(
                                Bounds::new(point(px(x), px(base_y - 1.2)), size(px(2.4), px(2.4))),
                                color,
                            )
                            .corner_radii(px(1.2)),
                        );
                        x += 5.2;
                    }
                }
                HighlightDecoration::Dashed => {
                    let mut x = x0;
                    while x < x1 {
                        let w = (x1 - x).min(5.5);
                        window.paint_quad(
                            fill(
                                Bounds::new(point(px(x), px(base_y - 1.0)), size(px(w), px(2.0))),
                                color,
                            )
                            .corner_radii(px(1.0)),
                        );
                        x += 9.0;
                    }
                }
                HighlightDecoration::Double => {
                    window.paint_quad(fill(
                        Bounds::new(point(px(x0), px(base_y - 2.2)), size(px(x1 - x0), px(1.3))),
                        color,
                    ));
                    window.paint_quad(fill(
                        Bounds::new(point(px(x0), px(base_y + 0.6)), size(px(x1 - x0), px(1.3))),
                        color,
                    ));
                }
                _ => {}
            }
        }
    }
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
    highlights_epoch: u64,
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
    slice_cache: &RefCell<HashMap<SliceRenderKey, SliceRenderValue>>,
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
                highlights_epoch,
                rendered_layouts,
                slice_cache,
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
                highlights_epoch,
                rendered_layouts,
                slice_cache,
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
                highlights_epoch,
                rendered_layouts,
                slice_cache,
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
    highlights_epoch: u64,
    selection_bounds: &[Option<Range<usize>>],
    rendered_layouts: Option<&Rc<RefCell<Vec<(usize, usize, usize, TextLayout)>>>>,
    slice_cache: &RefCell<HashMap<SliceRenderKey, SliceRenderValue>>,
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
                        highlights_epoch,
                        rendered_layouts,
                        slice_cache,
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

/// Returns `-1` for a left-edge tap (previous page), `1` for a right-edge tap (next page),
/// or `0` for a center tap.
pub fn edge_tap_direction(rel_x: f32, viewport_w: f32) -> i8 {
    if viewport_w <= 0.0 {
        return 0;
    }
    let frac = rel_x / viewport_w;
    if frac <= 0.24 {
        -1
    } else if frac >= 0.76 {
        1
    } else {
        0
    }
}

pub(crate) fn resolve_draft_dock_top(
    sel_mid_y: Option<f32>,
    vp_top: f32,
    vp_height: f32,
    prev_dock_top: bool,
    initialized: bool,
) -> bool {
    let Some(mid_y) = sel_mid_y else {
        return prev_dock_top && initialized;
    };
    if vp_height <= 1.0 {
        return false;
    }
    let center_y = vp_top + vp_height * 0.5;
    if !initialized {
        mid_y >= center_y
    } else if prev_dock_top {
        mid_y >= center_y - 20.0
    } else {
        mid_y > center_y + 20.0
    }
}

pub fn render_reader_view(
    app: &mut BookwormMobileApp,
    window: &mut Window,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let wide_viewport = f32::from(window.viewport_size().width) >= 700.0;
    let use_two_col = app.two_column_spread && wide_viewport;
    let width_pref = app.reader_width;

    if let Some((anim_gen, done)) = app.reader.as_ref().and_then(|r| {
        r.flip_anim.as_ref().map(|a| {
            (
                r.flip_anim_gen,
                a.started_at.elapsed().as_millis() as u64 >= a.duration_ms,
            )
        })
    }) && done
    {
        app.finish_page_flip_anim(anim_gen, window, cx);
    }

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
            app.maybe_schedule_adjacent_prepag(vp_w, vp_h, use_two_col, window, cx);
        } else {
            let entity = cx.entity().clone();
            window.on_next_frame(move |_, cx| {
                let _ = entity.update(cx, |_, cx| cx.notify());
            });
        }
    }

    let col_max_w = if use_two_col {
        Some(1140.0)
    } else {
        width_pref.max_width_px_opt()
    };

    let Some(r) = &app.reader else {
        return div().size_full().into_any_element();
    };
    if let Some(anim) = &r.flip_anim {
        let anim_gen = r.flip_anim_gen;
        let done = anim.started_at.elapsed().as_millis() as u64 >= anim.duration_ms;
        let entity = cx.entity().clone();
        if done {
            window.on_next_frame(move |window, cx| {
                let _ = entity.update(cx, |this, cx| {
                    this.finish_page_flip_anim(anim_gen, window, cx);
                });
            });
        } else {
            window.on_next_frame(move |_, cx| {
                let _ = entity.update(cx, |_, cx| cx.notify());
            });
        }
    }
    let book_rel = &r.book_rel;
    let book_title = &r.book_title;
    let book = &r.book;
    let ch_idx = r.chapter_idx.min(book.chapters.len().max(1).saturating_sub(1));
    let show_settings = &r.show_settings;
    let show_toc = &r.show_toc;
    let show_notes = &r.show_notes;
    let zen_mode = r.zen_mode;
    let notes_color_filter = &r.notes_color_filter;
    let notes_only_filter = &r.notes_only_filter;
    let draft = &r.draft;
    let draft_sheet_bounds_ref = r.draft_sheet_bounds.clone();
    let settings_strip_bounds_ref = r.settings_strip_bounds.clone();
    if !*show_settings {
        *settings_strip_bounds_ref.borrow_mut() = None;
    }
    if draft.is_none() {
        *draft_sheet_bounds_ref.borrow_mut() = None;
    }
    let rendered_layouts = &r.rendered_layouts;
    let layouts_measured_slot = r.layouts_measured.clone();
    let slice_cache = &r.slice_render_cache;
    let highlights_epoch = r.highlights_epoch;
    let decoded_images = &r.decoded_images;
    let vp_slot = r.viewport_bounds.clone();
    let vp_bounds = r.viewport_bounds.get();
    // Must compute dock_top BEFORE rendered_layouts is cleared and refilled with unmeasured TextLayouts.
    let dock_top = if draft.is_some() {
        let (sg, eg) = r.selection_handle_geometries(vp_bounds);
        let sel_mid_y = match (sg, eg) {
            (Some((sp, _)), Some((ep, elh))) => {
                Some((f32::from(sp.y) + f32::from(ep.y) + elh) * 0.5)
            }
            (Some((p, lh)), None) | (None, Some((p, lh))) => Some(f32::from(p.y) + lh * 0.5),
            (None, None) => None,
        };
        let dock = resolve_draft_dock_top(
            sel_mid_y,
            f32::from(vp_bounds.origin.y),
            f32::from(vp_bounds.size.height),
            r.draft_dock_top.get(),
            r.draft_dock_initialized.get(),
        );
        if sel_mid_y.is_some() {
            r.draft_dock_top.set(dock);
            r.draft_dock_initialized.set(true);
        }
        dock
    } else {
        false
    };
    let visual_swipe_x = r.visual_swipe_offset_x();
    let is_anim_or_swipe = r.flip_anim.is_some() || r.swipe_active;
    let (page_idx, total_pages) = {
        let tp = r.chapter_pages.len().max(1);
        let cp = r.current_page.min(tp - 1);
        (cp, tp)
    };
    let selection_bounds: Vec<Option<Range<usize>>> = if r.normalized_selection().is_some() {
        book.chapters
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
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let pal = app.reader_theme.palette();
    let light_paper = matches!(app.reader_theme, ReaderTheme::Sepia | ReaderTheme::Paper);
    let user_state_opt = app.library_store.books.get(book_rel);
    let highlights: &[BookHighlight] = user_state_opt
        .map(|u| u.highlights.as_slice())
        .unwrap_or(&[]);
    let bookmarks: &[bookworm_core::library::BookBookmark] = user_state_opt
        .map(|u| u.bookmarks.as_slice())
        .unwrap_or(&[]);
    let chapter = book.chapters.get(ch_idx);
    let ch_title = chapter
        .map(|c| c.title.clone())
        .unwrap_or_else(|| book_title.clone());
    let is_bookmarked = bookmarks.iter().any(|b| b.chapter_idx == ch_idx);
    let ann_count = highlights.len() + bookmarks.len();

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
                    Button::new("rdr-zen")
                        .secondary()
                        .small()
                        .label("Zen")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_reader_zen_mode(true, cx);
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

    let settings_bounds_tracker = settings_strip_bounds_ref.clone();
    let settings_strip = v_flex()
        .id("rdr-settings-overlay")
        .w_full()
        .px_3p5()
        .py_2p5()
        .gap_2()
        .bg(hex_color(pal.surface))
        .border_b_1()
        .border_color(hex_color(pal.border))
        .shadow_md()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .on_click(|_, _, cx| cx.stop_propagation())
        .child(
            canvas(
                move |bounds, _, _| {
                    *settings_bounds_tracker.borrow_mut() = Some(bounds);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        )
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
                            Button::new("rdr-zen-pill")
                                .secondary()
                                .small()
                                .label("Zen Mode")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_reader_zen_mode(true, cx);
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
        )
        .with_animation(
            "rdr-settings-drop",
            Animation::new(Duration::from_millis(175)).with_easing(ease_out_quint()),
            |el, t| el.mt(px(-14.0 * (1.0 - t))).opacity(t),
        );

    let main_area = if *show_toc {
        div()
            .id("rdr-toc-panel")
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
            .with_animation(
                "rdr-toc-fade",
                Animation::new(Duration::from_millis(170)).with_easing(ease_out_quint()),
                |el, t| el.mt(px(-10.0 * (1.0 - t))).opacity(t),
            )
            .into_any_element()
    } else if *show_notes {
        let active_color_filter = *notes_color_filter;
        let active_notes_only = *notes_only_filter;
        let mut filtered_hls: Vec<BookHighlight> = highlights
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
            .id("rdr-notes-panel")
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
                            .when(!highlights.is_empty(), |col| {
                                col.child(filter_bar)
                            })
                            .when(
                                highlights.is_empty() && bookmarks.is_empty(),
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
                                !highlights.is_empty()
                                    && filtered_hls.is_empty()
                                    && bookmarks.is_empty(),
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
                            .children(bookmarks.iter().enumerate().map(|(ix, bm)| {
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
                                                this.schedule_library_sync_save(true, cx);
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
                                                    if let Some(r) = &mut this.reader {
                                                        r.highlights_epoch =
                                                            r.highlights_epoch.wrapping_add(1);
                                                        r.slice_render_cache.borrow_mut().clear();
                                                    }
                                                    this.schedule_library_sync_save(true, cx);
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
            .with_animation(
                "rdr-notes-fade",
                Animation::new(Duration::from_millis(170)).with_easing(ease_out_quint()),
                |el, t| el.mt(px(-10.0 * (1.0 - t))).opacity(t),
            )
            .into_any_element()
    } else {
        let active_layouts = (!is_anim_or_swipe).then_some(rendered_layouts);
        if !is_anim_or_swipe {
            layouts_measured_slot.set(false);
            rendered_layouts.borrow_mut().clear();
        }
        let vp_w_cur = f32::from(vp_bounds.size.width).max(300.0);
        let vp_h_cur = f32::from(vp_bounds.size.height).max(200.0);
        let max_img_h = (vp_h_cur - 32.0) * 0.68;

        {
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
                highlights,
                highlights_epoch,
                &selection_bounds,
                active_layouts,
                slice_cache,
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
                        highlights,
                        highlights_epoch,
                        &[],
                        None,
                        slice_cache,
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
                        highlights,
                        highlights_epoch,
                        &[],
                        None,
                        slice_cache,
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
                        highlights,
                        highlights_epoch,
                        &[],
                        None,
                        slice_cache,
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
                        highlights,
                        highlights_epoch,
                        &[],
                        None,
                        slice_cache,
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
            let layouts_measured_ready = layouts_measured_slot.clone();
            let paint_accent = hex_color(pal.accent);
            let paint_border = hex_color(pal.fg);
            let page_touch_canvas = canvas(
                move |bounds, _, _| {
                    vp_measure.set(bounds);
                    layouts_measured_ready.set(true);
                },
                move |bounds, _, window, cx| {
                    if let Some(app_ent) = app_weak_page.upgrade() {
                        let app = app_ent.read(cx);
                        if let Some(r) = &app.reader {
                            if !r.show_toc
                                && !r.show_notes
                                && !r.swipe_active
                                && r.flip_anim.is_none()
                                && r.normalized_selection().is_some()
                            {
                                let (sh, eh) = r.selection_handle_geometries(bounds);
                                let sk = sh.map(|(p, lh)| (point(p.x, p.y + px(lh) + px(8.0)), lh));
                                let ek = eh.map(|(p, lh)| (point(p.x, p.y + px(lh) + px(8.0)), lh));
                                *r.sel_handle_positions.borrow_mut() =
                                    if sk.is_some() || ek.is_some() {
                                        Some((sk, ek))
                                    } else {
                                        None
                                    };
                                let paint_handle =
                                    |win: &mut Window, pt: Point<Pixels>, lh: f32| {
                                        win.paint_quad(fill(
                                            Bounds {
                                                origin: pt,
                                                size: size(px(2.), px(lh)),
                                            },
                                            paint_accent,
                                        ));
                                        win.paint_quad(quad(
                                            Bounds {
                                                origin: point(pt.x - px(8.), pt.y + px(lh)),
                                                size: size(px(16.), px(16.)),
                                            },
                                            px(8.),
                                            paint_accent,
                                            gpui::Edges::all(px(1.5)),
                                            paint_border,
                                            BorderStyle::Solid,
                                        ));
                                    };
                                if let Some((sp, slh)) = sh {
                                    paint_handle(window, sp, slh);
                                }
                                if let Some((ep, elh)) = eh {
                                    paint_handle(window, ep, elh);
                                }
                            } else {
                                *r.sel_handle_positions.borrow_mut() = None;
                            }
                        }
                    }
                    let weak_td = app_weak_page.clone();
                    window.on_mouse_event(move |ev: &TouchDragEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        if ev.phase == TouchPhase::Started && !bounds.contains(&ev.start_position) {
                            return;
                        }
                        let _ = weak_td.update(cx, |app, cx| {
                            let in_active_drag = app.reader.as_ref().is_some_and(|r| {
                                matches!(
                                    r.page_touch_mode,
                                    Some(
                                        PageTouchMode::HandleDrag(_)
                                            | PageTouchMode::LongPressSelecting
                                            | PageTouchMode::HorizontalSwipe
                                    )
                                )
                            });
                            let check_pos = if ev.phase == TouchPhase::Started {
                                ev.start_position
                            } else {
                                ev.position
                            };
                            let in_overlay = app.reader.as_ref().is_some_and(|r| {
                                r.draft_sheet_bounds
                                    .borrow()
                                    .is_some_and(|b| b.contains(&check_pos))
                                    || r.settings_strip_bounds
                                        .borrow()
                                        .is_some_and(|b| b.contains(&check_pos))
                            });
                            if !in_active_drag && in_overlay {
                                return;
                            }
                            let is_reading = app
                                .reader
                                .as_ref()
                                .is_some_and(|r| !r.show_toc && !r.show_notes);
                            if !is_reading {
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
                                    let cached_knobs = *r.sel_handle_positions.borrow();
                                    let (s_knob, e_knob) = cached_knobs.unwrap_or_else(|| {
                                        let (sh, eh) = r.selection_handle_geometries(bounds);
                                        let sk = sh.map(|(p, lh)| {
                                            (point(p.x, p.y + px(lh) + px(8.0)), lh)
                                        });
                                        let ek = eh.map(|(p, lh)| {
                                            (point(p.x, p.y + px(lh) + px(8.0)), lh)
                                        });
                                        (sk, ek)
                                    });
                                    let hit_radius_sq = 32.0_f32 * 32.0;
                                    let near = |pt_opt: Option<(Point<Pixels>, f32)>| -> Option<(f32, Point<Pixels>, f32)> {
                                        let (p, lh) = pt_opt?;
                                        let dx = f32::from(ev.start_position.x - p.x);
                                        let dy = f32::from(ev.start_position.y - p.y);
                                        let d2 = dx * dx + dy * dy;
                                        (d2 <= hit_radius_sq).then_some((d2, p, lh))
                                    };
                                    let chosen_handle = match (near(s_knob), near(e_knob)) {
                                        (Some((da, pa, lha)), Some((db, pb, lhb))) => Some(
                                            if da <= db {
                                                (SelectionHandleKind::Start, pa, lha)
                                            } else {
                                                (SelectionHandleKind::End, pb, lhb)
                                            },
                                        ),
                                        (Some((_, p, lh)), None) => {
                                            Some((SelectionHandleKind::Start, p, lh))
                                        }
                                        (None, Some((_, p, lh))) => {
                                            Some((SelectionHandleKind::End, p, lh))
                                        }
                                        (None, None) => None,
                                    };
                                    if let Some((kind, knob_pt, lh)) = chosen_handle {
                                        let line_center_y =
                                            f32::from(knob_pt.y) - 8.0 - lh * 0.5;
                                        r.handle_drag_offset_y =
                                            (f32::from(ev.start_position.y) - line_center_y)
                                                .clamp(10.0, 36.0);
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
                                    r.page_touch_cur_pos = start_pos;
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
                                            .timer(Duration::from_millis(360))
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
                                            let lp_pos = r.page_touch_cur_pos;
                                            if r.select_word_at_point(lp_pos, Some(lp_bounds)) {
                                                r.long_press_anchor_word =
                                                    r.normalized_selection();
                                                r.long_press_origin_pos = Some(lp_pos);
                                                r.long_press_dragged = false;
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
                                            let test_pos = point(
                                                ev.position.x,
                                                ev.position.y - px(r.handle_drag_offset_y),
                                            );
                                            if r.update_handle_drag_at(
                                                kind,
                                                test_pos,
                                                Some(bounds),
                                                last_c,
                                                last_s,
                                            ) {
                                                cx.notify();
                                            }
                                        }
                                        Some(PageTouchMode::LongPressSelecting) => {
                                            if !r.long_press_dragged {
                                                let moved =
                                                    r.long_press_origin_pos.is_some_and(|orig| {
                                                        let dx =
                                                            f32::from(ev.position.x - orig.x);
                                                        let dy =
                                                            f32::from(ev.position.y - orig.y);
                                                        dx * dx + dy * dy >= 64.0
                                                    });
                                                if !moved {
                                                    return;
                                                }
                                                r.long_press_dragged = true;
                                            }
                                            if let Some(hit) =
                                                r.hit_test_position(ev.position, Some(bounds))
                                                && let Some(anchor_word) =
                                                    r.long_press_anchor_word
                                            {
                                                let (new_a, new_h) =
                                                    super::extend_long_press_selection(
                                                        anchor_word,
                                                        hit,
                                                    );
                                                if r.selection_anchor != Some(new_a)
                                                    || r.selection_head != Some(new_h)
                                                {
                                                    r.selection_anchor = Some(new_a);
                                                    r.selection_head = Some(new_h);
                                                    cx.notify();
                                                }
                                            }
                                        }
                                        Some(PageTouchMode::Pending) => {
                                            r.page_touch_cur_pos = ev.position;
                                            let dx = f32::from(ev.position.x - ev.start_position.x);
                                            let dy = f32::from(ev.position.y - ev.start_position.y);
                                            if r.draft.is_some() || r.show_settings {
                                                if dx * dx + dy * dy > 64.0 {
                                                    r.page_touch_gen =
                                                        r.page_touch_gen.wrapping_add(1);
                                                    r._page_lp_task = None;
                                                }
                                            } else if dx.abs() >= 9.0
                                                && dx.abs() >= dy.abs() * 0.55
                                            {
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
                                            let quote = if let Some(r) = &mut app.reader {
                                                r.dragging_handle = None;
                                                r.normalized_selection()
                                                    .map(|(s, e)| r.extract_text_for_range(s, e))
                                            } else {
                                                None
                                            };
                                            if let Some(q) = quote {
                                                app.trigger_reader_dict_lookup(&q, cx);
                                            }
                                            cx.notify();
                                        }
                                        Some(PageTouchMode::LongPressSelecting) => {
                                            if let Some(r) = &mut app.reader {
                                                r.is_long_press_selecting = false;
                                                r.long_press_anchor_word = None;
                                                r.long_press_origin_pos = None;
                                                r.long_press_dragged = false;
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
                                            let mut tap_edge_dir: i8 = 0;
                                            let mut tap_handled = false;
                                            if let Some(r) = &mut app.reader {
                                                let down = r.mouse_down_pos.take();
                                                let moved_far = down.is_some_and(|p| {
                                                    let dx = f32::from(ev.position.x - p.x);
                                                    let dy = f32::from(ev.position.y - p.y);
                                                    dx * dx + dy * dy > 64.0
                                                });
                                                if !moved_far {
                                                    let hit_pos =
                                                        r.hit_test_strict(ev.position, Some(bounds));
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
                                                        tap_handled = true;
                                                    } else if r.show_settings {
                                                        r.show_settings = false;
                                                        cx.notify();
                                                        tap_handled = true;
                                                    } else {
                                                        tap_edge_dir = edge_tap_direction(
                                                            f32::from(
                                                                ev.position.x - bounds.origin.x,
                                                            ),
                                                            f32::from(bounds.size.width),
                                                        );
                                                    }
                                                }
                                            }
                                            if !tap_handled && tap_edge_dir != 0 {
                                                let two_col = app.two_column_spread
                                                    && f32::from(bounds.size.width) >= 700.0;
                                                if tap_edge_dir < 0 {
                                                    app.reader_prev_page_or_chapter(
                                                        None, two_col, window, cx,
                                                    );
                                                } else {
                                                    app.reader_next_page_or_chapter(
                                                        None, two_col, window, cx,
                                                    );
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
                    cx.listener(move |this, ev: &MouseUpEvent, window, cx| {
                        let mut edge_dir: i8 = 0;
                        let mut tap_handled = false;
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
                            let hit_pos = r.hit_test_strict(ev.position, Some(b));
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
                                tap_handled = true;
                            } else if r.show_settings {
                                r.show_settings = false;
                                cx.notify();
                                tap_handled = true;
                            } else {
                                edge_dir = edge_tap_direction(
                                    f32::from(ev.position.x - b.origin.x),
                                    f32::from(b.size.width),
                                );
                            }
                        }
                        if !tap_handled && edge_dir != 0 {
                            if edge_dir < 0 {
                                this.reader_prev_page_or_chapter(None, use_two_col, window, cx);
                            } else {
                                this.reader_next_page_or_chapter(None, use_two_col, window, cx);
                            }
                        }
                    }),
                )
                .child(layered_pages)
                .child(div().absolute().inset_0().child(page_touch_canvas))
                .into_any_element()
        }
    };

    let loc = r.global_location(app.reader_current_scroll_pct());
    let footer_meta = format!("Page {} / {} · {}%", loc.page, loc.total_pages, loc.progress_pct);
    let kb_height = gpui_mobile::keyboard_height();
    let draft_sheet_bottom = if zen_mode {
        app.safe_area.bottom.max(kb_height)
    } else {
        (48.0 + app.safe_area.bottom).max(kb_height)
    };
    let draft_sheet_top = if zen_mode { 8.0 } else { 52.0 };
    let dict_lookup_state = r.dict_lookup.clone();
    let draft_bounds_tracker = draft_sheet_bounds_ref.clone();
    let floating_draft_sheet = draft.clone().map(|d| {
        let preview: String = d.quote.chars().take(90).collect();
        let has_existing = d.existing_id.is_some();
        let dict_card: Option<AnyElement> = dict_lookup_state.clone().map(|dl| match dl {
            DictLookupState::Loading(term) => h_flex()
                .w_full()
                .p_2()
                .rounded(px(8.))
                .bg(hex_color(pal.quote_bg))
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(pal.muted))
                        .child(format!("Looking up \"{term}\"…")),
                )
                .into_any_element(),
            DictLookupState::NotFound(term) => h_flex()
                .w_full()
                .p_2()
                .rounded(px(8.))
                .bg(hex_color(pal.quote_bg))
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(pal.muted))
                        .child(format!("No entry found for \"{term}\"")),
                )
                .into_any_element(),
            DictLookupState::Found(entries) => {
                let top = entries.first();
                v_flex()
                    .w_full()
                    .p_2()
                    .gap_1()
                    .rounded(px(8.))
                    .bg(hex_color(pal.quote_bg))
                    .border_1()
                    .border_color(hex_color(pal.border))
                    .when_some(top, |box_el, entry| {
                        let hw = entry.display_headword().to_string();
                        let meta_line = [
                            (!entry.ipa.is_empty()).then(|| format!("[{}]", entry.ipa)),
                            (!entry.pos.is_empty()).then(|| entry.pos.clone()),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · ");
                        let infl_line = entry.matched_form.as_ref().map(|mf| match &entry.gram_tag {
                            Some(gt) => format!("form of {hw} ({gt}) · tapped: {mf}"),
                            None => format!("lemma for: {mf}"),
                        });
                        box_el
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap_1p5()
                                            .items_center()
                                            .flex_wrap()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(hex_color(pal.fg))
                                                    .child(hw),
                                            )
                                            .when(!meta_line.is_empty(), |r| {
                                                r.child(
                                                    div()
                                                        .text_size(px(10.5))
                                                        .text_color(hex_color(pal.muted))
                                                        .child(meta_line),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .px_1p5()
                                                    .py_0p5()
                                                    .rounded(px(4.))
                                                    .bg(hex_color(pal.surface))
                                                    .text_size(px(9.5))
                                                    .text_color(hex_color(pal.accent))
                                                    .child(entry.source.badge()),
                                            ),
                                    )
                                    .child(
                                        Button::new("hl-dict-save-note")
                                            .primary()
                                            .xsmall()
                                            .label("Save as Note")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.reader_save_dict_as_note(window, cx);
                                            })),
                                    ),
                            )
                            .when_some(infl_line, |col, txt| {
                                col.child(
                                    div()
                                        .text_size(px(10.))
                                        .italic()
                                        .text_color(hex_color(pal.muted))
                                        .child(txt),
                                )
                            })
                            .children(entry.glosses.iter().take(3).enumerate().map(|(ix, g)| {
                                div()
                                    .text_size(px(11.))
                                    .text_color(hex_color(pal.fg))
                                    .child(format!("{}. {g}", ix + 1))
                            }))
                            .when_some(entry.ru_translation.clone(), |col, ru| {
                                col.child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(hex_color(pal.accent))
                                        .child(format!("RU: {ru}")),
                                )
                            })
                    })
                    .into_any_element()
            }
        });
        div()
            .id("rdr-draft-sheet-overlay")
            .absolute()
            .left_0()
            .right_0()
            .when(dock_top, |d| d.top(px(draft_sheet_top)).pt_1())
            .when(!dock_top, |d| d.bottom(px(draft_sheet_bottom)).pb_1())
            .px_2()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll(|_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .relative()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .rounded(px(14.))
                    .bg(hex_color(pal.surface))
                    .border_1()
                    .border_color(hex_color(pal.border))
                    .shadow_lg()
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                *draft_bounds_tracker.borrow_mut() = Some(bounds);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .when_some(dict_card, |col, card| col.child(card))
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
                        v_flex()
                            .w_full()
                            .gap_1()
                            .children(HighlightDecoration::all().chunks(4).map(|row_styles| {
                                h_flex().w_full().gap_1p5().children(row_styles.iter().copied().map(
                                    |st| {
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
                                                pal.quote_bg
                                            }))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_size(px(11.))
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
                                    },
                                ))
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
            .with_animation(
                "rdr-draft-rise",
                Animation::new(Duration::from_millis(190)).with_easing(ease_out_quint()),
                |el, t| el.mb(px(-28.0 * (1.0 - t))).opacity(t),
            )
    });

    let footer = v_flex()
        .w_full()
        .flex_shrink_0()
        .bg(hex_color(pal.surface))
        .border_t_1()
        .border_color(hex_color(pal.border))
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

    div()
        .relative()
        .size_full()
        .bg(hex_color(pal.bg))
        .text_color(hex_color(pal.fg))
        .child(
            v_flex()
                .size_full()
                .when(!zen_mode, |col| col.child(top_bar))
                .child(main_area)
                .when(!zen_mode, |col| col.child(footer))
                .when(zen_mode && app.safe_area.bottom > 0.0, |col| {
                    col.child(
                        div()
                            .w_full()
                            .h(px(app.safe_area.bottom))
                            .flex_shrink_0()
                            .bg(hex_color(pal.bg)),
                    )
                }),
        )
        .when(!zen_mode && *show_settings, |root| {
            root.child(
                div()
                    .id("rdr-settings-backdrop")
                    .absolute()
                    .top(px(48.))
                    .left_0()
                    .right_0()
                    .child(settings_strip),
            )
        })
        .when_some(floating_draft_sheet, |root, sheet| root.child(sheet))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        BookHighlight, EpubBlock, EpubRun, HighlightColor, HighlightDecoration, edge_tap_direction,
        flatten_block_highlights, hex_color, resolve_draft_dock_top, split_blocks_two_columns,
    };

    #[test]
    fn draft_dock_top_picks_opposite_half_with_hysteresis() {
        // Upper half -> dock bottom (false); lower half -> dock top (true)
        assert!(!resolve_draft_dock_top(Some(150.0), 50.0, 600.0, false, false));
        assert!(resolve_draft_dock_top(Some(450.0), 50.0, 600.0, false, false));

        // Viewport midpoint is 50 + 300 = 350; hysteresis band [330, 370] preserves prior side
        assert!(resolve_draft_dock_top(Some(335.0), 50.0, 600.0, true, true));
        assert!(!resolve_draft_dock_top(Some(335.0), 50.0, 600.0, false, true));
        // Crossing past hysteresis flips dock side
        assert!(!resolve_draft_dock_top(Some(320.0), 50.0, 600.0, true, true));
        assert!(resolve_draft_dock_top(Some(375.0), 50.0, 600.0, false, true));

        // Zero-height viewport and missing selection default safely
        assert!(!resolve_draft_dock_top(Some(200.0), 0.0, 0.0, true, true));
        assert!(!resolve_draft_dock_top(None, 50.0, 600.0, false, false));
        assert!(resolve_draft_dock_top(None, 50.0, 600.0, true, true));
    }

    #[test]
    fn edge_tap_direction_splits_left_center_and_right() {
        assert_eq!(edge_tap_direction(20.0, 400.0), -1);
        assert_eq!(edge_tap_direction(96.0, 400.0), -1);
        assert_eq!(edge_tap_direction(200.0, 400.0), 0);
        assert_eq!(edge_tap_direction(304.0, 400.0), 1);
        assert_eq!(edge_tap_direction(380.0, 400.0), 1);
        assert_eq!(edge_tap_direction(50.0, 0.0), 0);
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

    #[test]
    fn adjacent_chapter_retention_keeps_two_chapter_radius() {
        let mut cache = std::collections::HashMap::new();
        for ch in 0..8usize {
            cache.insert(ch, ch);
        }
        super::super::retain_adjacent_chapters(&mut cache, 4);
        let mut kept: Vec<usize> = cache.keys().copied().collect();
        kept.sort_unstable();
        assert_eq!(kept, vec![2, 3, 4, 5, 6]);
    }
}
