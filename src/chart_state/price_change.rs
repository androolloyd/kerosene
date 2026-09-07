use super::{ChartBackfillRequestContext, ChartInstance};
use crate::api::{Candle, is_valid_candle};
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_positive_finite_number, positive_percent_change};
use crate::timeframe::Timeframe;
use std::collections::HashMap;

pub(crate) const PRICE_CHANGE_MINUTE_MS: u64 = 60_000;
pub(crate) const PRICE_CHANGE_DAY_MS: u64 = 24 * 60 * PRICE_CHANGE_MINUTE_MS;
pub(crate) const PRICE_CHANGE_HISTORY_AHEAD_MS: u64 = 60 * PRICE_CHANGE_MINUTE_MS;
pub(crate) const PRICE_CHANGE_PREFETCH_MS: u64 = 5 * PRICE_CHANGE_MINUTE_MS;

/// Historical prices are shared by symbol across chart timeframes and windows.
/// They are independent of the short-lived live asset context.
#[derive(Default)]
pub(crate) struct ChartPriceChangeHistory {
    pub(crate) symbols: HashMap<String, PriceChangeHistoryEntry>,
}

pub(crate) struct PriceChangeHistoryEntry {
    pub(crate) context: ChartBackfillRequestContext,
    pub(crate) candles: Vec<Candle>,
    pub(crate) pending: Option<PriceChangeHistoryRequest>,
    pub(crate) failures: u8,
    pub(crate) retry_at_ms: u64,
}

impl PriceChangeHistoryEntry {
    pub(crate) fn new(context: ChartBackfillRequestContext) -> Self {
        Self {
            context,
            candles: Vec::new(),
            pending: None,
            failures: 0,
            retry_at_ms: 0,
        }
    }

    pub(crate) fn record_failure(&mut self, now_ms: u64) {
        self.failures = self.failures.saturating_add(1);
        let delay = PRICE_CHANGE_MINUTE_MS
            .saturating_mul(1 << self.failures.saturating_sub(1).min(3))
            .min(5 * PRICE_CHANGE_MINUTE_MS);
        self.retry_at_ms = now_ms.saturating_add(delay);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PriceChangeHistoryRequest {
    pub(crate) symbol: String,
    pub(crate) context: ChartBackfillRequestContext,
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
}

#[derive(Clone, Copy)]
pub(crate) struct PriceChangeReference {
    pub(crate) price: f64,
    /// None denotes the exchange's own 24-hour reference. Candle references
    /// carry their actual close time so the view can disclose minute precision.
    pub(crate) close_time_ms: Option<u64>,
}

pub(crate) struct ChartPriceChange {
    pub(crate) absolute: f64,
    pub(crate) percent: f64,
    pub(crate) reference_time_ms: Option<u64>,
}

pub(crate) fn exchange_price_change_reference(
    instance: &ChartInstance,
) -> Option<PriceChangeReference> {
    let price = parse_positive_finite_number(instance.asset_ctx.as_ref()?.prev_day_px.as_deref()?)?;
    Some(PriceChangeReference {
        price,
        close_time_ms: None,
    })
}

/// Select a completed observation at or before the rolling boundary, never the
/// close of the candle containing it (that would include later trades). Limit
/// the time error to one minute, regardless of the chart's visible timeframe.
pub(crate) fn candle_price_change_reference(
    candles: &[Candle],
    now_ms: u64,
) -> Option<PriceChangeReference> {
    let cutoff_ms = now_ms.checked_sub(PRICE_CHANGE_DAY_MS)?;
    let end = candles.partition_point(|candle| candle.close_time <= cutoff_ms);
    let candle = candles.get(end.checked_sub(1)?)?;
    if !is_valid_candle(candle)
        || candle.close <= 0.0
        || candle.close_time.saturating_sub(candle.open_time) > PRICE_CHANGE_MINUTE_MS
        || cutoff_ms.saturating_sub(candle.close_time) >= PRICE_CHANGE_MINUTE_MS
    {
        return None;
    }
    Some(PriceChangeReference {
        price: candle.close,
        close_time_ms: Some(candle.close_time),
    })
}

impl TradingTerminal {
    pub(crate) fn chart_24h_change(
        &self,
        instance: &ChartInstance,
        now_ms: u64,
    ) -> Option<ChartPriceChange> {
        let current = instance.chart.candles.last()?.close;
        let reference = exchange_price_change_reference(instance)
            .or_else(|| self.shared_exchange_price_change_reference(&instance.symbol))
            .or_else(|| self.chart_candle_price_change_reference(&instance.symbol, now_ms))?;
        let percent = positive_percent_change(Some(current), Some(reference.price))?;
        Some(ChartPriceChange {
            absolute: current - reference.price,
            percent,
            reference_time_ms: reference.close_time_ms,
        })
    }

    pub(crate) fn shared_exchange_price_change_reference(
        &self,
        symbol: &str,
    ) -> Option<PriceChangeReference> {
        self.charts
            .values()
            .filter(|chart| chart.symbol == symbol)
            .filter_map(|chart| {
                exchange_price_change_reference(chart)
                    .map(|reference| (chart.asset_ctx_updated_at_ms, reference))
            })
            .max_by_key(|(updated_at, _)| *updated_at)
            .map(|(_, reference)| reference)
    }

    pub(crate) fn chart_candle_price_change_reference(
        &self,
        symbol: &str,
        now_ms: u64,
    ) -> Option<PriceChangeReference> {
        let context = self.chart_backfill_request_context_for_timeframe(Timeframe::M1);
        let fetched = self
            .chart_price_change_history
            .symbols
            .get(symbol)
            .filter(|entry| entry.context == context)
            .and_then(|entry| candle_price_change_reference(&entry.candles, now_ms));
        let loaded = self
            .charts
            .values()
            .filter(|chart| {
                chart.symbol == symbol
                    && matches!(chart.interval, Timeframe::S1 | Timeframe::M1)
                    && chart.candle_history_verified_at_ms.is_some()
            })
            .filter_map(|chart| candle_price_change_reference(&chart.chart.candles, now_ms))
            .max_by_key(|reference| reference.close_time_ms);
        loaded
            .into_iter()
            .chain(fetched)
            .max_by_key(|reference| reference.close_time_ms)
    }
}

#[cfg(test)]
mod tests;
