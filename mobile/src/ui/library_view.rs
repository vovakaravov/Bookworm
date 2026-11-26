use super::{BookwormMobileApp, LibraryFilter};
use crate::config::{LibraryGridSize, LibrarySort};
use crate::theme::{accent_alpha, hex_alpha, hex_color, kd};
use bookworm_core::library::{BookRecord, ReadingStatus};
use gpui::{prelude::*, *};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex, v_flex};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

fn lib_icon(name: IconName, sz: f32, color: u32) -> impl IntoElement {
    Icon::new(name).size(px(sz)).text_color(hex_color(color))
}

fn long_press_overlay(
    on_trigger: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_trigger = std::rc::Rc::new(on_trigger);
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal),
        move |_bounds, hitbox, window, _cx| {
            let on_trigger = on_trigger.clone();
            window.on_mouse_event(move |ev: &gpui::LongPressEvent, phase, window, cx| {
                if phase == gpui::DispatchPhase::Bubble
                    && ev.phase == gpui::TouchPhase::Started
                    && !window.default_prevented()
                    && hitbox.is_hovered(window)
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    on_trigger(ev.position, window, cx);
                }
            });
        },
    )
    .absolute()
    .inset_0()
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

pub fn compute_library_cols(
    viewport_width: f32,
    safe_left: f32,
    safe_right: f32,
    grid_size: LibraryGridSize,
) -> usize {
    let avail = (viewport_width - safe_left - safe_right).max(320.0);
    match grid_size {
        LibraryGridSize::Small => {
            if avail >= 900.0 {
                6
            } else if avail >= 600.0 {
                4
            } else {
                3
            }
        }
        LibraryGridSize::Medium => {
            if avail >= 900.0 {
                4
            } else if avail >= 600.0 {
                3
            } else {
                2
            }
        }
        LibraryGridSize::Large => {
            if avail >= 900.0 {
                3
            } else if avail >= 600.0 {
                2
            } else {
                1
            }
        }
    }
}

fn render_continue_reading_dock(
    book: &BookRecord,
    safe_bottom: f32,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let rel = book.rel_path.clone();
    let rel_lp = book.rel_path.clone();
    let app_weak_dock = cx.entity().downgrade();
    let title = book.display_title().to_string();
    let pct = book.progress_pct();
    let hl_count = book.user.highlights.len();
    let total_ch = book.user.total_chapters.max(book.chapter_count).max(1);
    let loc_str = if book.user.total_pages > 0 && book.user.current_page > 0 {
        format!(
            "Page {} / {} · {}%",
            book.user.current_page, book.user.total_pages, pct
        )
    } else {
        format!("Ch {} / {} · {}%", book.user.chapter_idx + 1, total_ch, pct)
    };
    let subtitle = if hl_count > 0 {
        format!("Continue · {loc_str} · ★{hl_count}")
    } else {
        format!("Continue · {loc_str}")
    };

    let thumb = if let Some(cov) = &book.cover_cache_path {
        div()
            .w(px(30.))
            .h(px(42.))
            .rounded(px(4.))
            .overflow_hidden()
            .flex_shrink_0()
            .bg(hex_color(kd::BG_PANEL))
            .child(img(cov.clone()).size_full().object_fit(ObjectFit::Cover))
            .into_any_element()
    } else {
        div()
            .w(px(30.))
            .h(px(42.))
            .rounded(px(4.))
            .flex_shrink_0()
            .bg(hex_color(kd::BG_PANEL))
            .border_1()
            .border_color(hex_color(kd::GREEN))
            .flex()
            .items_center()
            .justify_center()
            .child(lib_icon(IconName::BookOpen, 14., kd::GREEN))
            .into_any_element()
    };

    v_flex()
        .w_full()
        .flex_shrink_0()
        .bg(hex_color(kd::BG_DEEP))
        .border_t_1()
        .border_color(hex_color(kd::BORDER))
        .child(
            div()
                .w_full()
                .h(px(2.5))
                .bg(hex_color(kd::BG_PANEL))
                .child(
                    div()
                        .h_full()
                        .w(relative(pct as f32 / 100.0))
                        .bg(hex_color(kd::GREEN)),
                ),
        )
        .child(
            h_flex()
                .id("lib-continue-dock")
                .relative()
                .w_full()
                .px_3()
                .py_2()
                .gap_2p5()
                .items_center()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_book(&rel, window, cx);
                }))
                .child(long_press_overlay(move |_, window, cx| {
                    let _ = app_weak_dock.update(cx, |this, cx| {
                        this.open_book_context_menu(rel_lp.clone(), window, cx);
                    });
                }))
                .child(thumb)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex_color(kd::FG))
                                .truncate()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(hex_color(kd::GREEN))
                                .truncate()
                                .child(subtitle),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .px_2p5()
                        .py_1()
                        .rounded(px(7.))
                        .bg(hex_color(kd::GREEN))
                        .text_size(px(11.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex_color(kd::BG_MAIN))
                        .child("Resume"),
                ),
        )
        .when(safe_bottom > 0.0, |col| {
            col.child(
                div()
                    .w_full()
                    .h(px(safe_bottom))
                    .flex_shrink_0()
                    .bg(hex_color(kd::BG_DEEP)),
            )
        })
}

fn render_book_card_in_row(
    book: &BookRecord,
    ix: usize,
    grid_size: LibraryGridSize,
    cols: usize,
    app_weak: AnyWeakEntity,
) -> impl IntoElement {
    let rel = book.rel_path.clone();
    let rel_lp = book.rel_path.clone();
    let weak_click = app_weak.clone();
    let weak_lp = app_weak;
    let title = book.display_title().to_string();
    let author = book.authors_display();
    let pct = book.progress_pct();
    let hl_count = book.user.highlights.len();
    let status = book.user.status;
    let total_ch = book.user.total_chapters.max(book.chapter_count).max(1);

    let (status_bg, status_fg) = match status {
        ReadingStatus::Reading => (hex_alpha(0x87a98728), kd::GREEN),
        ReadingStatus::Finished => (hex_alpha(0x957fb828), kd::ACCENT_HOVER),
        ReadingStatus::OnHold => (hex_alpha(0xc4b28a28), kd::YELLOW),
        ReadingStatus::Unread => (hex_alpha(0x72716924), kd::FG_MUTED),
    };

    let pct_str = if hl_count > 0 {
        format!("{pct}% · ★{hl_count}")
    } else {
        format!("{pct}%")
    };

    if cols == 1 {
        let cover_el = if let Some(cov) = &book.cover_cache_path {
            div()
                .w(px(84.))
                .h(px(124.))
                .rounded(px(8.))
                .overflow_hidden()
                .flex_shrink_0()
                .bg(hex_color(kd::BG_PANEL))
                .child(img(cov.clone()).size_full().object_fit(ObjectFit::Cover))
                .into_any_element()
        } else {
            v_flex()
                .w(px(84.))
                .h(px(124.))
                .p_2()
                .justify_between()
                .flex_shrink_0()
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
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex_color(kd::FG))
                        .truncate()
                        .child(title.clone()),
                )
                .into_any_element()
        };

        return div()
            .id(SharedString::from(format!("book-card-{ix}")))
            .relative()
            .flex_1()
            .min_w_0()
            .p_2p5()
            .rounded(px(12.))
            .bg(hex_color(kd::BG_SIDEBAR))
            .border_1()
            .border_color(hex_color(kd::BORDER_SUBTLE))
            .cursor_pointer()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .on_click(move |_, window, cx| {
                if let Some(any_ent) = weak_click.upgrade()
                    && let Ok(ent) = any_ent.downcast::<BookwormMobileApp>()
                {
                    ent.update(cx, |this, cx| {
                        this.open_book(&rel, window, cx);
                    });
                }
            })
            .child(long_press_overlay(move |_, window, cx| {
                if let Some(any_ent) = weak_lp.upgrade()
                    && let Ok(ent) = any_ent.downcast::<BookwormMobileApp>()
                {
                    ent.update(cx, |this, cx| {
                        this.open_book_context_menu(rel_lp.clone(), window, cx);
                    });
                }
            }))
            .child(cover_el)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1p5()
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
                            .text_color(hex_color(kd::FG_MUTED))
                            .child(if book.user.total_pages > 0 && book.user.current_page > 0 {
                                format!(
                                    "Page {} / {} · {pct_str}",
                                    book.user.current_page, book.user.total_pages
                                )
                            } else {
                                format!(
                                    "Ch {} / {} · {pct_str}",
                                    book.user.chapter_idx + 1,
                                    total_ch
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
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
                                    .flex_1()
                                    .h(px(3.5))
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
                    ),
            )
            .into_any_element();
    }

    let (cover_h, compact_meta) = match grid_size {
        LibraryGridSize::Small => (if cols >= 4 { 150.0 } else { 136.0 }, true),
        LibraryGridSize::Medium => (if cols >= 3 { 198.0 } else { 188.0 }, false),
        LibraryGridSize::Large => (236.0, false),
    };

    let cover_el = if let Some(cov) = &book.cover_cache_path {
        div()
            .w_full()
            .h(px(cover_h))
            .rounded(px(8.))
            .overflow_hidden()
            .bg(hex_color(kd::BG_PANEL))
            .child(img(cov.clone()).size_full().object_fit(ObjectFit::Cover))
            .into_any_element()
    } else {
        v_flex()
            .w_full()
            .h(px(cover_h))
            .p_2p5()
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
                    .gap_0p5()
                    .child(
                        div()
                            .text_size(px(if compact_meta { 12. } else { 13.5 }))
                            .font_weight(FontWeight::BOLD)
                            .text_color(hex_color(kd::FG))
                            .truncate()
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(if compact_meta { 10.5 } else { 11.5 }))
                            .text_color(hex_color(kd::FG_SUBTLE))
                            .truncate()
                            .child(author.clone()),
                    ),
            )
            .into_any_element()
    };

    div()
        .id(SharedString::from(format!("book-card-{ix}")))
        .relative()
        .flex_1()
        .min_w_0()
        .p(px(if compact_meta { 6. } else { 10. }))
        .rounded(px(12.))
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .cursor_pointer()
        .flex()
        .flex_col()
        .gap(px(if compact_meta { 5. } else { 8. }))
        .on_click(move |_, window, cx| {
            if let Some(any_ent) = weak_click.upgrade()
                && let Ok(ent) = any_ent.downcast::<BookwormMobileApp>()
            {
                ent.update(cx, |this, cx| {
                    this.open_book(&rel, window, cx);
                });
            }
        })
        .child(long_press_overlay(move |_, window, cx| {
            if let Some(any_ent) = weak_lp.upgrade()
                && let Ok(ent) = any_ent.downcast::<BookwormMobileApp>()
            {
                ent.update(cx, |this, cx| {
                    this.open_book_context_menu(rel_lp.clone(), window, cx);
                });
            }
        }))
        .child(cover_el)
        .child(
            v_flex()
                .w_full()
                .gap_0p5()
                .child(
                    div()
                        .text_size(px(if compact_meta { 11.5 } else { 13.5 }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(kd::FG))
                        .truncate()
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(if compact_meta { 10. } else { 11.5 }))
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
                        .text_size(px(if compact_meta { 9. } else { 10. }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex_color(status_fg))
                        .child(status.label()),
                )
                .child(
                    div()
                        .text_size(px(if compact_meta { 9.5 } else { 10.5 }))
                        .text_color(hex_color(kd::FG_MUTED))
                        .child(pct_str),
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
        .into_any_element()
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
    let mut filtered: Vec<&BookRecord> = app
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
            if let Some(shelf) = &app.library_shelf_filter
                && !b.all_tags().iter().any(|t| t == shelf)
            {
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

    filtered.sort_by(|a, b| match app.library_sort {
        LibrarySort::Recent => {
            let ta = a.user.last_opened.as_deref().unwrap_or("");
            let tb = b.user.last_opened.as_deref().unwrap_or("");
            tb.cmp(ta).then_with(|| {
                a.display_title()
                    .to_lowercase()
                    .cmp(&b.display_title().to_lowercase())
            })
        }
        LibrarySort::Title => a
            .display_title()
            .to_lowercase()
            .cmp(&b.display_title().to_lowercase()),
        LibrarySort::Author => a
            .authors_display()
            .to_lowercase()
            .cmp(&b.authors_display().to_lowercase())
            .then_with(|| {
                a.display_title()
                    .to_lowercase()
                    .cmp(&b.display_title().to_lowercase())
            }),
        LibrarySort::Progress => b
            .progress_pct()
            .cmp(&a.progress_pct())
            .then_with(|| {
                let ta = a.user.last_opened.as_deref().unwrap_or("");
                let tb = b.user.last_opened.as_deref().unwrap_or("");
                tb.cmp(ta)
            }),
    });

    let continue_book = if q.is_empty()
        && app.library_filter == LibraryFilter::All
        && app.library_shelf_filter.is_none()
    {
        app.most_recent_in_progress_book().cloned()
    } else {
        None
    };

    let grid_size = app.library_grid_size;
    let cols = compute_library_cols(
        f32::from(window.viewport_size().width),
        app.safe_area.left,
        app.safe_area.right,
        grid_size,
    );

    let books_root_display = app.books_root.display().to_string();

    let header = h_flex()
        .w_full()
        .px_3()
        .py_2()
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
                .child(lib_header_icon_btn(
                    "lib-drawer-btn",
                    IconName::Menu,
                    cx.listener(|this, _, window, cx| {
                        this.toggle_library_drawer(!this.library_drawer_open, window, cx);
                    }),
                ))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex_color(kd::FG))
                                .truncate()
                                .child("Bookworm"),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
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

    let controls = v_flex()
        .w_full()
        .px_3()
        .py_1p5()
        .gap_1p5()
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_b_1()
        .border_color(hex_color(kd::BORDER_SUBTLE))
        .child(Input::new(&app.library_search_input).small().cleanable(true))
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_1p5()
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .children(LibraryFilter::all().iter().copied().map(|f| {
                            let active = app.library_filter == f;
                            div()
                                .id(SharedString::from(format!("lib-flt-{}", f.label())))
                                .px_2()
                                .py_1()
                                .rounded(px(7.))
                                .cursor_pointer()
                                .bg(hex_color(if active {
                                    kd::GREEN
                                } else {
                                    kd::BG_PANEL
                                }))
                                .text_size(px(11.))
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
                )
                .child(
                    h_flex()
                        .id("lib-grid-size-group")
                        .flex_shrink_0()
                        .p_0p5()
                        .rounded(px(8.))
                        .bg(hex_color(kd::BG_PANEL))
                        .border_1()
                        .border_color(hex_color(kd::BORDER_SUBTLE))
                        .gap_0p5()
                        .children(LibraryGridSize::all().iter().copied().map(|sz| {
                            let active = grid_size == sz;
                            div()
                                .id(SharedString::from(format!("lib-grid-size-{}", sz.key())))
                                .min_w(px(24.))
                                .h(px(24.))
                                .px_1p5()
                                .rounded(px(6.))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(if active {
                                    accent_alpha(0x3a)
                                } else {
                                    gpui::transparent_black()
                                })
                                .text_size(px(10.5))
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex_color(if active {
                                    kd::ACCENT_HOVER
                                } else {
                                    kd::FG_MUTED
                                }))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.set_library_grid_size(sz, cx);
                                }))
                                .child(sz.short_label())
                        })),
                ),
        )
        .when_some(app.library_shelf_filter.clone(), |col, shelf| {
            col.child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .id("lib-active-shelf-chip")
                            .px_2p5()
                            .py_1()
                            .rounded(px(7.))
                            .cursor_pointer()
                            .bg(accent_alpha(0x28))
                            .border_1()
                            .border_color(accent_alpha(0x80))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(hex_color(kd::ACCENT_HOVER))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_library_shelf_filter(None, cx);
                            }))
                            .child(format!("Shelf: {shelf}  ×")),
                    ),
            )
        });

    let grid = if filtered.is_empty() {
        div()
            .id("library-empty-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&app.library_scroll)
            .child(
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
                    ),
            )
            .into_any_element()
    } else {
        let books_rc: Rc<Vec<BookRecord>> = Rc::new(filtered.into_iter().cloned().collect());
        let row_count = (books_rc.len() + cols - 1) / cols;
        let app_weak_grid: AnyWeakEntity = cx.entity().downgrade().into();
        let list_id = SharedString::from(format!("library-book-rows-{}-{cols}", grid_size.key()));

        div()
            .relative()
            .size_full()
            .pt_2()
            .child(
                uniform_list(list_id, row_count, move |range, _, _| {
                    range
                        .map(|row_ix| {
                            let start = row_ix * cols;
                            let end = (start + cols).min(books_rc.len());
                            let mut row = h_flex()
                                .w_full()
                                .px_3()
                                .pb_2p5()
                                .gap_2()
                                .items_stretch();
                            for ix in start..end {
                                row = row.child(render_book_card_in_row(
                                    &books_rc[ix],
                                    ix,
                                    grid_size,
                                    cols,
                                    app_weak_grid.clone(),
                                ));
                            }
                            for _ in (end - start)..cols {
                                row = row.child(div().flex_1());
                            }
                            row
                        })
                        .collect()
                })
                .size_full()
                .track_scroll(&app.library_rows_scroll),
            )
            .child(Scrollbar::vertical(&app.library_rows_scroll))
            .into_any_element()
    };

    let has_continue = continue_book.is_some();

    v_flex()
        .size_full()
        .child(header)
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
                .child(grid),
        )
        .when_some(continue_book.as_ref(), |col, hero| {
            col.child(render_continue_reading_dock(hero, app.safe_area.bottom, cx))
        })
        .when(!has_continue && app.safe_area.bottom > 0.0, |col| {
            col.child(div().w_full().h(px(app.safe_area.bottom)).flex_shrink_0())
        })
}

fn book_menu_action_row(
    id: &'static str,
    icon: IconName,
    label: String,
    accent: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    h_flex()
        .id(id)
        .w_full()
        .h(px(44.))
        .px_3()
        .gap_2p5()
        .items_center()
        .rounded(px(10.))
        .bg(hex_color(if accent { kd::ACCENT } else { kd::BG_PANEL }))
        .border_1()
        .border_color(hex_color(if accent { kd::ACCENT } else { kd::BORDER_SUBTLE }))
        .cursor_pointer()
        .on_click(on_click)
        .child(lib_icon(
            icon,
            16.,
            if accent { kd::BG_MAIN } else { kd::FG },
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(hex_color(if accent { kd::BG_MAIN } else { kd::FG }))
                .truncate()
                .child(label),
        )
}

pub fn render_book_context_menu(
    app: &BookwormMobileApp,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let Some(rel_path) = app.book_context_menu.clone() else {
        return div().into_any_element();
    };
    let Some(book) = app.books.iter().find(|b| b.rel_path == rel_path) else {
        return div().into_any_element();
    };

    let title = book.display_title().to_string();
    let author = book.authors_display();
    let pct = book.progress_pct();
    let hl_count = book.user.highlights.len() + book.user.bookmarks.len();
    let current_status = book.user.status;

    let rel_open = rel_path.clone();
    let rel_hl = rel_path.clone();
    let rel_exp = rel_path.clone();

    v_flex()
        .id("book-ctx-backdrop")
        .absolute()
        .inset_0()
        .bg(hex_alpha(0x0000009a))
        .justify_end()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(|this, _, _, cx| {
            this.close_book_context_menu(cx);
        }))
        .child(
            v_flex()
                .id("book-ctx-sheet")
                .w_full()
                .p_4()
                .pb(px(16.0 + app.safe_area.bottom))
                .gap_3()
                .rounded_t(px(18.))
                .bg(hex_color(kd::BG_SIDEBAR))
                .border_t_1()
                .border_color(hex_color(kd::BORDER))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            div()
                                .text_base()
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
                                .child(format!("{author} · {pct}% · ★{hl_count}")),
                        ),
                )
                .child(book_menu_action_row(
                    "bctx-open",
                    IconName::BookOpen,
                    if pct > 0 {
                        "Resume Reading".to_string()
                    } else {
                        "Open Book".to_string()
                    },
                    true,
                    cx.listener(move |this, _, window, cx| {
                        this.book_context_menu = None;
                        this.open_book(&rel_open, window, cx);
                    }),
                ))
                .child(
                    v_flex()
                        .gap_1p5()
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(hex_color(kd::FG_MUTED))
                                .child("READING STATUS"),
                        )
                        .child(
                            h_flex().w_full().gap_1p5().children(
                                ReadingStatus::all().iter().copied().map(|st| {
                                    let active = current_status == st;
                                    let rel_st = rel_path.clone();
                                    div()
                                        .id(SharedString::from(format!("bctx-st-{}", st.label())))
                                        .flex_1()
                                        .py_2()
                                        .rounded(px(8.))
                                        .cursor_pointer()
                                        .bg(hex_color(if active {
                                            kd::ACCENT
                                        } else {
                                            kd::BG_PANEL
                                        }))
                                        .border_1()
                                        .border_color(hex_color(if active {
                                            kd::ACCENT
                                        } else {
                                            kd::BORDER_SUBTLE
                                        }))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(hex_color(if active {
                                            kd::BG_MAIN
                                        } else {
                                            kd::FG
                                        }))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.set_book_status(&rel_st, st, cx);
                                        }))
                                        .child(st.label())
                                }),
                            ),
                        ),
                )
                .child(book_menu_action_row(
                    "bctx-highlights",
                    IconName::File,
                    format!("View Highlights & Notes ({hl_count})"),
                    false,
                    cx.listener(move |this, _, window, cx| {
                        this.open_book_highlights(&rel_hl, window, cx);
                    }),
                ))
                .child(book_menu_action_row(
                    "bctx-export",
                    IconName::Folder,
                    "Export Notes to Markdown".to_string(),
                    false,
                    cx.listener(move |this, _, _, cx| {
                        this.export_single_book_notes(&rel_exp, cx);
                    }),
                ))
                .child(
                    Button::new("bctx-cancel")
                        .ghost()
                        .small()
                        .label("Cancel")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_book_context_menu(cx);
                        })),
                )
                .with_animation(
                    "book-ctx-rise",
                    Animation::new(Duration::from_millis(210)).with_easing(ease_out_quint()),
                    |el, t| el.mb(px(-64.0 * (1.0 - t))).opacity(t),
                ),
        )
        .with_animation(
            "book-ctx-fade",
            Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
            |el, t| el.opacity(t),
        )
        .into_any_element()
}

pub fn render_library_drawer(
    app: &BookwormMobileApp,
    _window: &Window,
    cx: &mut Context<BookwormMobileApp>,
) -> impl IntoElement {
    let mut tag_counts: BTreeMap<String, usize> = BTreeMap::new();
    for book in &app.books {
        for tag in book.all_tags() {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }
    }

    let current_sort = app.library_sort;
    let active_shelf = app.library_shelf_filter.clone();
    let total_books = app.books.len();

    let section_label = |label: &'static str| {
        div()
            .px_1()
            .pt_1()
            .text_size(px(10.5))
            .font_weight(FontWeight::BOLD)
            .text_color(hex_color(kd::FG_SUBTLE))
            .child(label)
    };

    div()
        .id("lib-drawer-scrim")
        .absolute()
        .inset_0()
        .bg(hex_alpha(0x00000090))
        .occlude()
        .flex()
        .flex_row()
        .on_click(cx.listener(|this, _, window, cx| {
            this.toggle_library_drawer(false, window, cx);
        }))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .child(
            v_flex()
                .id("lib-drawer-panel")
                .w(px(284.))
                .max_w(px(300.))
                .h_full()
                .bg(hex_color(kd::BG_DEEP))
                .border_r_1()
                .border_color(hex_color(kd::BORDER))
                .shadow_xl()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    h_flex()
                        .w_full()
                        .px_3()
                        .py_2p5()
                        .items_center()
                        .justify_between()
                        .bg(hex_color(kd::BG_SIDEBAR))
                        .border_b_1()
                        .border_color(hex_color(kd::BORDER_SUBTLE))
                        .child(
                            v_flex()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(hex_color(kd::FG))
                                        .child("Bookworm"),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(hex_color(kd::FG_SUBTLE))
                                        .child(format!("{total_books} books")),
                                ),
                        )
                        .child(lib_header_icon_btn(
                            "lib-drawer-close",
                            IconName::Close,
                            cx.listener(|this, _, window, cx| {
                                this.toggle_library_drawer(false, window, cx);
                            }),
                        )),
                )
                .child(
                    div()
                        .id("lib-drawer-scroll")
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .overflow_y_scroll()
                        .track_scroll(&app.library_drawer_scroll)
                        .child(
                            v_flex()
                                .w_full()
                                .p_3()
                                .gap_3()
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(section_label("SORT BY"))
                                        .children(LibrarySort::all().iter().copied().map(
                                            |sort_opt| {
                                                let active = current_sort == sort_opt;
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "lib-drawer-sort-{}",
                                                        sort_opt.key()
                                                    )))
                                                    .w_full()
                                                    .px_2p5()
                                                    .py_2()
                                                    .rounded(px(8.))
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .bg(if active {
                                                        accent_alpha(0x28)
                                                    } else {
                                                        hex_color(kd::BG_PANEL)
                                                    })
                                                    .border_1()
                                                    .border_color(if active {
                                                        accent_alpha(0x88)
                                                    } else {
                                                        hex_color(kd::BORDER_SUBTLE)
                                                    })
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.set_library_sort(sort_opt, cx);
                                                            this.toggle_library_drawer(
                                                                false, window, cx,
                                                            );
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(if active {
                                                                FontWeight::SEMIBOLD
                                                            } else {
                                                                FontWeight::MEDIUM
                                                            })
                                                            .text_color(hex_color(if active {
                                                                kd::ACCENT_HOVER
                                                            } else {
                                                                kd::FG
                                                            }))
                                                            .child(sort_opt.label()),
                                                    )
                                                    .when(active, |row| {
                                                        row.child(lib_icon(
                                                            IconName::Check,
                                                            13.,
                                                            kd::ACCENT_HOVER,
                                                        ))
                                                    })
                                            },
                                        )),
                                )
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(section_label("SHELVES & TAGS"))
                                        .child(
                                            div()
                                                .id("lib-drawer-shelf-all")
                                                .w_full()
                                                .px_2p5()
                                                .py_2()
                                                .rounded(px(8.))
                                                .cursor_pointer()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .bg(if active_shelf.is_none() {
                                                    hex_color(kd::GREEN)
                                                } else {
                                                    hex_color(kd::BG_PANEL)
                                                })
                                                .border_1()
                                                .border_color(if active_shelf.is_none() {
                                                    hex_color(kd::GREEN)
                                                } else {
                                                    hex_color(kd::BORDER_SUBTLE)
                                                })
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.set_library_shelf_filter(None, cx);
                                                    this.toggle_library_drawer(false, window, cx);
                                                }))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(hex_color(
                                                            if active_shelf.is_none() {
                                                                kd::BG_MAIN
                                                            } else {
                                                                kd::FG
                                                            },
                                                        ))
                                                        .child("All Books"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.5))
                                                        .text_color(hex_color(
                                                            if active_shelf.is_none() {
                                                                kd::BG_MAIN
                                                            } else {
                                                                kd::FG_SUBTLE
                                                            },
                                                        ))
                                                        .child(format!("{total_books}")),
                                                ),
                                        )
                                        .when(tag_counts.is_empty(), |col| {
                                            col.child(
                                                div()
                                                    .px_2()
                                                    .py_1()
                                                    .text_size(px(11.))
                                                    .text_color(hex_color(kd::FG_SUBTLE))
                                                    .child("No tags yet — long-press any book or edit metadata."),
                                            )
                                        })
                                        .children(tag_counts.into_iter().enumerate().map(
                                            |(ix, (tag, count))| {
                                                let is_active =
                                                    active_shelf.as_deref() == Some(tag.as_str());
                                                let tag_click = tag.clone();
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "lib-drawer-tag-{ix}"
                                                    )))
                                                    .w_full()
                                                    .px_2p5()
                                                    .py_2()
                                                    .rounded(px(8.))
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .bg(if is_active {
                                                        accent_alpha(0x28)
                                                    } else {
                                                        hex_color(kd::BG_PANEL)
                                                    })
                                                    .border_1()
                                                    .border_color(if is_active {
                                                        accent_alpha(0x88)
                                                    } else {
                                                        hex_color(kd::BORDER_SUBTLE)
                                                    })
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.set_library_shelf_filter(
                                                                Some(tag_click.clone()),
                                                                cx,
                                                            );
                                                            this.toggle_library_drawer(
                                                                false, window, cx,
                                                            );
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .text_xs()
                                                            .truncate()
                                                            .text_color(hex_color(if is_active {
                                                                kd::ACCENT_HOVER
                                                            } else {
                                                                kd::FG
                                                            }))
                                                            .child(tag),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(10.5))
                                                            .text_color(hex_color(kd::FG_SUBTLE))
                                                            .child(format!("{count}")),
                                                    )
                                            },
                                        )),
                                )
                                .child(
                                    v_flex()
                                        .gap_1p5()
                                        .child(section_label("ACTIONS"))
                                        .child(
                                            Button::new("lib-drawer-pick-root")
                                                .secondary()
                                                .small()
                                                .w_full()
                                                .icon(IconName::Folder)
                                                .label("Choose Books Folder")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.toggle_library_drawer(false, window, cx);
                                                    this.open_folder_picker(window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("lib-drawer-export-md")
                                                .secondary()
                                                .small()
                                                .w_full()
                                                .icon(IconName::BookOpen)
                                                .label("Export Highlights (.md)")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.toggle_library_drawer(false, window, cx);
                                                    this.export_notes_to_markdown(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("lib-drawer-sample")
                                                .secondary()
                                                .small()
                                                .w_full()
                                                .label("Create Sample EPUB")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.toggle_library_drawer(false, window, cx);
                                                    this.create_sample_book(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("lib-drawer-settings")
                                                .secondary()
                                                .small()
                                                .w_full()
                                                .icon(IconName::Settings)
                                                .label("Settings")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.toggle_library_drawer(false, window, cx);
                                                    this.open_surface_settings(window, cx);
                                                })),
                                        ),
                                )
                                .child(div().w_full().h(px(app.safe_area.bottom))),
                        ),
                )
                .with_animation(
                    "lib-drawer-slide",
                    Animation::new(Duration::from_millis(200)).with_easing(ease_out_quint()),
                    |el, t| el.ml(px(-284.0 * (1.0 - t))).opacity(t),
                ),
        )
        .with_animation(
            "lib-drawer-scrim-fade",
            Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
            |el, t| el.opacity(t),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::compute_library_cols;
    use crate::config::LibraryGridSize;

    #[test]
    fn library_cols_adapt_from_phone_to_tablet_and_grid_size() {
        assert_eq!(
            compute_library_cols(412.0, 0.0, 0.0, LibraryGridSize::Small),
            3
        );
        assert_eq!(
            compute_library_cols(412.0, 0.0, 0.0, LibraryGridSize::Medium),
            2
        );
        assert_eq!(
            compute_library_cols(412.0, 0.0, 0.0, LibraryGridSize::Large),
            1
        );

        assert_eq!(
            compute_library_cols(680.0, 0.0, 0.0, LibraryGridSize::Small),
            4
        );
        assert_eq!(
            compute_library_cols(680.0, 0.0, 0.0, LibraryGridSize::Medium),
            3
        );
        assert_eq!(
            compute_library_cols(680.0, 0.0, 0.0, LibraryGridSize::Large),
            2
        );

        assert_eq!(
            compute_library_cols(1024.0, 0.0, 0.0, LibraryGridSize::Small),
            6
        );
        assert_eq!(
            compute_library_cols(1024.0, 0.0, 0.0, LibraryGridSize::Medium),
            4
        );
        assert_eq!(
            compute_library_cols(1024.0, 0.0, 0.0, LibraryGridSize::Large),
            3
        );
    }
}
