use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

/// The complete URL can contain credentials. Only the secret payload serializes it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ProxyUrl(Zeroizing<String>);

impl fmt::Debug for ProxyUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProxyUrl(<redacted>)")
    }
}

impl ProxyUrl {
    pub(crate) fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        let invalid = || {
            "Enter a proxy URL: http://host:port, https://host:port, socks5://host:port, or socks5h://host:port".to_string()
        };
        if input
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
        {
            return Err(invalid());
        }
        let mut url = reqwest::Url::parse(input).map_err(|_| invalid())?;
        if !matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h")
            || url.host_str().is_none_or(str::is_empty)
            || !matches!(url.path(), "" | "/")
            || url.query().is_some()
            || url.fragment().is_some()
            || url.port() == Some(0)
        {
            return Err(invalid());
        }
        if matches!(url.scheme(), "socks5" | "socks5h") && url.port().is_none() {
            url.set_port(Some(1080)).map_err(|_| invalid())?;
        }
        url.set_path("");
        Ok(Self(url.to_string().into()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Safe label for the settings list; neither username nor password is shown.
    pub(crate) fn label(&self) -> String {
        let Ok(url) = reqwest::Url::parse(self.as_str()) else {
            return "Invalid proxy".into();
        };
        let Some(host) = url.host_str() else {
            return "Invalid proxy".into();
        };
        let port = url.port_or_known_default().unwrap_or(1080);
        let auth = if !url.username().is_empty() || url.password().is_some() {
            " (authenticated)"
        } else {
            ""
        };
        format!("{}://{host}:{port}{auth}", url.scheme())
    }
}
