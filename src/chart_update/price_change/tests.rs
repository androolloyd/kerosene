use super::*;
use crate::chart_state::ChartInstance;

const NOW: u64 = 1_800_000_000_000;

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    for (id, timeframe) in [(1, Timeframe::M1), (2, Timeframe::H1), (3, Timeframe::D1)] {
        let mut chart = ChartInstance::new(id, "BTC".to_string(), timeframe);
        chart.chart.candles = vec![Candle::test_flat(NOW, 110.0)];
        terminal.charts.insert(id, chart);
    }
    terminal
}

fn request(terminal: &TradingTerminal) -> PriceChangeHistoryRequest {
    terminal.chart_price_change_history.symbols["BTC"]
        .pending
        .clone()
        .expect("pending request")
}

fn history(request: &PriceChangeHistoryRequest, price: f64) -> Vec<Candle> {
    (request.start_ms..request.end_ms)
        .step_by(PRICE_CHANGE_MINUTE_MS as usize)
        .map(|time| Candle::test_flat(time, price))
        .collect()
}

#[test]
fn history_request_is_shared_across_timeframes_and_covers_an_hour_of_rolling_references() {
    let mut terminal = terminal();
    assert_eq!(terminal.queue_chart_price_change_history(NOW).len(), 1);
    let request = request(&terminal);
    assert_eq!(
        request.start_ms,
        NOW - PRICE_CHANGE_DAY_MS - PRICE_CHANGE_MINUTE_MS
    );
    assert_eq!(
        request.end_ms,
        NOW - PRICE_CHANGE_DAY_MS + PRICE_CHANGE_HISTORY_AHEAD_MS
    );
    assert!(
        terminal
            .queue_chart_price_change_history(NOW + 1_000)
            .is_empty()
    );

    terminal.apply_chart_price_change_history(request.clone(), Ok(history(&request, 100.0)), NOW);
    for chart in terminal.charts.values() {
        let change = terminal
            .chart_24h_change(chart, NOW)
            .expect("derived 24h change");
        assert_eq!(change.absolute, 10.0);
        assert_eq!(change.percent, 10.0);
        assert_eq!(
            change.reference_time_ms,
            Some(NOW - PRICE_CHANGE_DAY_MS - 1)
        );
    }
    assert!(
        terminal
            .queue_chart_price_change_history(NOW + 30 * PRICE_CHANGE_MINUTE_MS)
            .is_empty()
    );
    assert_eq!(
        terminal
            .queue_chart_price_change_history(NOW + 56 * PRICE_CHANGE_MINUTE_MS)
            .len(),
        1
    );
    assert!(
        terminal
            .chart_24h_change(&terminal.charts[&1], NOW + 56 * PRICE_CHANGE_MINUTE_MS)
            .is_some()
    );
}

#[test]
fn history_failure_backs_off_and_preserves_a_usable_reference() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let first_request = request(&terminal);
    terminal.apply_chart_price_change_history(
        first_request.clone(),
        Ok(history(&first_request, 100.0)),
        NOW,
    );
    let refresh_ms = NOW + 56 * PRICE_CHANGE_MINUTE_MS;
    let _tasks = terminal.queue_chart_price_change_history(refresh_ms);
    terminal.apply_chart_price_change_history(
        request(&terminal),
        Err("HTTP 429".to_string()),
        refresh_ms,
    );
    assert!(
        terminal
            .chart_24h_change(&terminal.charts[&1], refresh_ms)
            .is_some()
    );
    assert!(
        terminal
            .queue_chart_price_change_history(refresh_ms + 59_999)
            .is_empty()
    );
    assert_eq!(
        terminal
            .queue_chart_price_change_history(refresh_ms + 60_000)
            .len(),
        1
    );
}

#[test]
fn empty_history_does_not_invent_a_reference_or_retry_every_tick() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    terminal.apply_chart_price_change_history(request(&terminal), Ok(Vec::new()), NOW);
    assert!(
        terminal
            .chart_24h_change(&terminal.charts[&1], NOW)
            .is_none()
    );
    assert!(
        terminal
            .queue_chart_price_change_history(NOW + 1_000)
            .is_empty()
    );
}

#[test]
fn previous_provider_results_cannot_fill_the_current_history() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let stale = request(&terminal);
    terminal.read_data_provider_generation += 1;
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW)
            .is_none()
    );
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let current = request(&terminal);
    terminal.apply_chart_price_change_history(stale.clone(), Ok(history(&stale, 1.0)), NOW);
    assert_eq!(request(&terminal), current);
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW)
            .is_none()
    );
    terminal.apply_chart_price_change_history(current.clone(), Ok(history(&current, 100.0)), NOW);
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW)
            .is_some()
    );
}

#[test]
fn closing_all_symbol_charts_removes_history_and_ignores_late_results() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let stale = request(&terminal);
    terminal.charts.clear();
    assert!(terminal.queue_chart_price_change_history(NOW).is_empty());
    terminal.apply_chart_price_change_history(stale.clone(), Ok(history(&stale, 100.0)), NOW);
    assert!(terminal.chart_price_change_history.symbols.is_empty());
}

#[test]
fn only_requested_minute_history_can_supply_the_reference() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let pending = request(&terminal);
    let cutoff = NOW - PRICE_CHANGE_DAY_MS;
    let candles = vec![
        Candle::test_ohlcv(cutoff - 3_600_000, cutoff - 1, [10.0; 4], 1.0),
        Candle::test_flat(cutoff, 100.0),
        Candle::test_flat(pending.end_ms + PRICE_CHANGE_MINUTE_MS, 100.0),
    ];
    terminal.apply_chart_price_change_history(pending, Ok(candles), NOW);
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW)
            .is_none()
    );
    assert_eq!(
        terminal.chart_price_change_history.symbols["BTC"].failures,
        1
    );
}

#[test]
fn history_is_not_reused_after_provider_key_rotation_or_a_clock_jump() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let pending = request(&terminal);
    terminal.apply_chart_price_change_history(pending.clone(), Ok(history(&pending, 100.0)), NOW);
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW)
            .is_some()
    );
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW + 2 * PRICE_CHANGE_HISTORY_AHEAD_MS)
            .is_none()
    );

    terminal.hydromancer_key_generation += 1;
    assert!(
        terminal
            .chart_candle_price_change_reference("BTC", NOW)
            .is_none()
    );
    assert_eq!(terminal.queue_chart_price_change_history(NOW).len(), 1);
    assert!(
        terminal.chart_price_change_history.symbols["BTC"]
            .candles
            .is_empty()
    );
}

#[test]
fn hiding_a_symbol_during_completion_does_not_leave_its_request_stuck() {
    let mut terminal = terminal();
    let _tasks = terminal.queue_chart_price_change_history(NOW);
    let pending = request(&terminal);
    terminal.market_universe = crate::config::MarketUniverseConfig::hip3_dex("xyz");
    terminal.apply_chart_price_change_history(pending.clone(), Ok(history(&pending, 100.0)), NOW);
    let entry = &terminal.chart_price_change_history.symbols["BTC"];
    assert!(entry.pending.is_none());
    assert!(entry.candles.is_empty());
    terminal.market_universe = crate::config::MarketUniverseConfig::All;
    assert_eq!(terminal.queue_chart_price_change_history(NOW).len(), 1);
}

/// Opt-in verification of the real provider request, parsing, message handler,
/// and chart calculation, with no account or provider credentials.
#[tokio::test]
#[ignore = "requires the public Hyperliquid API"]
async fn live_chart_24h_candle_fallback() {
    for symbol in ["BTC", "ETH", "HYPE"] {
        let context = api::fetch_chart_asset_context(symbol.to_string())
            .await
            .expect("public asset context request")
            .expect("listed asset context");
        let price = context
            .mark_px
            .as_deref()
            .or(context.mid_px.as_deref())
            .and_then(crate::helpers::parse_positive_finite_number)
            .expect("current public price");
        let now_ms = TradingTerminal::now_ms();
        let mut terminal = terminal();
        for chart in terminal.charts.values_mut() {
            chart.set_symbol_identity(symbol.to_string(), symbol.to_string());
            chart.chart.candles = vec![Candle::test_flat(now_ms, price)];
        }
        let tasks = terminal.queue_chart_price_change_history(now_ms);
        assert_eq!(tasks.len(), 1);
        let pending = terminal.chart_price_change_history.symbols[symbol]
            .pending
            .clone()
            .expect("pending public candle request");
        let candles = api::fetch_chart_backfill_candles(api::ChartCandleFetchRequest {
            source: pending.context.source,
            hydromancer_api_key: Default::default(),
            coin: symbol.to_string(),
            interval: Timeframe::M1.api_str().to_string(),
            start_time: pending.start_ms,
            end_time: pending.end_ms,
            policy: api::CandleFetchPolicy::NetworkOnly,
        })
        .await
        .expect("public minute candle history");
        let _task =
            terminal.update_chart(Message::ChartPriceChangeHistoryLoaded(pending, Ok(candles)));
        let now_ms = TradingTerminal::now_ms();
        for chart in terminal.charts.values() {
            let change = terminal
                .chart_24h_change(chart, now_ms)
                .expect("derived live 24h change");
            let reference_ms = change.reference_time_ms.expect("candle-derived reference");
            let cutoff_ms = now_ms - PRICE_CHANGE_DAY_MS;
            assert!(reference_ms <= cutoff_ms && cutoff_ms - reference_ms < PRICE_CHANGE_MINUTE_MS);
            assert!(change.percent.is_finite());
        }
    }
}
