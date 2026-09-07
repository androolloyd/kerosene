use crate::api::{self, Candle};
use crate::app_state::TradingTerminal;
use crate::chart_state::{
    PRICE_CHANGE_DAY_MS, PRICE_CHANGE_HISTORY_AHEAD_MS, PRICE_CHANGE_MINUTE_MS,
    PRICE_CHANGE_PREFETCH_MS, PriceChangeHistoryEntry, PriceChangeHistoryRequest,
    candle_price_change_reference,
};
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::Task;
use std::collections::BTreeSet;

impl TradingTerminal {
    /// Warm a small slice of yesterday's minute candles only where live
    /// metadata is missing. One slice serves every chart for the symbol and
    /// advances locally for an hour without a request on each clock tick.
    pub(crate) fn queue_chart_price_change_history(&mut self, now_ms: u64) -> Vec<Task<Message>> {
        let symbols: BTreeSet<_> = self
            .charts
            .values()
            .filter(|chart| !chart.symbol.is_empty() && !self.symbol_key_is_hidden(&chart.symbol))
            .map(|chart| chart.symbol.clone())
            .collect();
        self.chart_price_change_history
            .symbols
            .retain(|symbol, _| symbols.contains(symbol));
        let mut tasks = Vec::new();
        for symbol in symbols {
            let context =
                self.chart_backfill_request_context_for_symbol_timeframe(&symbol, Timeframe::M1);
            let entry = self
                .chart_price_change_history
                .symbols
                .entry(symbol.clone())
                .or_insert_with(|| PriceChangeHistoryEntry::new(context));
            if entry.context != context {
                *entry = PriceChangeHistoryEntry::new(context);
            }
            if entry.pending.is_some() || now_ms < entry.retry_at_ms {
                continue;
            }
            if self
                .shared_exchange_price_change_reference(&symbol)
                .is_some()
                || (self
                    .chart_candle_price_change_reference(&symbol, now_ms)
                    .is_some()
                    && self
                        .chart_candle_price_change_reference(
                            &symbol,
                            now_ms.saturating_add(PRICE_CHANGE_PREFETCH_MS),
                        )
                        .is_some())
            {
                continue;
            }
            let Some(cutoff_ms) = now_ms.checked_sub(PRICE_CHANGE_DAY_MS) else {
                continue;
            };
            let minute_ms = cutoff_ms / PRICE_CHANGE_MINUTE_MS * PRICE_CHANGE_MINUTE_MS;
            let request = PriceChangeHistoryRequest {
                symbol: symbol.clone(),
                context,
                start_ms: minute_ms.saturating_sub(PRICE_CHANGE_MINUTE_MS),
                end_ms: minute_ms.saturating_add(PRICE_CHANGE_HISTORY_AHEAD_MS),
            };
            if let Some(entry) = self.chart_price_change_history.symbols.get_mut(&symbol) {
                entry.pending = Some(request.clone());
            }
            let fetch_request = api::ChartCandleFetchRequest {
                source: context.source,
                hydromancer_api_key: self.hydromancer_api_key_for_task(),
                schwab_access_token: self.schwab.access_token_for_task(),
                coin: symbol,
                interval: Timeframe::M1.api_str().to_string(),
                start_time: request.start_ms,
                end_time: request.end_ms,
                policy: api::CandleFetchPolicy::NetworkOnly,
            };
            tasks.push(Task::perform(
                api::fetch_chart_backfill_candles(fetch_request),
                move |result| Message::ChartPriceChangeHistoryLoaded(request.clone(), result),
            ));
        }
        tasks
    }

    pub(crate) fn apply_chart_price_change_history(
        &mut self,
        request: PriceChangeHistoryRequest,
        result: Result<Vec<Candle>, String>,
        now_ms: u64,
    ) {
        let current_context = self
            .chart_backfill_request_context_for_symbol_timeframe(&request.symbol, Timeframe::M1);
        let hidden = self.symbol_key_is_hidden(&request.symbol);
        let Some(entry) = self
            .chart_price_change_history
            .symbols
            .get_mut(&request.symbol)
        else {
            return;
        };
        if entry.pending.as_ref() != Some(&request) {
            return;
        }
        entry.pending = None;
        if request.context != current_context || hidden {
            return;
        }
        if let Ok(candles) = result {
            let candles = api::normalize_candles(candles)
                .into_iter()
                .filter(|candle| {
                    candle.open_time >= request.start_ms
                        && candle.close_time <= request.end_ms
                        && candle.close_time.saturating_sub(candle.open_time)
                            <= PRICE_CHANGE_MINUTE_MS
                })
                .collect::<Vec<_>>();
            if candle_price_change_reference(&candles, now_ms).is_some() {
                entry.candles = candles;
                entry.failures = 0;
                entry.retry_at_ms = 0;
                return;
            }
        }
        // Preserve useful earlier history while retrying failed or empty reads.
        entry.record_failure(now_ms);
    }
}

#[cfg(test)]
mod tests;
