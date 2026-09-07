use crate::app_state::TradingTerminal;
use crate::config;
use crate::market_state::LiveWatchlistInstance;

// ---------------------------------------------------------------------------
// Layout Live-Watchlist Restoration
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn restore_layout_live_watchlists(&mut self, layout: &config::SavedLayout) {
        self.live_watchlists = layout
            .live_watchlists
            .clone()
            .into_iter()
            .map(|watchlist_config| {
                (
                    watchlist_config.id,
                    LiveWatchlistInstance {
                        id: watchlist_config.id,
                        preset_id: watchlist_config.preset_id,
                        symbols: {
                            let mut seen = std::collections::HashSet::new();
                            watchlist_config
                                .symbols
                                .into_iter()
                                .filter(|symbol| !self.is_ticker_muted(symbol))
                                .map(|symbol| {
                                    self.exchange_symbol_for_key(&symbol)
                                        .map(|metadata| metadata.key.clone())
                                        .unwrap_or(symbol)
                                })
                                .filter(|symbol| seen.insert(symbol.clone()))
                                .collect()
                        },
                        search_query: String::new(),
                        sort_column: watchlist_config.sort_column,
                        sort_direction: watchlist_config.sort_direction,
                        visible_columns: watchlist_config.visible_columns,
                        row_cache: Vec::new(),
                    },
                )
            })
            .collect();
        let missing_ids = self
            .workspace_pane_kinds()
            .filter_map(|(_, _, kind)| match kind {
                crate::pane_state::PaneKind::LiveWatchlist(id)
                    if !self.live_watchlists.contains_key(id) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for id in missing_ids {
            let preset_id = self.ensure_default_watchlist_preset();
            let symbols = self
                .watchlist_preset(preset_id)
                .map(|preset| {
                    preset
                        .symbols
                        .iter()
                        .filter(|symbol| !self.symbol_key_is_hidden(symbol))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            self.live_watchlists.insert(
                id,
                LiveWatchlistInstance {
                    id,
                    preset_id: Some(preset_id),
                    symbols,
                    search_query: String::new(),
                    sort_column: Default::default(),
                    sort_direction: Default::default(),
                    visible_columns: config::default_live_watchlist_columns(),
                    row_cache: Vec::new(),
                },
            );
        }
        self.refresh_live_watchlist_row_caches();
    }
}
