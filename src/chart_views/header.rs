mod actions;
mod feedback;
mod metrics;
mod symbol;

use self::feedback::format_signed_usd_change;
use self::metrics::{
    ChartHeaderMetricVisibility, push_asset_context_columns, push_outcome_asset_context_columns,
    push_outcome_volume_column,
};
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::helpers::{parse_positive_finite_number, positive_percent_change};
use crate::message::Message;
use iced::widget::{Space, column, responsive, row, text};
use iced::{Element, Fill, Length};

impl TradingTerminal {
    pub(crate) fn view_chart_header<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        surface_id: ChartSurfaceId,
    ) -> Element<'a, Message> {
        responsive(move |size| {
            self.view_chart_header_sized(chart_id, instance, surface_id, size.width)
        })
        .height(Length::Shrink)
        .into()
    }

    fn view_chart_header_sized<'a>(
        &'a self,
        chart_id: ChartId,
        instance: &'a ChartInstance,
        surface_id: ChartSurfaceId,
        available_width: f32,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        if instance.header_collapsed {
            return self.view_chart_collapsed_header(chart_id, instance, &theme);
        }

        let Some(last) = instance.chart.candles.last() else {
            return self.view_chart_placeholder_header(chart_id, instance, &theme);
        };

        let now_ms = instance.chart.clock_now_ms();
        let sym_btn = self.view_chart_symbol_button(
            chart_id,
            instance,
            last.close,
            instance.last_price_flash,
            now_ms,
            &theme,
        );

        let metric_visibility = ChartHeaderMetricVisibility::for_width(available_width);
        let mut header_row = row![sym_btn].spacing(16).align_y(iced::Alignment::Center);

        if metric_visibility.show_24h_change {
            let change_text = chart_24h_change(instance)
                .map(|(change, change_pct)| {
                    format!("{} ({change_pct:+.2}%)", format_signed_usd_change(change))
                })
                .unwrap_or_else(|| "-".to_string());
            let chg_val = text(change_text)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().text);
            let col_chg = column![
                text("24h Chg")
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
                chg_val
            ]
            .spacing(2);
            header_row = header_row.push(Space::new().width(8)).push(col_chg);
        }

        let is_outcome = self.is_outcome_coin(&instance.symbol);
        if is_outcome {
            let outcome_time_left = self
                .exchange_symbols
                .iter()
                .find(|symbol| symbol.key == instance.symbol)
                .and_then(|symbol| symbol.outcome.as_ref())
                .and_then(|info| info.time_left_label(now_ms));

            if let Some(ctx) = &instance.asset_ctx {
                header_row = push_outcome_asset_context_columns(
                    header_row,
                    &theme,
                    chart_id,
                    ctx,
                    self.outcome_volumes_24h.get(&instance.symbol).copied(),
                    outcome_time_left,
                    last.close,
                    instance.outcome_volume_as_notional,
                    instance.open_interest_as_notional,
                    metric_visibility,
                );
            } else if let Some(volume) = self.outcome_volumes_24h.get(&instance.symbol) {
                header_row = push_outcome_volume_column(
                    header_row,
                    &theme,
                    chart_id,
                    *volume,
                    outcome_time_left,
                    instance.outcome_volume_as_notional,
                    metric_visibility,
                );
            }
        }

        if !is_outcome && let Some(ctx) = &instance.asset_ctx {
            header_row = push_asset_context_columns(
                header_row,
                &theme,
                chart_id,
                ctx,
                &instance.symbol_display,
                last.close,
                instance.asset_volume_as_notional,
                instance.open_interest_as_notional,
                metric_visibility,
                now_ms,
            );
        }

        header_row = header_row
            .push(Space::new().width(Fill))
            .push(self.view_chart_screenshot_button(chart_id, surface_id));

        header_row.into()
    }
}

/// Use the displayed last price and the exchange's 24-hour reference. Candle
/// history covers an arbitrary lookback, so its first open is never a fallback
/// for a missing (or expired) previous-day price.
fn chart_24h_change(instance: &ChartInstance) -> Option<(f64, f64)> {
    let current = instance.chart.candles.last()?.close;
    let previous =
        parse_positive_finite_number(instance.asset_ctx.as_ref()?.prev_day_px.as_deref()?)?;
    let percent = positive_percent_change(Some(current), Some(previous))?;
    Some((current - previous, percent))
}

#[cfg(test)]
mod tests;
