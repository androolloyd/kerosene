use super::*;
use crate::account::{WalletDetailsData, WalletPositionDetail, WalletTrackerSnapshot};
use crate::config::KeroseneConfig;
use crate::wallet_state::{AddressBookEntry, WalletTrackerRow};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, renderer, widget::Tree};
use iced::{Event, Font, Pixels, Point, Rectangle, Size};

const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

#[test]
fn compact_wallet_money_preserves_small_pnl_and_abbreviates_large_values() {
    let denomination = crate::denomination::DisplayDenominationContext::usd();
    assert_eq!(compact_money(&denomination, 0.25, true), "+$0.25");
    assert_eq!(compact_money(&denomination, -0.25, true), "-$0.25");
    assert_eq!(compact_money(&denomination, 2_000_000.0, false), "$2.00M");
    assert_eq!(compact_money(&denomination, -25_000.0, true), "-$25.0K");
}

#[tokio::test]
async fn compact_wallet_tables_render_and_emit_navigation_at_compact_and_wide_sizes() {
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal.wallet_tracker.tracked_addresses = vec![ADDRESS.into()];
    terminal.address_book.insert(
        ADDRESS.into(),
        AddressBookEntry {
            label: "Macro wallet".into(),
            ..Default::default()
        },
    );
    terminal.wallet_tracker.rows.insert(
        ADDRESS.into(),
        WalletTrackerRow {
            snapshot: Some(WalletTrackerSnapshot {
                equity: Some(2_345_678.0),
                withdrawable: Some(1_000_000.0),
                unrealized_pnl: Some(-12_345.67),
                margin_used_pct: Some(25.0),
                open_trade_count: Some(3),
                open_order_count: 0,
                long_exposure: Some(100_000.0),
                short_exposure: Some(900_000.0),
                valuation_warning: None,
            }),
            last_updated_ms: Some(terminal.status_bar_now_ms),
            ..Default::default()
        },
    );
    for width in [336.0, 640.0] {
        let messages = render_and_click(
            &terminal,
            &mut renderer,
            width,
            "list",
            Point::new(30.0, 50.0),
        );
        assert!(messages.iter().any(|message| matches!(message, Message::CompactWalletSelected(7, address) if address.as_str() == ADDRESS)));
    }
    let mut selection = CompactWalletSelection::new(ADDRESS.into());
    let clearinghouse = serde_json::from_value(serde_json::json!({
        "marginSummary": {"accountValue": "2345678", "totalNtlPos": "1000000", "totalMarginUsed": "250000"},
        "withdrawable": "1000000", "assetPositions": []
    })).expect("fixture clearinghouse");
    let spot = serde_json::from_value(serde_json::json!({"balances": []})).expect("fixture spot");
    let positions = [
        ("", "BTC", "-3.5", "65000", "227500", "-12345.67"),
        ("", "ETH", "25", "3400", "85000", "1200.25"),
        ("xyz", "GOLD", "12.5", "2800", "35000", "0.25"),
    ]
    .into_iter()
    .map(
        |(dex, coin, size, entry, value, upnl)| WalletPositionDetail {
            dex: dex.into(),
            asset_position: serde_json::from_value(serde_json::json!({
                "position": {"coin": coin, "szi": size, "entryPx": entry,
                    "positionValue": value, "unrealizedPnl": upnl, "marginUsed": "1000",
                    "leverage": {"type": "cross", "value": 5}}
            }))
            .expect("fixture position"),
        },
    )
    .collect();
    selection.data = Some(WalletDetailsData {
        clearinghouse,
        spot,
        positions,
        open_orders: vec![],
        fills: vec![],
        warnings: vec![],
        fetched_at_ms: terminal.status_bar_now_ms,
    });
    terminal
        .wallet_tracker
        .compact_selections
        .insert(7, selection);
    for width in [336.0, 640.0] {
        let messages = render_and_click(
            &terminal,
            &mut renderer,
            width,
            "positions",
            Point::new(30.0, 20.0),
        );
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, Message::CompactWalletBack(7)))
        );
    }
}

fn render_and_click(
    terminal: &TradingTerminal,
    renderer: &mut iced::Renderer,
    width: f32,
    name: &str,
    point: Point,
) -> Vec<Message> {
    let size = Size::new(width, 240.0);
    let bounds = Rectangle::with_size(size);
    let theme = terminal.theme();
    let mut view = terminal.view_compact_wallet_tracker(7);
    let mut tree = Tree::new(view.as_widget());
    let node = view
        .as_widget_mut()
        .layout(&mut tree, renderer, &layout::Limits::new(size, size));
    assert_eq!(node.size(), size);
    iced::advanced::Renderer::reset(renderer, bounds);
    view.as_widget().draw(
        &tree,
        renderer,
        &theme,
        &renderer::Style {
            text_color: theme.palette().text,
        },
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &bounds,
    );
    let pixels = renderer.screenshot(
        Size::new(width as u32, 240),
        1.0,
        theme.palette().background,
    );
    assert_eq!(pixels.len(), width as usize * 240 * 4);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    // Optional synthetic-data previews for visual QA, never real account state.
    if let Some(directory) = std::env::var_os("KEROSENE_COMPACT_PREVIEW_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("preview directory");
        image::save_buffer(
            directory.join(format!("{name}-{}.png", width as u32)),
            &pixels,
            width as u32,
            240,
            image::ColorType::Rgba8,
        )
        .expect("save synthetic preview");
    }
    let mut messages = Vec::new();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        view.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(event),
            Layout::new(&node),
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &bounds,
        );
    }
    messages
}
