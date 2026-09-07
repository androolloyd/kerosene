use super::*;

#[test]
fn compact_wallet_bias_uses_dominant_notional_and_handles_hedges() {
    use WalletPositionBias::*;
    for (long, short, expected) in [
        (0.0, 0.0, Flat),
        (10.0, 0.0, Long),
        (0.0, 10.0, Short),
        (200.0, 100.0, Long),
        (100.0, 200.0, Short),
        (199.0, 100.0, Balanced),
        (100.0, 199.0, Balanced),
        (100.0, 100.0, Balanced),
        (f64::MAX, f64::MAX, Balanced),
        (f64::MAX, 1.0, Long),
    ] {
        assert_eq!(wallet_position_bias(Some(long), Some(short)), expected);
    }
}

#[test]
fn compact_wallet_bias_does_not_turn_missing_or_invalid_values_into_flat() {
    for (long, short) in [
        (None, Some(0.0)),
        (Some(0.0), None),
        (Some(f64::NAN), Some(0.0)),
        (Some(0.0), Some(f64::INFINITY)),
        (Some(-1.0), Some(10.0)),
        (Some(10.0), Some(-1.0)),
    ] {
        assert_eq!(
            wallet_position_bias(long, short),
            WalletPositionBias::Unavailable
        );
    }
}

#[test]
fn compact_wallet_debug_redacts_selection_and_response_errors() {
    let address = "0xabc0000000000000000000000000000000000000";
    let selection = CompactWalletSelection::new(address.into());
    assert!(!format!("{selection:?}").contains(address));
    let result = CompactWalletDetailsResult(Err("sentinel-secret-error".to_string()));
    let message = crate::message::Message::CompactWalletDetailsLoaded(
        7,
        1,
        crate::read_data_provider::ReadDataRequestContext {
            provider: crate::config::ReadDataProvider::Hyperliquid,
            read_data_provider_generation: 0,
            hydromancer_key_generation: 0,
        },
        result,
    );
    assert!(!format!("{message:?}").contains("sentinel-secret-error"));
    assert!(
        !format!(
            "{:?}",
            crate::message::Message::CompactWalletSelected(7, address.into())
        )
        .contains(address)
    );
}
