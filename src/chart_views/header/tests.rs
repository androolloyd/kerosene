use super::{
    chart_24h_change,
    feedback::{chart_header_changed_text, format_signed_usd_change},
};
use crate::account::AssetContext;
use crate::api::Candle;
use crate::chart_state::ChartInstance;
use crate::helpers::assert_close;
use crate::market_state::MARKET_ASSET_CONTEXT_MAX_AGE_MS;
use crate::timeframe::Timeframe;

#[test]
fn changed_text_highlights_only_changed_decimal_digit() {
    let parts = chart_header_changed_text("82,543.2", "82,543.3").expect("changed text");

    assert_eq!(parts.before, "82,543.");
    assert_eq!(parts.changed, "3");
    assert_eq!(parts.after, "");
}

#[test]
fn changed_text_keeps_shared_suffix_when_middle_digits_change() {
    let parts = chart_header_changed_text("82,543.2", "82,613.2").expect("changed text");

    assert_eq!(parts.before, "82,");
    assert_eq!(parts.changed, "61");
    assert_eq!(parts.after, "3.2");
}

#[test]
fn changed_text_ignores_equal_formatted_prices() {
    assert_eq!(chart_header_changed_text("82,543.2", "82,543.2"), None);
}

#[test]
fn signed_usd_change_marks_nonfinite_values_invalid() {
    assert_eq!(format_signed_usd_change(12.5), "+$12.50");
    assert_eq!(format_signed_usd_change(-12.5), "-$12.50");
    assert_eq!(format_signed_usd_change(f64::NAN), "Invalid data");
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
