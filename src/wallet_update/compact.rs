use crate::account::fetch_wallet_details_scoped_with_provider;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::wallet_state::compact::{
    CompactWalletDetailsResult, CompactWalletSelection, CompactWalletTrackerId,
};
use iced::Task;
use std::sync::atomic::{AtomicU64, Ordering};

const COMPACT_WALLET_REFRESH_MS: u64 = 60_000;
// Requests remain unique across pane reuse, layout changes, and config resets.
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

impl TradingTerminal {
    pub(crate) fn wallet_tracker_is_visible(&self) -> bool {
        self.wallet_tracker.window_id.is_some()
            || self.pane_is_open(|kind| matches!(kind, PaneKind::CompactWalletTracker(_)))
    }

    fn compact_wallet_pane_is_open(&self, id: CompactWalletTrackerId) -> bool {
        self.pane_is_open(
            |kind| matches!(kind, PaneKind::CompactWalletTracker(open) if *open == id),
        )
    }

    pub(super) fn update_compact_wallet_tracker(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CompactWalletSelected(id, address) => {
                if !self.compact_wallet_pane_is_open(id)
                    || !self
                        .wallet_tracker
                        .tracked_addresses
                        .iter()
                        .any(|tracked| tracked == address.as_str())
                {
                    return Task::none();
                }
                self.wallet_tracker
                    .compact_selections
                    .insert(id, CompactWalletSelection::new(address));
                return self.refresh_compact_wallet(id);
            }
            Message::CompactWalletBack(id) => {
                self.wallet_tracker.compact_selections.remove(&id);
            }
            Message::CompactWalletRefresh(id) => return self.refresh_compact_wallet(id),
            Message::CompactWalletDetailsLoaded(id, request_id, context, result) => {
                if !self.compact_wallet_pane_is_open(id) {
                    self.wallet_tracker.compact_selections.remove(&id);
                    return Task::none();
                }
                let current = self.read_data_request_context_is_current(context);
                let Some(selection) = self.wallet_tracker.compact_selections.get_mut(&id) else {
                    return Task::none();
                };
                if selection.pending_request != Some((request_id, context)) {
                    return Task::none();
                }
                selection.pending_request = None;
                if !current {
                    selection.last_attempt_ms = None;
                    return Task::none();
                }
                match result.0 {
                    Ok(data) => {
                        selection.data = Some(data);
                        selection.error = None;
                    }
                    Err(_) => {
                        selection.error = Some(if selection.data.is_some() {
                            "Refresh failed · showing previous positions"
                        } else {
                            "Could not load positions"
                        });
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn refresh_compact_wallet(&mut self, id: CompactWalletTrackerId) -> Task<Message> {
        if !self.compact_wallet_pane_is_open(id) {
            self.wallet_tracker.compact_selections.remove(&id);
            return Task::none();
        }
        let context = self.read_data_request_context();
        let Some(selection) = self.wallet_tracker.compact_selections.get_mut(&id) else {
            return Task::none();
        };
        if selection.pending_request.is_some() {
            return Task::none();
        }
        let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        selection.pending_request = Some((request_id, context));
        selection.last_attempt_ms = Some(Self::now_ms());
        let address = selection.address.as_str().to_string();
        Task::perform(
            fetch_wallet_details_scoped_with_provider(
                address,
                self.account_data_fetch_scope(),
                self.read_data_provider,
                self.hydromancer_api_key_for_task(),
            ),
            move |result| {
                Message::CompactWalletDetailsLoaded(
                    id,
                    request_id,
                    context,
                    CompactWalletDetailsResult(result),
                )
            },
        )
    }

    pub(crate) fn refresh_compact_wallets_due(&mut self) -> Task<Message> {
        let open_ids: Vec<_> = self
            .workspace_pane_kinds()
            .filter_map(|(_, _, kind)| {
                if let PaneKind::CompactWalletTracker(id) = kind {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        self.wallet_tracker
            .compact_selections
            .retain(|id, selection| {
                open_ids.contains(id)
                    && self
                        .wallet_tracker
                        .tracked_addresses
                        .iter()
                        .any(|address| address == selection.address.as_str())
            });
        let now_ms = Self::now_ms();
        let due: Vec<_> = self
            .wallet_tracker
            .compact_selections
            .iter()
            .filter_map(|(id, selection)| {
                (selection.pending_request.is_none()
                    && selection.last_attempt_ms.is_none_or(|last| {
                        now_ms.saturating_sub(last) >= COMPACT_WALLET_REFRESH_MS
                    }))
                .then_some(*id)
            })
            .collect();
        Task::batch(due.into_iter().map(|id| self.refresh_compact_wallet(id)))
    }
}

#[cfg(test)]
mod tests;
