use crate::theme::{hex_color, kd};
use gpui_kit::{
    Div, ElementId, FontWeight, InteractiveElement as _, MouseButton, ParentElement, SharedString,
    Stateful, Styled, div, px,
};

pub const FIND_INPUT_W: f32 = 176.0;

pub fn floating_find_card(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .absolute()
        .top(px(8.))
        .right(px(18.))
        .p_1p5()
        .rounded(px(8.))
        .bg(hex_color(kd::BG_SIDEBAR))
        .border_1()
        .border_color(hex_color(kd::BORDER))
        .shadow_md()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
}

pub fn find_count_badge(label: impl Into<SharedString>, is_zero: bool) -> Div {
    div()
        .min_w(px(42.))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(if is_zero {
            hex_color(kd::RED)
        } else {
            hex_color(kd::FG_MUTED)
        })
        .child(label.into())
}
