use super::{BookwormMobileApp, LibraryFilter};
use crate::theme::{accent_alpha, hex_alpha, hex_color, kd};
use bookworm_core::library::{BookRecord, ReadingStatus};
use gpui::{prelude::*, *};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex, v_flex};

fn lib_icon(name: IconName, sz: f32, color: u32) -> impl IntoElement {
    Icon::new(name).size(px(sz)).text_color(hex_color(color))
}

fn lib_header_icon_btn(
    id: &'static str,
    icon: IconName,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(34.))
        .h(px(34.))
        .rounded(px(8.))
        .flex_shrink_0()
        .cursor_pointer()
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .flex()
        .items_center()
        .justify_center()
        .on_click(on_click)
        .child(lib_icon(icon, 16., kd::FG))
}

pub fn compute_library_cols(viewport_width: f32, safe_left: f32, safe_right: f32) -> usize {
    let avail = (viewport_width - safe_left - safe_right).max(320.0);
    if avail >= 900.0 {
        4
    } else if avail >= 600.0 {
        3
    } else {
        2
    }
}

fn render_continue_reading_hero(
    book: &BookRecord,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let rel = book.rel_path.clone();
    let title = book.display_title().to_string();
    let author = book.authors_display();
    let pct = book.progress_pct();
    let hl_count = book.user.highlights.len();
    let total_ch = book.user.total_chapters.max(book.chapter_count).max(1);
    let progress_line = if hl_count > 0 {
        format!(
            "Ch {} / {} · {}% · ★{}",
            book.user.chapter_idx + 1,
            total_ch,
            pct,
            hl_count
        )
    } else {
        format!("Ch {} / {} · {}%", book.user.chapter_idx + 1, total_ch, pct)
    };

    let thumb = if let Some(cov) = &book.cover_cache_path {
        div()
            .w(px(58.))
            .h(px(84.))
            .rounded(px(7.))
            .overflow_hidden()
            .flex_shrink_0()
            .bg(hex_color(kd::BG_PANEL))
            .child(img(cov.clone()).size_full().object_fit(ObjectFit::Cover))
            .into_any_element()
    } else {
        v_flex()
            .w(px(58.))
            .h(px(84.))
            .p_1p5()
            .justify_between()
            .flex_shrink_0()
            .rounded(px(7.))
            .bg(hex_color(kd::BG_PANEL))
            .border_1()
            .border_color(hex_color(kd::GREEN))
            .child(lib_icon(IconName::BookOpen, 14., kd::GREEN))
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(hex_color(kd::FG))
                    .truncate()
                    .child(title.clone()),
            )
            .into_any_element()
    };

    v_flex()
        .w_full()
        .px_3()
        .pt_3()
        .gap_1p5()
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::BOLD)
                .text_color(hex_color(kd::GREEN))
                .child("CONTINUE READING"),
        )
        .child(
            h_flex()
                .id("lib-continue-hero")
                .w_full()
                .p_3()
                .gap_3()
                .items_center()
                .rounded(px(12.))
                .bg(hex_color(kd::BG_SIDEBAR))
                .border_1()
                .border_color(hex_color(kd::BORDER))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_book(&rel, window, cx);
                }))
                .child(thumb)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex_color(kd::FG))
                                .truncate()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(hex_color(kd::FG_SUBTLE))
                                .truncate()
                                .child(author),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(hex_color(kd::GREEN))
                                .child(progress_line),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(4.))
                                .rounded(px(2.))
                                .bg(hex_color(kd::BG_PANEL))
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(pct as f32 / 100.0))
                                        .rounded(px(2.))
                                        .bg(hex_color(kd::GREEN)),
                                ),
                        ),
                )
                .child(
                    div()
                        .px_2p5()
                        .py_1p5()
                        .rounded(px(8.))
                        .bg(hex_color(kd::GREEN))
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex_color(kd::BG_MAIN))
                        .child("Resume"),
                ),
        )
}

fn render_book_card(
    book: &BookRecord,
    ix: usize,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let rel = book.rel_path.clone();
    let title = book.display_title().to_string();
    let author = book.authors_display();
    let pct = book.progress_pct();
    let hl_count = book.user.highlights.len();
    let status = book.user.status;

    let (status_bg, status_fg) = match status {
        ReadingStatus::Reading => (hex_alpha(0x87a98728), kd::GREEN),
        ReadingStatus::Finished => (hex_alpha(0x957fb828), kd::ACCENT_HOVER),
        ReadingStatus::OnHold => (hex_alpha(0xc4b28a28), kd::YELLOW),
        ReadingStatus::Unread => (hex_alpha(0x72716924), kd::FG_MUTED),
    };

    let cover_el = if let Some(cov) = &book.cover_cache_path {
        div()
            .w_full()
            .h(px(196.))
            .rounded(px(8.))
            .overflow_hidden()
            .bg(hex_color(kd::BG_PANEL))
            .child(img(cov.clone()).size_full().object_fit(ObjectFit::Cover))
            .into_any_element()
    } else {
        v_flex()
            .w_full()
            .h(px(196.))
            .p_3()
            .justify_between()
            .rounded(px(8.))
            .bg(hex_color(kd::BG_PANEL))
            .border_1()
            .border_color(hex_color(kd::BORDER))
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(hex_color(kd::GREEN))
                    .child("EPUB"),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .text_color(hex_color(kd::FG))
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(hex_color(kd::FG_SUBTLE))
                            .truncate()
                            .child(author.clone()),
                    ),
            )
            .into_any_element()
    };

    div()
        .id(SharedString::from(format!("book-card-{ix}")))
        .flex_1()
        .min_w_0()
        .p_2p5()
        .rounded(px(12.))
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .cursor_pointer()
        .flex()
        .flex_col()
        .gap_2()
        .on_click(cx.listener(move |this, _, window, cx| {
            this.open_book(&rel, window, cx);
        }))
        .child(cover_el)
        .child(
            v_flex()
                .w_full()
                .gap_0p5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(kd::FG))
                        .truncate()
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(hex_color(kd::FG_SUBTLE))
                        .truncate()
                        .child(author),
                ),
        )
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(5.))
                        .bg(status_bg)
                        .text_size(px(10.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(status_fg))
                        .child(status.label()),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(hex_color(kd::FG_MUTED))
                        .child(if hl_count > 0 {
                            format!("{pct}% · ★{hl_count}")
                        } else {
                            format!("{pct}%")
                        }),
                ),
        )
        .child(
            div()
                .w_full()
                .h(px(3.))
                .rounded(px(2.))
                .bg(hex_color(kd::BG_PANEL))
                .child(
                    div()
                        .h_full()
                        .w(relative(pct as f32 / 100.0))
                        .rounded(px(2.))
                        .bg(hex_color(kd::GREEN)),
                ),
        )
}

pub fn render_library_tab(
    app: &BookwormMobileApp,
    window: &Window,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let q = app
        .library_search_input
        .read(cx)
        .value()
        .trim()
        .to_lowercase();
    let filtered: Vec<&BookRecord> = app
        .books
        .iter()
        .filter(|b| {
            let status_ok = match app.library_filter {
                LibraryFilter::All => true,
                LibraryFilter::Reading => b.user.status == ReadingStatus::Reading,
                LibraryFilter::Unread => b.user.status == ReadingStatus::Unread,
                LibraryFilter::Finished => b.user.status == ReadingStatus::Finished,
            };
            if !status_ok {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            b.display_title().to_lowercase().contains(&q)
                || b.authors_display().to_lowercase().contains(&q)
                || b.all_tags().iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect();

    let continue_book = if q.is_empty() && app.library_filter == LibraryFilter::All {
        app.most_recent_in_progress_book().cloned()
    } else {
        None
    };

    let cols = compute_library_cols(
        f32::from(window.viewport_size().width),
        app.safe_area.left,
        app.safe_area.right,
    );

    let books_root_display = app.books_root.display().to_string();

    let header = h_flex()
        .w_full()
        .px_3()
        .py_2p5()
        .bg(hex_color(kd::BG_DEEP))
        .border_b_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .justify_between()
        .items_center()
        .gap_2()
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(32.))
                        .h(px(32.))
                        .rounded(px(8.))
                        .bg(hex_alpha(0x87a98728))
                        .border_1()
                        .border_color(hex_color(kd::GREEN))
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_shrink_0()
                        .child(lib_icon(IconName::BookOpen, 16., kd::GREEN)),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex_color(kd::FG))
                                .truncate()
                                .child("Bookworm"),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(hex_color(kd::FG_SUBTLE))
                                .child(if app.library_scanning {
                                    "Scanning for EPUBs…".to_string()
                                } else {
                                    format!("{} books", app.books.len())
                                }),
                        ),
                ),
        )
        .child(
            h_flex()
                .gap_1p5()
                .flex_shrink_0()
                .child(lib_header_icon_btn(
                    "lib-pick-root",
                    IconName::Folder,
                    cx.listener(|this, _, window, cx| {
                        this.open_folder_picker(window, cx);
                    }),
                ))
                .child(lib_header_icon_btn(
                    "lib-refresh",
                    IconName::RefreshCw,
                    cx.listener(|this, _, _, cx| {
                        this.refresh_library_async(cx);
                    }),
                ))
                .child(lib_header_icon_btn(
                    "lib-settings",
                    IconName::Settings,
                    cx.listener(|this, _, window, cx| {
                        this.open_surface_settings(window, cx);
                    }),
                )),
        );

    let root_bar = div()
        .id("lib-root-strip")
        .w_full()
        .px_3()
        .py_2()
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_b_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .on_click(cx.listener(|this, _, window, cx| {
            this.open_folder_picker(window, cx);
        }))
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_2()
                .child(lib_icon(IconName::Folder, 14., kd::GREEN))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex_color(kd::FG_MUTED))
                                .child("BOOKS ROOT"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(hex_color(kd::FG))
                                .truncate()
                                .child(books_root_display.clone()),
                        ),
                ),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .rounded(px(6.))
                .flex_shrink_0()
                .bg(accent_alpha(0x2e))
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(hex_color(kd::ACCENT_HOVER))
                .child("Change"),
        );

    let controls = v_flex()
        .w_full()
        .px_3()
        .py_2()
        .gap_2()
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_b_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .child(Input::new(&app.library_search_input).small().cleanable(true))
        .child(
            h_flex()
                .w_full()
                .gap_1p5()
                .children(LibraryFilter::all().iter().copied().map(|f| {
                    let active = app.library_filter == f;
                    div()
                        .id(SharedString::from(format!("lib-flt-{}", f.label())))
                        .px_2p5()
                        .py_1()
                        .rounded(px(8.))
                        .cursor_pointer()
                        .bg(hex_color(if active {
                            kd::GREEN
                        } else {
                            kd::BG_PANEL
                        }))
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(if active {
                            kd::BG_MAIN
                        } else {
                            kd::FG_SUBTLE
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.library_filter = f;
                            cx.notify();
                        }))
                        .child(f.label())
                })),
        );

    let grid = if filtered.is_empty() {
        v_flex()
            .w_full()
            .p_4()
            .gap_3()
            .child(
                v_flex()
                    .w_full()
                    .p_4()
                    .gap_2p5()
                    .rounded(px(12.))
                    .bg(hex_color(kd::BG_SIDEBAR))
                    .border_1()
                    .border_color(hex_color(kd::BORDER_SUBTLE))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(hex_color(kd::FG))
                            .child("No EPUB books found here"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(hex_color(kd::FG_SUBTLE))
                            .child(format!(
                                "Currently scanning: {books_root_display}. Pick another folder or create a sample book."
                            )),
                    )
                    .child(
                        h_flex()
                            .pt_1()
                            .gap_2()
                            .flex_wrap()
                            .child(
                                Button::new("lib-empty-pick-root")
                                    .primary()
                                    .small()
                                    .label("Choose Books Folder")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_folder_picker(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("lib-empty-sample")
                                    .secondary()
                                    .small()
                                    .label("Create Sample EPUB")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.create_sample_book(cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    } else {
        let mut content = v_flex().w_full().gap_2();
        if let Some(hero) = continue_book {
            content = content.child(render_continue_reading_hero(&hero, cx));
        }
        let mut rows = v_flex().w_full().p_3().gap_3();
        let chunks = filtered.chunks(cols);
        for (row_ix, chunk) in chunks.enumerate() {
            let mut row = h_flex().w_full().gap_2p5().items_stretch();
            for (c_ix, book) in chunk.iter().enumerate() {
                let global_ix = row_ix * cols + c_ix;
                row = row.child(render_book_card(book, global_ix, cx));
            }
            for _ in chunk.len()..cols {
                row = row.child(div().flex_1());
            }
            rows = rows.child(row);
        }
        content.child(rows).into_any_element()
    };

    v_flex()
        .size_full()
        .child(header)
        .child(root_bar)
        .when(!app.has_storage_access, |col| {
            col.child(
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .justify_between()
                    .items_center()
                    .bg(hex_alpha(0xc4b28a20))
                    .border_b_1()
                    .border_color(hex_color(kd::YELLOW))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(hex_color(kd::YELLOW))
                            .child("All files access needed to read EPUBs & highlights in /sdcard"),
                    )
                    .child(
                        Button::new("lib-grant-storage")
                            .primary()
                            .small()
                            .label("Grant Access")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_storage_permission(cx);
                            })),
                    ),
            )
        })
        .when_some(app.status_banner.clone(), |col, msg| {
            col.child(
                h_flex()
                    .w_full()
                    .px_3()
                    .py_1p5()
                    .bg(hex_color(kd::BG_DEEP))
                    .border_b_1()
                    .border_color(hex_color(kd::BORDER_SUBTLE))
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(hex_color(kd::GREEN))
                            .truncate()
                            .child(msg),
                    )
                    .child(
                        div()
                            .id("lib-banner-close")
                            .cursor_pointer()
                            .p_1()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.status_banner = None;
                                cx.notify();
                            }))
                            .child(lib_icon(IconName::Close, 13., kd::FG_MUTED)),
                    ),
            )
        })
        .child(controls)
        .child(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(
                    div()
                        .id("library-grid-scroll")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&app.library_scroll)
                        .child(grid),
                )
                .child(Scrollbar::vertical(&app.library_scroll)),
        )
}

#[cfg(test)]
mod tests {
    use super::compute_library_cols;

    #[test]
    fn library_cols_adapt_from_phone_to_tablet() {
        assert_eq!(compute_library_cols(412.0, 0.0, 0.0), 2);
        assert_eq!(compute_library_cols(680.0, 0.0, 0.0), 3);
        assert_eq!(compute_library_cols(1024.0, 0.0, 0.0), 4);
    }
}
