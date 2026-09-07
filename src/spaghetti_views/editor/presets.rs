use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};

use iced::widget::{button, pick_list, row, text};
use iced::{Alignment, Element, Fill, Length};

impl TradingTerminal {
    pub(super) fn view_spaghetti_watchlist_preset_controls(
        &self,
        id: SpaghettiChartId,
        instance: &SpaghettiChartInstance,
    ) -> Element<'_, Message> {
        let selected = instance
            .watchlist_preset_id
            .and_then(|preset_id| self.watchlist_preset(preset_id).cloned());
        let picker = pick_list(self.watchlist_presets.clone(), selected, move |preset| {
            Message::SpaghettiWatchlistPresetSelected(id, preset.id)
        })
        .placeholder("Populate from watchlist...")
        .padding([4, 8])
        .text_size(11)
        .width(Length::Fixed(210.0));

        let mut controls = row![text("Watchlist").size(11), picker]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Fill);
        if instance.watchlist_preset_id.is_some() {
            controls = controls.push(
                button(text("Unlink").size(10))
                    .on_press(Message::SpaghettiClearWatchlistPreset(id))
                    .padding([3, 7]),
            );
        }
        controls.into()
    }
}
