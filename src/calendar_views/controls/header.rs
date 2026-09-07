use super::super::{
    COLUMN_SPACING, DATE_WIDTH, IMPACT_WIDTH, MARKET_WIDTH, SCROLLBAR_WIDTH, TIME_WIDTH,
    VALUE_WIDTH,
};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{container, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_calendar_table_header(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let label = |value| {
            text(value)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
        };

        container(row![
            container(label("DATE")).padding([0, 8]).width(DATE_WIDTH),
            container(
                row![
                    label("TIME").width(TIME_WIDTH),
                    label("CCY").width(MARKET_WIDTH).center(),
                    label("IMPACT").width(IMPACT_WIDTH).center(),
                    label("EVENT").width(Fill),
                    label("FORECAST").width(VALUE_WIDTH).align_x(iced::Right),
                    label("PREV").width(VALUE_WIDTH).align_x(iced::Right),
                ]
                .spacing(COLUMN_SPACING),
            )
            // Account for the date divider and reserved scrollbar gutter as well as row padding.
            .padding(iced::Padding {
                left: 9.0,
                right: 8.0 + SCROLLBAR_WIDTH,
                ..Default::default()
            })
            .width(Fill),
        ])
        .padding([6, 0])
        .style(|theme: &iced::Theme| container::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(0.5)
                    .into(),
            ),
            ..Default::default()
        })
        .into()
    }
}
