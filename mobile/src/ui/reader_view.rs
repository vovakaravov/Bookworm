use super::image_util::decode_bytes_image;
use super::{BookwormMobileApp, ReaderState};
use crate::theme::{ReaderPalette, ReaderTheme, hex_alpha, hex_color};
use bookworm_core::epub::{EpubBlock, EpubRun};
use bookworm_core::library::{BookHighlight, HighlightColor, HighlightDecoration, clamp_to_char_boundary};
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

fn highlight_fill_alpha(color: HighlightColor, light_paper: bool) -> Hsla {
    let alpha = if light_paper { 0x5c } else { 0x48 };
    hex_alpha((color.rgb_hex() << 8) | alpha)
}

pub fn flatten_block_highlights(
    text: &str,
    runs: &[EpubRun],
    block_hls: &[(&BookHighlight, Range<usize>)],
    code_bg: Hsla,
    code_fg: Hsla,
    light_paper: bool,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut pts = Vec::with_capacity((runs.len() + block_hls.len()) * 2);
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

        if base_run.is_none() && matching_hls.is_empty() {
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

fn render_paragraph_text(
    blk_idx: usize,
    text: String,
    runs: &[EpubRun],
    highlights: &[BookHighlight],
    chapter_idx: usize,
    code_bg: Hsla,
    code_fg: Hsla,
    light_paper: bool,
    rendered_layouts: &Rc<RefCell<Vec<(usize, usize, TextLayout)>>>,
) -> AnyElement {
    let text_len = text.len();
    let block_hls: Vec<(&BookHighlight, Range<usize>)> = highlights
        .iter()
        .filter_map(|hl| {
            hl.block_byte_range(chapter_idx, blk_idx, text_len)
                .map(|r| (hl, r))
        })
        .collect();
    let flat = flatten_block_highlights(&text, runs, &block_hls, code_bg, code_fg, light_paper);
    let st = if flat.is_empty() {
        StyledText::new(text)
    } else {
        StyledText::new(text).with_highlights(flat)
    };
    rendered_layouts
        .borrow_mut()
        .push((blk_idx, text_len, st.layout().clone()));
    st.into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_chapter_block(
    b_ix: usize,
    blk: &EpubBlock,
    selected: bool,
    ch_idx: usize,
    font_size: f32,
    pal: ReaderPalette,
    light_paper: bool,
    highlights: &[BookHighlight],
    rendered_layouts: &Rc<RefCell<Vec<(usize, usize, TextLayout)>>>,
    decoded_images: &RefCell<HashMap<(usize, usize), Arc<Image>>>,
    cx: &mut Context<BookwormMobileApp>,
) -> AnyElement {
    let code_bg = hex_color(pal.surface);
    let code_fg = hex_color(pal.accent);
    match blk {
        EpubBlock::Heading { level, text } => {
            let size = match level {
                1 => (font_size + 7.0).min(34.0),
                2 => (font_size + 4.5).min(30.0),
                _ => (font_size + 2.0).min(26.0),
            };
            let rendered = render_paragraph_text(
                b_ix,
                text.clone(),
                &[],
                highlights,
                ch_idx,
                code_bg,
                code_fg,
                light_paper,
                rendered_layouts,
            );
            div()
                .id(SharedString::from(format!("blk-{b_ix}")))
                .pt_2()
                .pb_1()
                .text_size(px(size))
                .font_weight(FontWeight::BOLD)
                .text_color(hex_color(pal.fg))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.reader_open_highlight_draft(b_ix, window, cx);
                }))
                .child(rendered)
                .into_any_element()
        }
        EpubBlock::Paragraph {
            text,
            runs,
            is_quote,
        } => {
            let rendered = render_paragraph_text(
                b_ix,
                text.clone(),
                runs,
                highlights,
                ch_idx,
                code_bg,
                code_fg,
                light_paper,
                rendered_layouts,
            );
            div()
                .id(SharedString::from(format!("blk-{b_ix}")))
                .w_full()
                .px_1p5()
                .py_1()
                .rounded(px(6.))
                .cursor_pointer()
                .when(*is_quote, |el| {
                    el.pl_3()
                        .border_l_2()
                        .border_color(hex_color(pal.accent))
                        .bg(hex_color(pal.quote_bg))
                })
                .when(selected, |el| {
                    el.border_1().border_color(hex_color(pal.accent))
                })
                .text_size(px(font_size))
                .line_height(relative(1.62))
                .text_color(hex_color(if *is_quote { pal.muted } else { pal.fg }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.reader_open_highlight_draft(b_ix, window, cx);
                }))
                .child(rendered)
                .into_any_element()
        }
        EpubBlock::ListItem { text, runs } => {
            let rendered = render_paragraph_text(
                b_ix,
                text.clone(),
                runs,
                highlights,
                ch_idx,
                code_bg,
                code_fg,
                light_paper,
                rendered_layouts,
            );
            h_flex()
                .id(SharedString::from(format!("blk-{b_ix}")))
                .w_full()
                .items_start()
                .gap_2()
                .pl_2()
                .py_0p5()
                .rounded(px(6.))
                .cursor_pointer()
                .when(selected, |el| {
                    el.border_1().border_color(hex_color(pal.accent))
                })
                .text_size(px(font_size))
                .line_height(relative(1.58))
                .text_color(hex_color(pal.fg))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.reader_open_highlight_draft(b_ix, window, cx);
                }))
                .child(div().text_color(hex_color(pal.accent)).child("•"))
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
            div()
                .w_full()
                .h(px(220.))
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

pub fn render_reader_view(
    app: &mut BookwormMobileApp,
    window: &Window,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    app.maybe_schedule_reader_scroll_save(cx);
    let wide_viewport = f32::from(window.viewport_size().width) >= 700.0;
    let use_two_col = app.two_column_spread && wide_viewport;

    let Some(ReaderState {
        book_rel,
        book_title,
        book,
        chapter_idx,
        show_settings,
        show_toc,
        show_notes,
        notes_color_filter,
        notes_only_filter,
        draft,
        rendered_layouts,
        decoded_images,
        ..
    }) = &app.reader
    else {
        return div().size_full().into_any_element();
    };

    let pal = app.reader_theme.palette();
    let light_paper = matches!(app.reader_theme, ReaderTheme::Sepia | ReaderTheme::Paper);
    let user_state = app
        .library_store
        .books
        .get(book_rel)
        .cloned()
        .unwrap_or_default();
    let total_ch = book.chapters.len().max(1);
    let ch_idx = (*chapter_idx).min(total_ch.saturating_sub(1));
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
                                            .child("Tap any paragraph while reading to highlight it in 5 Moon+ Reader colors and attach a margin note."),
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
        let body_el = if let Some(ch) = chapter {
            if use_two_col && ch.blocks.len() >= 2 {
                let split_idx = split_blocks_two_columns(&ch.blocks);
                let mut left_col = v_flex().flex_1().min_w_0().gap_3p5();
                let mut right_col = v_flex().flex_1().min_w_0().gap_3p5();
                for (b_ix, blk) in ch.blocks.iter().enumerate() {
                    let selected = draft.as_ref().is_some_and(|d| d.block_idx == b_ix);
                    let el = render_chapter_block(
                        b_ix,
                        blk,
                        selected,
                        ch_idx,
                        app.reader_font_size,
                        pal,
                        light_paper,
                        &user_state.highlights,
                        rendered_layouts,
                        decoded_images,
                        cx,
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
                    let selected = draft.as_ref().is_some_and(|d| d.block_idx == b_ix);
                    col = col.child(render_chapter_block(
                        b_ix,
                        blk,
                        selected,
                        ch_idx,
                        app.reader_font_size,
                        pal,
                        light_paper,
                        &user_state.highlights,
                        rendered_layouts,
                        decoded_images,
                        cx,
                    ));
                }
                col.into_any_element()
            }
        } else {
            v_flex().flex_none().w_full().px_5().py_4().into_any_element()
        };

        div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                v_flex()
                    .id("epub-chapter-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&app.reader_scroll)
                    .child(body_el),
            )
            .child(Scrollbar::vertical(&app.reader_scroll))
            .into_any_element()
    };

    let pct = (((ch_idx + 1) as f32 / total_ch as f32) * 100.0).round() as u8;
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
                                Button::new("hl-cancel")
                                    .ghost()
                                    .small()
                                    .label("Cancel")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(r) = &mut this.reader {
                                            r.draft = None;
                                        }
                                        window.blur(cx);
                                        gpui_mobile::hide_keyboard();
                                        cx.notify();
                                    })),
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
                            if ch_idx > 0 {
                                this.reader_go_chapter(ch_idx - 1, window, cx);
                            }
                        })),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(pal.muted))
                        .child(format!("Ch {} / {} · {}%", ch_idx + 1, total_ch, pct)),
                )
                .child(
                    Button::new("rdr-next")
                        .secondary()
                        .small()
                        .label("Next →")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if ch_idx + 1 < total_ch {
                                this.reader_go_chapter(ch_idx + 1, window, cx);
                            }
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
            hex_color(0x222222),
            hex_color(0xeeeeee),
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
