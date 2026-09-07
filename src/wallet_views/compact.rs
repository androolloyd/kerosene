use super::numbers::{parse_wallet_number, wallet_has_visible_nonzero};
use super::position_metrics::{wallet_position_upnl, wallet_position_value};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::wallet_state::compact::{
    CompactWalletSelection, CompactWalletTrackerId, WalletPositionBias, wallet_position_bias,
};
use iced::widget::{
    Column, Row, button, column, container, responsive, row, rule, scrollable, text, tooltip,
};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_compact_wallet_tracker(
        &self,
        id: CompactWalletTrackerId,
    ) -> Element<'_, Message> {
        responsive(move |size| {
            let width = (size.width - 16.0).max(320.0);
            let content = match self.wallet_tracker.compact_selections.get(&id) {
                Some(selection)
                    if self
                        .wallet_tracker
                        .tracked_addresses
                        .iter()
                        .any(|address| address == selection.address.as_str()) =>
                {
                    self.view_compact_wallet_positions(id, selection, width)
                }
                _ => self.view_compact_wallet_list(id),
            };
            container(
                scrollable(container(content).width(width).height(Fill))
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::new(),
                    ))
                    .height(Fill),
            )
            .padding(8)
            .width(Fill)
            .height(Fill)
            .into()
        })
        .into()
    }

    fn view_compact_wallet_list(&self, id: CompactWalletTrackerId) -> Element<'_, Message> {
        let theme = self.theme();
        let palette = theme.palette();
        let muted = theme.extended_palette().background.weak.text;
        let denomination = self.display_denomination_context();
        let header = table_header(
            &[("Wallet", 3), ("Value", 3), ("uPnL", 3), ("Bias", 2)],
            muted,
        );
        let mut rows = Column::new().spacing(2).width(Fill);
        for address in &self.wallet_tracker.tracked_addresses {
            let state = self.wallet_tracker.rows.get(address);
            let snapshot = state.and_then(|state| state.snapshot.as_ref());
            let stale = state.is_some_and(|state| {
                state.error.is_some()
                    || state.last_updated_ms.is_some_and(|updated| {
                        self.status_bar_now_ms.saturating_sub(updated) > 120_000
                    })
                    || state
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.valuation_warning.is_some())
            });
            let label = self.wallet_display(address).primary;
            let (bias, bias_color) = bias_display(
                wallet_position_bias(
                    snapshot.and_then(|snapshot| snapshot.long_exposure),
                    snapshot.and_then(|snapshot| snapshot.short_exposure),
                ),
                &theme,
            );
            let upnl = snapshot
                .and_then(|snapshot| snapshot.unrealized_pnl)
                .filter(|value| value.is_finite());
            let equity = snapshot
                .and_then(|snapshot| snapshot.equity)
                .filter(|value| value.is_finite());
            let empty = if state.is_some_and(|state| state.loading) {
                "…"
            } else {
                "—"
            };
            let content = row![
                cell(
                    if stale {
                        format!("{label} ·")
                    } else {
                        label.clone()
                    },
                    3,
                    false,
                    if stale { palette.warning } else { palette.text }
                ),
                cell(
                    equity
                        .map(|value| compact_money(&denomination, value, false))
                        .unwrap_or_else(|| empty.to_string()),
                    3,
                    true,
                    palette.text
                ),
                cell(
                    upnl.map(|value| compact_money(&denomination, value, true))
                        .unwrap_or_else(|| empty.to_string()),
                    3,
                    true,
                    signed_color(upnl, &theme)
                ),
                cell(bias, 2, true, bias_color),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            let status = if stale {
                " · Stale or partial snapshot"
            } else if state.is_some_and(|state| state.loading) {
                " · Refreshing"
            } else if snapshot.is_none() {
                " · Waiting for snapshot"
            } else {
                ""
            };
            rows = rows.push(tooltip(
                button(content)
                    .on_press(Message::CompactWalletSelected(id, address.clone().into()))
                    .padding([6, 8])
                    .width(Fill)
                    .style(compact_row_style),
                text(format!("{label}{status}")).size(11),
                tooltip::Position::Top,
            ));
        }
        if self.wallet_tracker.tracked_addresses.is_empty() {
            rows = rows
                .push(container(text("No tracked wallets").size(12).color(muted)).padding([8, 8]));
        }
        column![header, rule::horizontal(1), scrollable(rows).height(Fill)]
            .spacing(4)
            .height(Fill)
            .into()
    }

    fn view_compact_wallet_positions<'a>(
        &'a self,
        id: CompactWalletTrackerId,
        selection: &'a CompactWalletSelection,
        width: f32,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let palette = theme.palette();
        let muted = theme.extended_palette().background.weak.text;
        let denomination = self.display_denomination_context();
        let label = self.wallet_display(selection.address.as_str()).primary;
        let loading = selection.pending_request.is_some();
        let controls = row![
            button(text("‹ Back").size(11))
                .padding([4, 6])
                .style(button::text)
                .on_press(Message::CompactWalletBack(id)),
            cell(label, 1, false, palette.text),
            button(text(if loading { "…" } else { "Refresh" }).size(11))
                .padding([4, 6])
                .style(button::text)
                .on_press_maybe((!loading).then_some(Message::CompactWalletRefresh(id))),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        let mut content = column![controls, rule::horizontal(1)]
            .spacing(4)
            .height(Fill);
        if let Some(error) = selection.error {
            content = content.push(text(error).size(11).color(palette.warning));
        }
        let Some(data) = &selection.data else {
            return content
                .push(
                    container(
                        text(if loading {
                            "Loading positions…"
                        } else {
                            "Positions unavailable"
                        })
                        .size(12)
                        .color(muted),
                    )
                    .center_x(Fill)
                    .center_y(Fill),
                )
                .into();
        };
        if !data.warnings.is_empty() {
            content = content.push(text("Partial snapshot").size(11).color(palette.warning));
        }
        let show_entry = width >= 560.0;
        let mut columns = vec![("Position", 3), ("Size", 2)];
        if show_entry {
            columns.push(("Entry", 2));
        }
        columns.extend([("Value", 3), ("uPnL", 3)]);
        content = content
            .push(table_header(&columns, muted))
            .push(rule::horizontal(1));
        let mut positions: Vec<_> = data
            .positions
            .iter()
            .filter(|detail| {
                let position = &detail.asset_position.position;
                wallet_has_visible_nonzero(&position.szi)
                    && self
                        .visible_wallet_detail_symbol(&detail.dex, &position.coin)
                        .is_some()
            })
            .collect();
        positions.sort_by(|a, b| {
            let value = |detail: &crate::account::WalletPositionDetail| {
                parse_wallet_number(&detail.asset_position.position.position_value)
                    .unwrap_or(0.0)
                    .abs()
            };
            value(b).total_cmp(&value(a)).then_with(|| {
                Self::wallet_detail_symbol(&a.dex, &a.asset_position.position.coin).cmp(
                    &Self::wallet_detail_symbol(&b.dex, &b.asset_position.position.coin),
                )
            })
        });
        let no_positions = positions.is_empty();
        let mut rows = Column::new().spacing(2).width(Fill);
        for detail in positions {
            let pos = &detail.asset_position.position;
            let size = parse_wallet_number(&pos.szi);
            let entry = parse_wallet_number(&pos.entry_px);
            let value = wallet_position_value(size, &pos.position_value, None);
            let upnl = wallet_position_upnl(size, entry, &pos.unrealized_pnl, None);
            let symbol = Self::wallet_detail_symbol(&detail.dex, &pos.coin);
            let (side, color) = match size {
                Some(size) if size > 0.0 => ("Long", palette.success),
                Some(_) => ("Short", palette.danger),
                None => ("—", palette.warning),
            };
            let identity = column![
                text(symbol).size(12).wrapping(text::Wrapping::None),
                text(format!("{side} · {}x", pos.leverage.value))
                    .size(10)
                    .color(color),
            ]
            .spacing(2);
            let mut cells = row![
                container(identity).width(Length::FillPortion(3)).clip(true),
                cell(
                    size.map(|size| crate::helpers::format_size(size.abs()))
                        .unwrap_or_else(|| "—".to_string()),
                    2,
                    true,
                    palette.text
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if show_entry {
                cells = cells.push(cell(
                    entry
                        .map(|entry| denomination.format_price(entry))
                        .unwrap_or_else(|| "—".to_string()),
                    2,
                    true,
                    muted,
                ));
            }
            cells = cells
                .push(cell(
                    value
                        .map(|value| compact_money(&denomination, value, false))
                        .unwrap_or_else(|| "—".to_string()),
                    3,
                    true,
                    palette.text,
                ))
                .push(cell(
                    upnl.map(|value| compact_money(&denomination, value, true))
                        .unwrap_or_else(|| "—".to_string()),
                    3,
                    true,
                    signed_color(upnl, &theme),
                ));
            rows = rows.push(container(cells).padding([6, 8]).width(Fill));
        }
        if no_positions {
            rows = rows
                .push(container(text("No open positions").size(12).color(muted)).padding([8, 8]));
        }
        content.push(scrollable(rows).height(Fill)).into()
    }
}

fn compact_money(
    denomination: &crate::denomination::DisplayDenominationContext,
    value: f64,
    signed: bool,
) -> String {
    if denomination
        .convert_usd_value(value)
        .is_some_and(|value| value.abs() >= 10_000.0)
    {
        let compact = denomination.format_signed_compact_value(value);
        if signed {
            compact
        } else {
            compact.trim_start_matches('+').to_string()
        }
    } else if signed {
        denomination.format_signed_value(value, 2)
    } else {
        denomination.format_value(value, 2)
    }
}

fn cell(
    value: impl Into<String>,
    portion: u16,
    right: bool,
    color: Color,
) -> Element<'static, Message> {
    container(
        text(value.into())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(color)
            .wrapping(text::Wrapping::None)
            .width(Fill)
            .align_x(if right {
                Alignment::End
            } else {
                Alignment::Start
            }),
    )
    .width(Length::FillPortion(portion))
    .clip(true)
    .into()
}

fn table_header(columns: &[(&str, u16)], color: Color) -> Element<'static, Message> {
    let cells = columns
        .iter()
        .enumerate()
        .fold(Row::new().spacing(8), |row, (index, (label, portion))| {
            row.push(cell(*label, *portion, index > 0, color))
        });
    container(cells).padding([4, 8]).width(Fill).into()
}

fn signed_color(value: Option<f64>, theme: &Theme) -> Color {
    match value {
        Some(value) if value > 0.0 => theme.palette().success,
        Some(value) if value < 0.0 => theme.palette().danger,
        _ => theme.extended_palette().background.weak.text,
    }
}

fn bias_display(bias: WalletPositionBias, theme: &Theme) -> (&'static str, Color) {
    match bias {
        WalletPositionBias::Long => ("Long", theme.palette().success),
        WalletPositionBias::Short => ("Short", theme.palette().danger),
        WalletPositionBias::Balanced => ("Mixed", theme.extended_palette().background.weak.text),
        WalletPositionBias::Flat => ("Flat", theme.extended_palette().background.weak.text),
        WalletPositionBias::Unavailable => ("—", theme.extended_palette().background.weak.text),
    }
}

fn compact_row_style(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            match status {
                button::Status::Hovered | button::Status::Pressed => {
                    theme.extended_palette().background.strong.color
                }
                _ => theme.extended_palette().background.base.color,
            }
            .into(),
        ),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
