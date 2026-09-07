use crate::app_state::TradingTerminal;
use crate::config::KeroseneConfig;
use iced::Color;

#[test]
fn hyperdash_uses_reference_surfaces_and_candle_colors() {
    let (terminal, _) = TradingTerminal::boot_from_config(KeroseneConfig {
        active_theme: "Custom: Hyperdash".to_string(),
        ..KeroseneConfig::default()
    });
    let theme = terminal.theme();
    let palette = theme.palette();
    let extended = theme.extended_palette();

    assert!(TradingTerminal::palette_matches_hyperdash_source(palette));
    assert_eq!(extended.background.base.color, palette.background);
    assert_eq!(
        extended.background.weak.color,
        Color::from_rgb8(0x1D, 0x1A, 0x18)
    );
    assert_eq!(
        extended.background.strong.color,
        Color::from_rgb8(0x19, 0x16, 0x13)
    );
    assert_eq!(
        extended.background.weak.text,
        Color::from_rgb8(0x92, 0x8D, 0x86)
    );
    assert_eq!(extended.primary.base.color, palette.primary);
    assert_eq!(extended.primary.base.text, Color::WHITE);
    assert_eq!(extended.success.base.color, palette.success);
    assert_eq!(extended.danger.base.color, palette.danger);
    assert!(extended.is_dark);

    let overrides = terminal.active_chart_theme_overrides();
    assert_eq!(overrides.bull, Some(Color::from_rgb8(0x38, 0xA6, 0x7C)));
    assert_eq!(overrides.bear, Some(Color::from_rgb8(0xBC, 0x26, 0x3E)));
    assert_eq!(overrides.line, None);
    assert_eq!(overrides.line_gradient, None);
    assert_eq!(
        terminal.direction_colors(&theme),
        (
            Color::from_rgb8(0x38, 0xA6, 0x7C),
            Color::from_rgb8(0xBC, 0x26, 0x3E)
        )
    );
}

#[test]
fn hyperdash_customizations_use_generated_surfaces() {
    for field in [
        "background",
        "text",
        "primary",
        "success",
        "warning",
        "danger",
    ] {
        let mut config = KeroseneConfig::default();
        let hyperdash = config
            .custom_themes
            .iter_mut()
            .find(|theme| theme.name == "Hyperdash")
            .expect("Hyperdash preset");
        let mut value = serde_json::to_value(&*hyperdash).expect("theme serializes");
        value[field] = serde_json::json!("#123456");
        *hyperdash = serde_json::from_value(value).expect("customized theme deserializes");

        let (terminal, _) = TradingTerminal::boot_from_config(config);
        let theme = terminal.get_theme_by_name("Custom: Hyperdash");

        assert!(
            !TradingTerminal::palette_matches_hyperdash_source(theme.palette()),
            "customized {field} must disable the source palette"
        );
        assert_ne!(
            theme.extended_palette().background.weak,
            TradingTerminal::hyperdash_source_extended_palette()
                .background
                .weak,
            "customized {field} must use generated surfaces"
        );
    }
}
