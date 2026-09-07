use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, container, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_calendar_top_bar(&self) -> Element<'_, Message> {
        let refresh_btn = button(text("Refresh").size(10).center())
            .on_press_maybe((!self.calendar_loading).then_some(Message::RefreshCalendar))
            .padding([4, 7])
            .style(button::subtle);

        container(
            row![
                container(self.view_calendar_filters()).width(Fill),
                refresh_btn,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([5, 6])
        .into()
    }
}
