use super::*;
use crate::account::{
    ClearinghouseState, MarginSummary, SpotClearinghouseState, WalletDetailsData,
};
use crate::canvas_state::WorkspaceId;
use crate::config::{KeroseneConfig, PaneKindConfig, PaneLayoutConfig};

const FIRST: &str = "0xabc0000000000000000000000000000000000000";
const SECOND: &str = "0xdef0000000000000000000000000000000000000";

fn terminal() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.wallet_tracker.tracked_addresses = vec![FIRST.to_string(), SECOND.to_string()];
    let _ = terminal.update(Message::AddCompactWalletTrackerPane);
    let _ = terminal.update(Message::AddCompactWalletTrackerPane);
    terminal
}

fn select(
    terminal: &mut TradingTerminal,
    id: u64,
    address: &str,
) -> (u64, crate::read_data_provider::ReadDataRequestContext) {
    let _ = terminal.update(Message::CompactWalletSelected(id, address.into()));
    terminal.wallet_tracker.compact_selections[&id]
        .pending_request
        .expect("selection starts loading")
}

fn snapshot() -> WalletDetailsData {
    WalletDetailsData {
        clearinghouse: ClearinghouseState {
            margin_summary: MarginSummary {
                account_value: "1000".into(),
                total_ntl_pos: "0".into(),
                total_margin_used: "0".into(),
            },
            cross_margin_summary: None,
            cross_maintenance_margin_used: None,
            withdrawable: "1000".into(),
            asset_positions: vec![],
        },
        spot: SpotClearinghouseState {
            balances: vec![],
            portfolio_margin_enabled: false,
            portfolio_margin_ratio: None,
            token_to_available_after_maintenance: None,
        },
        positions: vec![],
        open_orders: vec![],
        fills: vec![],
        warnings: vec![],
        fetched_at_ms: 42,
    }
}

fn loaded(
    id: u64,
    request: (u64, crate::read_data_provider::ReadDataRequestContext),
    result: Result<WalletDetailsData, String>,
) -> Message {
    Message::CompactWalletDetailsLoaded(
        id,
        request.0,
        request.1,
        CompactWalletDetailsResult(result),
    )
}

#[test]
fn compact_wallet_selections_are_independent_and_never_switch_the_trading_account() {
    let mut terminal = terminal();
    let connected = terminal.connected_address.clone();
    let first = select(&mut terminal, 0, FIRST);
    select(&mut terminal, 1, SECOND);
    let _ = terminal.update(loaded(0, first, Ok(snapshot())));
    assert!(
        terminal.wallet_tracker.compact_selections[&0]
            .data
            .is_some()
    );
    assert!(
        terminal.wallet_tracker.compact_selections[&1]
            .data
            .is_none()
    );
    assert!(terminal.wallet_detail_windows.is_empty());
    assert_eq!(terminal.connected_address, connected);
    let _ = terminal.update(Message::CompactWalletBack(0));
    assert!(!terminal.wallet_tracker.compact_selections.contains_key(&0));
    assert!(terminal.wallet_tracker.compact_selections.contains_key(&1));
    let _ = terminal.update(loaded(0, first, Ok(snapshot())));
    assert!(!terminal.wallet_tracker.compact_selections.contains_key(&0));
}

#[test]
fn compact_wallet_reselection_rejects_late_results_even_for_the_same_address() {
    let mut terminal = terminal();
    let first = select(&mut terminal, 0, FIRST);
    select(&mut terminal, 0, SECOND);
    let latest = select(&mut terminal, 0, FIRST);
    let _ = terminal.update(loaded(0, first, Err("old failure".into())));
    let selection = &terminal.wallet_tracker.compact_selections[&0];
    assert_eq!(selection.pending_request, Some(latest));
    assert!(selection.error.is_none());
    let _ = terminal.update(loaded(0, latest, Ok(snapshot())));
    assert!(
        terminal.wallet_tracker.compact_selections[&0]
            .data
            .is_some()
    );
}

#[test]
fn compact_wallet_provider_change_discards_inflight_and_cached_data() {
    let mut terminal = terminal();
    let first = select(&mut terminal, 0, FIRST);
    let _ = terminal.update(loaded(0, first, Ok(snapshot())));
    let _ = terminal.update(Message::CompactWalletRefresh(0));
    let old = terminal.wallet_tracker.compact_selections[&0]
        .pending_request
        .expect("refresh");
    terminal.read_data_provider_generation += 1;
    terminal.invalidate_wallet_read_data_requests();
    let _ = terminal.refresh_compact_wallets_due();
    let latest = terminal.wallet_tracker.compact_selections[&0]
        .pending_request
        .expect("new request");
    assert_ne!(old, latest);
    let _ = terminal.update(loaded(0, old, Ok(snapshot())));
    let selection = &terminal.wallet_tracker.compact_selections[&0];
    assert!(selection.data.is_none());
    assert_eq!(selection.pending_request, Some(latest));
}

#[test]
fn compact_wallet_failed_refresh_preserves_snapshot_and_backs_off() {
    let mut terminal = terminal();
    let first = select(&mut terminal, 0, FIRST);
    let _ = terminal.update(loaded(0, first, Ok(snapshot())));
    let _ = terminal.update(Message::CompactWalletRefresh(0));
    let refresh = terminal.wallet_tracker.compact_selections[&0]
        .pending_request
        .expect("refresh");
    let _ = terminal.update(loaded(0, refresh, Err("private error".into())));
    let _ = terminal.refresh_compact_wallets_due();
    let selection = &terminal.wallet_tracker.compact_selections[&0];
    assert_eq!(
        selection
            .data
            .as_ref()
            .expect("retained data")
            .fetched_at_ms,
        42
    );
    assert!(selection.error.is_some());
    assert!(selection.pending_request.is_none());
    terminal
        .wallet_tracker
        .compact_selections
        .get_mut(&0)
        .expect("selection")
        .last_attempt_ms = Some(0);
    let _ = terminal.update(Message::WalletTrackerRefreshDue);
    assert!(
        terminal.wallet_tracker.compact_selections[&0]
            .pending_request
            .is_some()
    );
}

#[test]
fn compact_wallet_close_remove_and_layout_restore_clear_selections() {
    let mut terminal = terminal();
    let first = select(&mut terminal, 0, FIRST);
    let pane = terminal
        .find_pane_matching(|kind| matches!(kind, PaneKind::CompactWalletTracker(0)))
        .expect("pane");
    let _ = terminal.update(Message::ClosePane(WorkspaceId::Main, pane));
    assert!(!terminal.wallet_tracker.compact_selections.contains_key(&0));
    let _ = terminal.update(Message::AddCompactWalletTrackerPane);
    let replacement = select(&mut terminal, 0, FIRST);
    let _ = terminal.update(loaded(0, first, Ok(snapshot())));
    assert_eq!(
        terminal.wallet_tracker.compact_selections[&0].pending_request,
        Some(replacement)
    );
    let _ = terminal.update(Message::WalletTrackerRemove(FIRST.into()));
    assert!(!terminal.wallet_tracker.compact_selections.contains_key(&0));
    select(&mut terminal, 0, SECOND);
    let layout = terminal.saved_layout_snapshot("compact".into());
    let _ = terminal.apply_layout(layout);
    assert!(terminal.wallet_tracker.compact_selections.is_empty());
    assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::CompactWalletTracker(0))));
}

#[test]
fn compact_wallet_restores_without_instance_config_and_enables_background_refresh() {
    let config = KeroseneConfig {
        pane_layout: Some(PaneLayoutConfig::Leaf(
            PaneKindConfig::CompactWalletTracker { id: 42 },
        )),
        ..KeroseneConfig::default()
    };
    let terminal = TradingTerminal::boot_from_config(config).0;
    assert!(terminal.wallet_tracker.window_id.is_none());
    assert!(terminal.wallet_tracker_is_visible());
    assert!(terminal.wallet_tracker.compact_selections.is_empty());
    assert!(terminal.pane_is_open(|kind| matches!(kind, PaneKind::CompactWalletTracker(42))));
    let saved = terminal.saved_layout_snapshot("restored compact".into());
    let mut restored = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    let _ = restored.apply_layout(saved);
    assert!(restored.pane_is_open(|kind| matches!(kind, PaneKind::CompactWalletTracker(42))));
}

#[test]
fn compact_wallet_ignores_selection_of_untracked_wallets_and_missing_panes() {
    let mut terminal = terminal();
    let _ = terminal.update(Message::CompactWalletSelected(999, FIRST.into()));
    let _ = terminal.update(Message::CompactWalletSelected(
        0,
        "0x1230000000000000000000000000000000000000".into(),
    ));
    assert!(terminal.wallet_tracker.compact_selections.is_empty());
}
