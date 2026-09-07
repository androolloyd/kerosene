use crate::api::proxy::{ProxyPool, ProxyUrl};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;
use zeroize::Zeroize;

impl TradingTerminal {
    pub(super) fn update_hyperliquid_proxies(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HyperliquidProxyInputChanged(value) => {
                self.hyperliquid_proxies.input = value.into_zeroizing().into();
                self.hyperliquid_proxies.status = None;
            }
            Message::SetHyperliquidProxiesEnabled(enabled) => {
                if enabled && self.hyperliquid_proxies.urls.is_empty() {
                    self.hyperliquid_proxies.status =
                        Some(("Add a proxy or unlock saved credentials first".into(), true));
                    return Task::none();
                }
                match ProxyPool::build(enabled, &self.hyperliquid_proxies.urls) {
                    Ok(pool) => {
                        crate::api::proxy::install(pool);
                        self.hyperliquid_proxies.enabled = enabled;
                        self.hyperliquid_proxies.status = None;
                        self.persist_config();
                    }
                    Err(error) => self.hyperliquid_proxies.status = Some((error, true)),
                }
            }
            Message::AddHyperliquidProxy => {
                let proxy = match ProxyUrl::parse(&self.hyperliquid_proxies.input) {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        self.hyperliquid_proxies.status = Some((error, true));
                        return Task::none();
                    }
                };
                if self.hyperliquid_proxies.urls.contains(&proxy) {
                    self.hyperliquid_proxies.status =
                        Some(("This proxy is already configured".into(), true));
                    return Task::none();
                }
                let mut urls = self.hyperliquid_proxies.urls.clone();
                urls.push(proxy);
                self.commit_hyperliquid_proxies(urls);
            }
            Message::RemoveHyperliquidProxy(index)
                if index < self.hyperliquid_proxies.urls.len() =>
            {
                let mut urls = self.hyperliquid_proxies.urls.clone();
                urls.remove(index);
                self.commit_hyperliquid_proxies(urls);
            }
            _ => {}
        }
        Task::none()
    }

    fn commit_hyperliquid_proxies(&mut self, urls: Vec<ProxyUrl>) {
        let enabled = self.hyperliquid_proxies.enabled && !urls.is_empty();
        let pool = match ProxyPool::build(enabled, &urls) {
            Ok(pool) => pool,
            Err(error) => {
                self.hyperliquid_proxies.status = Some((error, true));
                return;
            }
        };
        if !self.persist_hyperliquid_proxy_secrets(&urls) {
            self.hyperliquid_proxies.status = self.secret_store_status.clone();
            return;
        }
        crate::api::proxy::install(pool);
        self.hyperliquid_proxies.urls = urls;
        self.hyperliquid_proxies.enabled = enabled;
        self.hyperliquid_proxies.input.zeroize();
        self.hyperliquid_proxies.status = self.secret_store_status.clone();
        self.persist_config();
    }
}

#[cfg(test)]
mod tests;
