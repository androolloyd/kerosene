use crate::account::WalletDetailsData;
use crate::message::RedactedAddress;
use crate::read_data_provider::ReadDataRequestContext;
use std::fmt;

pub(crate) type CompactWalletTrackerId = u64;

/// Selection is local to a pane and deliberately absent from saved layouts.
#[derive(Debug)]
pub(crate) struct CompactWalletSelection {
    pub(crate) address: RedactedAddress,
    pub(crate) data: Option<WalletDetailsData>,
    pub(crate) pending_request: Option<(u64, ReadDataRequestContext)>,
    pub(crate) last_attempt_ms: Option<u64>,
    pub(crate) error: Option<&'static str>,
}

impl CompactWalletSelection {
    pub(crate) fn new(address: RedactedAddress) -> Self {
        Self {
            address,
            data: None,
            pending_request: None,
            last_attempt_ms: None,
            error: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct CompactWalletDetailsResult(pub(crate) Result<WalletDetailsData, String>);

impl fmt::Debug for CompactWalletDetailsResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CompactWalletDetailsResult(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalletPositionBias {
    Long,
    Short,
    Balanced,
    Flat,
    Unavailable,
}

/// A side dominates when it accounts for at least two thirds of gross notional.
/// Counting positions would let many small positions outweigh one large hedge.
pub(crate) fn wallet_position_bias(long: Option<f64>, short: Option<f64>) -> WalletPositionBias {
    let Some((long, short)) = long.zip(short) else {
        return WalletPositionBias::Unavailable;
    };
    if !long.is_finite() || !short.is_finite() || long < 0.0 || short < 0.0 {
        return WalletPositionBias::Unavailable;
    }
    if long == 0.0 && short == 0.0 {
        WalletPositionBias::Flat
    } else if long / 2.0 >= short {
        WalletPositionBias::Long
    } else if short / 2.0 >= long {
        WalletPositionBias::Short
    } else {
        WalletPositionBias::Balanced
    }
}

#[cfg(test)]
mod tests;
