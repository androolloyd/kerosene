use super::*;
use crate::account::AssetContext;
use crate::helpers::assert_close;
use crate::market_state::MARKET_ASSET_CONTEXT_MAX_AGE_MS;

fn chart_24h_change(instance: &ChartInstance) -> Option<(f64, f64)> {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    terminal
        .chart_24h_change(instance, instance.chart.clock_now_ms())
        .map(|change| (change.absolute, change.percent))
}

#[test]
fn change_24h_uses_exchange_reference_for_gains_losses_and_flat_prices() {
    for (current, previous, expected_change, expected_percent) in [
        (110.0, " 100.0 ", 10.0, 10.0),
        (75.0, "100", -25.0, -25.0),
        (100.0, "100", 0.0, 0.0),
        (0.00006, "0.00004", 0.00002, 50.0),
    ] {
        let mut instance = chart_with_prices(50.0, current);
        instance.set_asset_context_at(Some(context(Some(previous))), 1_000);

        let (change, percent) = chart_24h_change(&instance).expect("valid 24h change");
        assert_close(change, expected_change);
        assert_close(percent, expected_percent);
    }
}

#[test]
fn change_24h_never_substitutes_loaded_history_for_missing_or_invalid_reference() {
    let mut instance = chart_with_prices(50.0, 110.0);
    assert_eq!(chart_24h_change(&instance), None);

    for previous in [
        None,
        Some(""),
        Some("invalid"),
        Some("NaN"),
        Some("inf"),
        Some("-inf"),
        Some("0"),
        Some("-100"),
    ] {
        instance.set_asset_context_at(Some(context(previous)), 1_000);
        assert_eq!(chart_24h_change(&instance), None, "{previous:?}");
    }
}

#[test]
fn change_24h_is_independent_of_timeframe_and_history_backfill() {
    for timeframe in [
        Timeframe::Tick,
        Timeframe::S1,
        Timeframe::M1,
        Timeframe::H1,
        Timeframe::D1,
        Timeframe::W1,
        Timeframe::Mo1,
    ] {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), timeframe);
        instance.chart.candles = vec![Candle::test_flat(1_800_000_000_000, 110.0)];
        instance.set_asset_context_at(Some(context(Some("100"))), 1_000);
        assert_eq!(chart_24h_change(&instance), Some((10.0, 10.0)));

        // Scrolling back can prepend months of history at a very different price.
        instance
            .chart
            .candles
            .insert(0, Candle::test_flat(1_700_000_000_000, 5.0));
        assert_eq!(chart_24h_change(&instance), Some((10.0, 10.0)));

        instance.set_asset_context(None);
        assert_eq!(chart_24h_change(&instance), None);
    }
}

#[test]
fn change_24h_requires_a_valid_current_price_and_finite_result() {
    let mut instance = chart_with_prices(50.0, 110.0);
    instance.set_asset_context_at(Some(context(Some("1"))), 1_000);
    for current in [
        0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
    ] {
        instance.chart.candles = vec![Candle::test_flat(1_000, current)];
        assert_eq!(chart_24h_change(&instance), None, "{current}");
    }

    instance.chart.candles.clear();
    assert_eq!(chart_24h_change(&instance), None);
}

#[test]
fn change_24h_updates_with_live_candles_and_previous_day_reference() {
    let mut instance = chart_with_prices(50.0, 110.0);
    instance.set_asset_context_at(Some(context(Some("100"))), 1_000);
    assert_eq!(chart_24h_change(&instance), Some((10.0, 10.0)));

    let updated_tail = Candle::test_flat(1_800_000_000_000, 125.0);
    assert!(instance.chart.push_candle(updated_tail).applied());
    assert_eq!(chart_24h_change(&instance), Some((25.0, 25.0)));

    instance.set_asset_context_at(Some(context(Some("125"))), 2_000);
    assert_eq!(chart_24h_change(&instance), Some((0.0, 0.0)));
}

#[test]
fn change_24h_is_unavailable_after_context_expiry_and_recovers_from_rest() {
    let mut instance = chart_with_prices(50.0, 110.0);
    instance.set_asset_context_at(Some(context(Some("100"))), 1_000);
    let now_ms = 1_000 + MARKET_ASSET_CONTEXT_MAX_AGE_MS + 1;

    assert!(instance.expire_asset_context_if_stale(now_ms));
    assert_eq!(chart_24h_change(&instance), None);

    instance.fill_asset_context_from_rest(context(Some("125")), now_ms);
    assert_eq!(chart_24h_change(&instance), Some((-15.0, -12.0)));
}

fn chart_with_prices(first_open: f64, last_close: f64) -> ChartInstance {
    let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    instance.chart.candles = vec![
        Candle::test_flat(1_700_000_000_000, first_open),
        Candle::test_flat(1_800_000_000_000, last_close),
    ];
    instance
}

fn context(previous: Option<&str>) -> AssetContext {
    serde_json::from_value(serde_json::json!({ "prevDayPx": previous }))
        .expect("test asset context")
}

const NOW: u64 = 1_800_000_000_000;

#[test]
fn candle_reference_uses_clock_time_and_never_the_close_after_the_cutoff() {
    let cutoff = NOW - PRICE_CHANGE_DAY_MS;
    let candles = vec![
        Candle::test_flat(cutoff - 2 * PRICE_CHANGE_MINUTE_MS, 50.0),
        Candle::test_flat(cutoff - PRICE_CHANGE_MINUTE_MS, 100.0),
        Candle::test_flat(cutoff, 900.0),
    ];
    for now_ms in [NOW, NOW + 30_000, NOW + 59_998] {
        let reference = candle_price_change_reference(&candles, now_ms).expect("past minute close");
        assert_eq!(reference.price, 100.0);
        assert_eq!(reference.close_time_ms, Some(cutoff - 1));
    }
    let reference =
        candle_price_change_reference(&candles, NOW + 59_999).expect("next closed minute");
    assert_eq!(reference.price, 900.0);
    assert_eq!(reference.close_time_ms, Some(cutoff + 59_999));
}

#[test]
fn candle_reference_rejects_short_history_gaps_coarse_candles_and_invalid_prices() {
    let cutoff = NOW - PRICE_CHANGE_DAY_MS;
    for candles in [
        Vec::new(),
        vec![Candle::test_flat(
            NOW - 12 * 60 * PRICE_CHANGE_MINUTE_MS,
            50.0,
        )],
        vec![
            Candle::test_flat(cutoff - 2 * PRICE_CHANGE_MINUTE_MS, 50.0),
            Candle::test_flat(cutoff, 100.0),
        ],
        vec![Candle::test_ohlcv(
            cutoff - 3_600_000,
            cutoff - 1,
            [100.0; 4],
            1.0,
        )],
        vec![Candle::test_flat(cutoff - PRICE_CHANGE_MINUTE_MS, f64::NAN)],
        vec![Candle::test_flat(cutoff - PRICE_CHANGE_MINUTE_MS, 0.0)],
    ] {
        assert!(candle_price_change_reference(&candles, NOW).is_none());
    }
    assert!(candle_price_change_reference(&[], PRICE_CHANGE_DAY_MS - 1).is_none());
}

#[test]
fn verified_minute_history_supplies_other_timeframes_without_any_extra_request() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    let mut minute = ChartInstance::new(1, "BTC".to_string(), Timeframe::M1);
    let cutoff = NOW - PRICE_CHANGE_DAY_MS;
    minute.chart.candles = (cutoff - PRICE_CHANGE_MINUTE_MS..=NOW)
        .step_by(PRICE_CHANGE_MINUTE_MS as usize)
        .map(|time| Candle::test_flat(time, 100.0))
        .collect();
    terminal.charts.insert(1, minute);
    let mut daily = ChartInstance::new(2, "BTC".to_string(), Timeframe::D1);
    daily.chart.candles = vec![Candle::test_flat(NOW, 110.0)];
    terminal.charts.insert(2, daily);
    assert!(
        terminal
            .chart_24h_change(&terminal.charts[&2], NOW)
            .is_none(),
        "unverified cache is not a reference"
    );
    terminal
        .charts
        .get_mut(&1)
        .expect("minute chart")
        .candle_history_verified_at_ms = Some(NOW);
    let change = terminal
        .chart_24h_change(&terminal.charts[&2], NOW)
        .expect("shared minute history");
    assert_eq!(change.percent, 10.0);
    assert_eq!(change.reference_time_ms, Some(cutoff - 1));
    assert!(terminal.queue_chart_price_change_history(NOW).is_empty());

    terminal
        .charts
        .get_mut(&1)
        .expect("minute chart")
        .set_symbol_identity("ETH".to_string(), "ETH".to_string());
    assert!(
        terminal
            .chart_24h_change(&terminal.charts[&2], NOW)
            .is_none(),
        "different symbol cannot supply the reference"
    );
}

#[test]
fn exchange_reference_takes_priority_over_candle_fallback_and_is_shared() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.charts.clear();
    let mut first = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
    first.set_asset_context_at(Some(context(Some("100"))), NOW);
    terminal.charts.insert(1, first);
    let mut second = ChartInstance::new(2, "BTC".to_string(), Timeframe::D1);
    second.chart.candles = vec![Candle::test_flat(NOW, 110.0)];
    terminal.charts.insert(2, second);
    let source_context = terminal.chart_backfill_request_context_for_timeframe(Timeframe::M1);
    let mut entry = PriceChangeHistoryEntry::new(source_context);
    entry.candles = vec![Candle::test_flat(
        NOW - PRICE_CHANGE_DAY_MS - PRICE_CHANGE_MINUTE_MS,
        50.0,
    )];
    terminal
        .chart_price_change_history
        .symbols
        .insert("BTC".to_string(), entry);
    let change = terminal
        .chart_24h_change(&terminal.charts[&2], NOW)
        .expect("shared exchange reference");
    assert_eq!(change.percent, 10.0);
    assert_eq!(change.reference_time_ms, None);
    assert!(terminal.queue_chart_price_change_history(NOW).is_empty());

    terminal
        .charts
        .get_mut(&1)
        .expect("first chart")
        .set_asset_context(None);
    let change = terminal
        .chart_24h_change(&terminal.charts[&2], NOW)
        .expect("candle fallback survives metadata loss");
    assert_eq!(change.percent, 120.0);
    assert_eq!(
        change.reference_time_ms,
        Some(NOW - PRICE_CHANGE_DAY_MS - 1)
    );
}
