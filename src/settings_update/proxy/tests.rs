use crate::api::proxy::ProxyUrl;
use crate::app_state::TradingTerminal;
use crate::config::{CredentialStorageMode, KeroseneConfig, SecretPayload};
use crate::message::Message;

fn terminal() -> TradingTerminal {
    TradingTerminal::boot_from_config(KeroseneConfig::default()).0
}

fn snapshot(terminal: &mut TradingTerminal) -> KeroseneConfig {
    let mut saved = None;
    terminal
        .persist_config_immediately_with(|config| {
            saved = Some(config.clone());
            Ok(())
        })
        .expect("save snapshot");
    saved.expect("snapshot captured")
}

fn encrypted_terminal() -> TradingTerminal {
    let mut terminal = terminal();
    terminal.secret_storage_mode = CredentialStorageMode::EncryptedConfig;
    terminal.encrypted_secrets_unlocked = true;
    terminal.encrypted_secret_password = "test-only-password".to_string().into();
    terminal
}

#[test]
fn proxy_settings_add_deduplicate_enable_and_remove() {
    let mut terminal = encrypted_terminal();
    for input in ["http://proxy.test:80/", "http://PROXY.test"] {
        let _ = terminal.update(Message::HyperliquidProxyInputChanged(input.into()));
        let _ = terminal.update(Message::AddHyperliquidProxy);
    }
    assert_eq!(terminal.hyperliquid_proxies.urls.len(), 1);
    assert!(!terminal.hyperliquid_proxies.enabled);
    assert!(
        terminal
            .hyperliquid_proxies
            .status
            .as_ref()
            .expect("duplicate status")
            .1
    );
    let _ = terminal.update(Message::SetHyperliquidProxiesEnabled(true));
    assert!(terminal.hyperliquid_proxies.enabled);
    assert!(snapshot(&mut terminal).hyperliquid_proxies_enabled);
    let _ = terminal.update(Message::RemoveHyperliquidProxy(99));
    assert_eq!(terminal.hyperliquid_proxies.urls.len(), 1);
    let _ = terminal.update(Message::RemoveHyperliquidProxy(0));
    assert!(terminal.hyperliquid_proxies.urls.is_empty());
    assert!(!terminal.hyperliquid_proxies.enabled);
    assert!(!snapshot(&mut terminal).hyperliquid_proxies_enabled);
}

#[test]
fn invalid_proxy_and_empty_enable_do_not_change_settings() {
    let mut terminal = terminal();
    let _ = terminal.update(Message::SetHyperliquidProxiesEnabled(true));
    assert!(!terminal.hyperliquid_proxies.enabled);
    let _ = terminal.update(Message::HyperliquidProxyInputChanged(
        "http://user:sentinel-secret@host/path".into(),
    ));
    let _ = terminal.update(Message::AddHyperliquidProxy);
    assert!(terminal.hyperliquid_proxies.urls.is_empty());
    assert!(!format!("{:?}", terminal.hyperliquid_proxies).contains("sentinel-secret"));
}

#[test]
fn failed_secret_save_keeps_active_proxies_and_input() {
    let mut terminal = encrypted_terminal();
    let original = ProxyUrl::parse("http://original.test").expect("URL");
    terminal.hyperliquid_proxies.urls.push(original.clone());
    terminal.hyperliquid_proxies.enabled = true;
    // Missing storage password prevents the credential mutation from committing.
    terminal.encrypted_secret_password = Default::default();
    let _ = terminal.update(Message::HyperliquidProxyInputChanged(
        "http://new.test".into(),
    ));
    let _ = terminal.update(Message::AddHyperliquidProxy);
    assert_eq!(
        terminal.hyperliquid_proxies.urls.as_slice(),
        std::slice::from_ref(&original)
    );
    assert_eq!(
        terminal.hyperliquid_proxies.input.as_str(),
        "http://new.test"
    );
    let _ = terminal.update(Message::RemoveHyperliquidProxy(0));
    assert_eq!(terminal.hyperliquid_proxies.urls, [original]);
    assert!(terminal.hyperliquid_proxies.enabled);
}

#[test]
fn proxy_credentials_round_trip_encrypted_storage_and_stay_out_of_snapshots() {
    let mut terminal = encrypted_terminal();
    let _ = terminal.update(Message::HyperliquidProxyInputChanged(
        "socks5h://sentinel-user:sentinel-password@proxy.test:1080".into(),
    ));
    let _ = terminal.update(Message::AddHyperliquidProxy);
    assert_eq!(terminal.hyperliquid_proxies.urls.len(), 1);
    let snapshot = snapshot(&mut terminal);
    assert!(snapshot.hyperliquid_proxy_urls.is_empty());
    let json = serde_json::to_string(&snapshot).expect("config JSON");
    assert!(!json.contains("sentinel"));
    assert!(!json.contains("proxy.test"));
    let payload = crate::config::decrypt_secrets(
        snapshot.encrypted_secrets.as_ref().expect("saved blob"),
        "test-only-password",
    )
    .expect("decrypt");
    assert_eq!(
        payload.global.hyperliquid_proxy_urls,
        terminal.hyperliquid_proxies.urls
    );
    assert_eq!(
        terminal
            .current_secret_payload()
            .global
            .hyperliquid_proxy_urls,
        terminal.hyperliquid_proxies.urls
    );
    let mut restored = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    restored.hyperliquid_proxies.enabled = true;
    restored.apply_secret_payload(payload);
    assert_eq!(restored.hyperliquid_proxies.urls.len(), 1);
    assert!(restored.hyperliquid_proxies.enabled);
}

#[test]
fn old_configs_default_to_disabled_and_raw_proxy_urls_are_ignored() {
    let mut value = serde_json::to_value(KeroseneConfig::default()).expect("config");
    value
        .as_object_mut()
        .expect("object")
        .remove("hyperliquid_proxies_enabled");
    value["hyperliquid_proxy_urls"] = serde_json::json!(["http://sentinel-password@host"]);
    let config: KeroseneConfig = serde_json::from_value(value).expect("old config");
    assert!(!config.hyperliquid_proxies_enabled);
    assert!(config.hyperliquid_proxy_urls.is_empty());
    let old_payload: SecretPayload =
        serde_json::from_str(r#"{"schema":"kerosene.secrets.v1","global":{}}"#)
            .expect("old secrets");
    assert!(old_payload.global.hyperliquid_proxy_urls.is_empty());
    let urls = vec![ProxyUrl::parse("https://sentinel-password@proxy.test").expect("URL")];
    let payload = old_payload.with_hyperliquid_proxies(&urls);
    assert!(!payload.is_empty());
    assert!(!format!("{payload:?}").contains("sentinel"));
    let restored: SecretPayload =
        serde_json::from_str(&serde_json::to_string(&payload).expect("secret JSON"))
            .expect("secret round trip");
    assert_eq!(restored.global.hyperliquid_proxy_urls, urls);
}
