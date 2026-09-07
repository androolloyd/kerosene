use crate::calendar_views::IMPACT_WIDTH;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{container, text, tooltip};
use iced::{Color, Element, Theme};

pub(super) fn calendar_badge(
    label: &str,
    accent: Color,
    text_color: Color,
) -> Element<'static, Message> {
    // Keep non-economic releases compact without obscuring the original feed label.
    let full_label = label.to_string();
    let label = if label.eq_ignore_ascii_case("non-economic") {
        "OTHER".to_string()
    } else {
        label.to_uppercase()
    };
    let badge = container(
        text(label)
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .wrapping(text::Wrapping::None)
            .color(text_color),
    )
    .padding([2, 4])
    .center_x(IMPACT_WIDTH)
    .clip(true)
    .style(move |_theme: &Theme| container_style::Style {
        background: Some(accent.scale_alpha(0.10).into()),
        border: iced::Border {
            color: accent.scale_alpha(0.25),
            width: 1.0,
            radius: 9.0.into(),
        },
        ..Default::default()
    });
    tooltip(
        badge,
        container(text(full_label).size(11)).padding(6),
        tooltip::Position::Top,
    )
    .style(container::rounded_box)
    .into()
}

pub(super) fn calendar_row_style(row_bg: Color) -> container_style::Style {
    container_style::Style {
        background: Some(row_bg.into()),
        ..Default::default()
    }
}
