mod chrome;
mod layouts;

use self::chrome::calendar_badge;
use self::layouts::{
    CalendarRowLayout, view_compact_calendar_event_row, view_full_calendar_event_row,
};
use super::super::helpers::impact_color;
use crate::api;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::{Color, Element};

impl TradingTerminal {
    pub(super) fn view_calendar_event_row<'a>(
        &'a self,
        compact: bool,
        event: &'a api::CalendarEvent,
        time_str: String,
        rel_str: String,
        is_past: bool,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let event_text_color = theme
            .palette()
            .text
            .scale_alpha(if is_past { 0.6 } else { 1.0 });
        let secondary_text = theme.extended_palette().background.weak.text;
        let accent = impact_color(&event.impact, &theme);
        let row_bg = if is_past {
            Color::TRANSPARENT
        } else {
            accent.scale_alpha(0.035)
        };
        let impact = calendar_badge(
            &event.impact,
            accent.scale_alpha(if is_past { 0.6 } else { 1.0 }),
            event_text_color,
        );
        let time_color = if time_str == "TBD" || is_past {
            secondary_text.scale_alpha(0.5)
        } else {
            theme.palette().primary
        };

        let layout = CalendarRowLayout {
            event,
            time_str,
            rel_str,
            impact,
            event_text_color,
            secondary_text,
            time_color,
            row_bg,
        };

        if compact {
            view_compact_calendar_event_row(layout)
        } else {
            view_full_calendar_event_row(layout)
        }
    }
}
