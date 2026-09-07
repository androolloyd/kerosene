use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{button, checkbox, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Element, Fill};

impl TradingTerminal {
    pub(super) fn view_settings_network_section(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let settings = &self.hyperliquid_proxies;
        let mut content = column![
            text("Network").size(16),
            rule::horizontal(1),
            text("Hyperliquid proxies").size(14),
            checkbox(settings.enabled)
                .label("Distribute API reads across proxies")
                .on_toggle(Message::SetHyperliquidProxiesEnabled)
                .size(12).spacing(8).text_size(12),
            text("Applies to official Hyperliquid REST reads. Orders and live streams use their existing connections.")
                .size(11).color(theme.extended_palette().background.weak.text),
            row![
                text_input("Proxy URL", &settings.input)
                    .style(helpers::text_input_style)
                    .secure(true)
                    .on_input(|value| Message::HyperliquidProxyInputChanged(value.into()))
                    .on_submit(Message::AddHyperliquidProxy)
                    .size(12).padding(6).width(Fill),
                button(text("Add").size(12)).padding([6, 12])
                    .on_press_maybe((!settings.input.trim().is_empty()).then_some(Message::AddHyperliquidProxy)),
            ].spacing(8).align_y(Alignment::Center),
            text("HTTP, HTTPS, SOCKS5 or SOCKS5h. Optional username:password@host authentication.")
                .size(11).color(theme.extended_palette().background.weak.text),
            text(format!("{} proxies configured", settings.urls.len())).size(12),
        ].spacing(12).width(Fill);
        for (index, url) in settings.urls.iter().enumerate() {
            content = content.push(
                row![
                    text(url.label()).size(12).width(Fill),
                    button(text("Remove").size(12))
                        .padding([4, 8])
                        .on_press(Message::RemoveHyperliquidProxy(index)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        if settings.enabled && settings.urls.is_empty() && self.encrypted_credentials_locked() {
            content = content.push(
                button(text("Unlock saved proxies").size(12))
                    .on_press(Message::OpenUnlockCredentialsPopup),
            );
        }
        content = content.push(text("URLs are saved with your credentials in Settings > Storage. Failed or rate-limited proxies pause automatically; reads resume when a route is available.")
            .size(11).color(theme.extended_palette().background.weak.text));
        if let Some((message, is_error)) = &settings.status {
            content = content.push(text(message).size(11).color(if *is_error {
                theme.palette().danger
            } else {
                theme.extended_palette().background.weak.text
            }));
        }
        container(scrollable(content).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }
}
