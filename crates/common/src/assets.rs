//! Asset definitions and cryptocurrency-specific types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents different cryptocurrency assets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Asset {
    /// Bitcoin
    BTC,
    /// Ethereum
    ETH,
    /// Tether USD
    USDT,
    /// USD Coin
    USDC,
    /// Custom asset
    Custom(&'static str),
}

impl Asset {
    /// Returns the symbol for this asset
    pub fn symbol(&self) -> &'static str {
        match self {
            Asset::BTC => "BTC",
            Asset::ETH => "ETH",
            Asset::USDT => "USDT",
            Asset::USDC => "USDC",
            Asset::Custom(symbol) => symbol,
        }
    }

    /// Returns the number of decimal places for this asset
    pub fn decimals(&self) -> u8 {
        match self {
            Asset::BTC => 8,
            Asset::ETH => 18,
            Asset::USDT => 6,
            Asset::USDC => 6,
            Asset::Custom(_) => 8, // Default for custom assets
        }
    }

    /// Returns the minimum lot size for this asset
    pub fn lot_size(&self) -> u64 {
        match self {
            Asset::BTC => 1, // 1 satoshi
            Asset::ETH => 1, // 1 wei
            Asset::USDT => 1, // 1 micro USDT
            Asset::USDC => 1, // 1 micro USDC
            Asset::Custom(_) => 1,
        }
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Represents a trading pair
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradingPair {
    pub base: Asset,
    pub quote: Asset,
}

impl TradingPair {
    /// Creates a new trading pair
    pub fn new(base: Asset, quote: Asset) -> Self {
        Self { base, quote }
    }

    /// Returns the symbol for this trading pair
    pub fn symbol(&self) -> String {
        format!("{}/{}", self.base.symbol(), self.quote.symbol())
    }

    /// Returns the tick size for this pair
    pub fn tick_size(&self) -> u64 {
        match (self.base, self.quote) {
            (Asset::BTC, Asset::USDT) => 100, // $0.0001
            (Asset::ETH, Asset::USDT) => 1000, // $0.001
            _ => 1,
        }
    }

    /// Returns the minimum order size for this pair
    pub fn min_order_size(&self) -> u64 {
        match (self.base, self.quote) {
            (Asset::BTC, Asset::USDT) => 1_000, // 0.00001 BTC
            (Asset::ETH, Asset::USDT) => 100_000, // 0.0001 ETH
            _ => 1,
        }
    }
}

impl fmt::Display for TradingPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_properties() {
        assert_eq!(Asset::BTC.symbol(), "BTC");
        assert_eq!(Asset::BTC.decimals(), 8);
        assert_eq!(Asset::USDT.decimals(), 6);
    }

    #[test]
    fn test_trading_pair() {
        let pair = TradingPair::new(Asset::BTC, Asset::USDT);
        assert_eq!(pair.symbol(), "BTC/USDT");
        assert_eq!(pair.tick_size(), 100);
    }
}
