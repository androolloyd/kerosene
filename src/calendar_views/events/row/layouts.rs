use super::chrome::calendar_row_style;
use crate::api;
use crate::calendar_views::{COLUMN_SPACING, MARKET_WIDTH, ROW_HEIGHT, TIME_WIDTH, VALUE_WIDTH};
use crate::message::Message;
use iced::widget::{column, container, row, text, tooltip};
use iced::{Color, Element, Fill, Theme};

pub(super) struct CalendarRowLayout<'a> {
    pub(super) event: &'a api::CalendarEvent,
    pub(super) time_str: String,
    pub(super) rel_str: String,
    pub(super) impact: Element<'static, Message>,
    pub(super) event_text_color: Color,
    pub(super) secondary_text: Color,
    pub(super) time_color: Color,
    pub(super) row_bg: Color,
}

pub(super) fn view_compact_calendar_event_row<'a>(
    layout: CalendarRowLayout<'a>,
) -> Element<'a, Message> {
    let mut content = column![
        row![
            view_time(
                layout.time_str,
                layout.rel_str,
                layout.time_color,
                layout.secondary_text
            ),
            text(&layout.event.country)
                .font(crate::app_fonts::monospace_font())
                .size(11)
                .color(layout.event_text_color),
            layout.impact,
        ]
        .spacing(8)
        .align_y(iced::Center)
        .wrap(),
        text(&layout.event.title)
            .size(12)
            .color(layout.event_text_color),
    ]
    .spacing(3);

    // Do not allocate an empty details line for speeches, holidays, or other releases without values.
    if !layout.event.forecast.is_empty() || !layout.event.previous.is_empty() {
        content = content.push(
            row![
                text(format!(
                    "Forecast {}",
                    display_value(&layout.event.forecast)
                ))
                .size(10)
                .color(layout.secondary_text),
                text(format!("Prev {}", display_value(&layout.event.previous)))
                    .size(10)
                    .color(layout.secondary_text),
            ]
            .spacing(14)
            .wrap(),
        );
    }

    container(content)
        .width(Fill)
        .padding([5, 8])
        .style(move |_theme: &Theme| calendar_row_style(layout.row_bg))
        .into()
}

pub(super) fn view_full_calendar_event_row<'a>(
    layout: CalendarRowLayout<'a>,
) -> Element<'a, Message> {
    let title = tooltip(
        container(
            text(&layout.event.title)
                .size(12)
                .color(layout.event_text_color)
                .wrapping(text::Wrapping::None),
        )
        .width(Fill)
        .height(16)
        .clip(true),
        container(text(&layout.event.title).size(12))
            .padding(6)
            .max_width(420),
        tooltip::Position::Top,
    )
    .style(container::rounded_box);

    container(
        row![
            container(view_time(
                layout.time_str,
                layout.rel_str,
                layout.time_color,
                layout.secondary_text
            ))
            .width(TIME_WIDTH),
            text(&layout.event.country)
                .font(crate::app_fonts::monospace_font())
                .size(11)
                .color(layout.event_text_color)
                .width(MARKET_WIDTH)
                .center(),
            layout.impact,
            title,
            view_value(&layout.event.forecast, layout.event_text_color),
            view_value(&layout.event.previous, layout.event_text_color),
        ]
        .spacing(COLUMN_SPACING)
        .align_y(iced::Center),
    )
    .padding([0, 8])
    .center_y(ROW_HEIGHT)
    .width(Fill)
    .style(move |_theme: &Theme| calendar_row_style(layout.row_bg))
    .into()
}

fn view_time(
    time: String,
    relative: String,
    dot_color: Color,
    text_color: Color,
) -> Element<'static, Message> {
    tooltip(
        row![
            container(iced::widget::Space::new())
                .width(5)
                .height(5)
                .style(move |_: &Theme| container::Style {
                    background: Some(dot_color.into()),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            text(time)
                .font(crate::app_fonts::monospace_font())
                .size(11)
                .color(text_color),
        ]
        .spacing(5)
        .align_y(iced::Center),
        container(text(relative).size(11)).padding(6),
        tooltip::Position::Top,
    )
    .style(container::rounded_box)
    .into()
}

fn display_value(value: &str) -> &str {
    if value.is_empty() { "—" } else { value }
}

fn view_value(value: &str, color: Color) -> Element<'_, Message> {
    tooltip(
        container(
            text(display_value(value))
                .font(crate::app_fonts::monospace_font())
                .size(11)
                .color(color)
                .align_x(iced::Right)
                .width(Fill)
                .wrapping(text::Wrapping::None),
        )
        .width(VALUE_WIDTH)
        .height(15)
        .clip(true),
        container(text(display_value(value)).size(11)).padding(6),
        tooltip::Position::Top,
    )
    .style(container::rounded_box)
    .into()
}
