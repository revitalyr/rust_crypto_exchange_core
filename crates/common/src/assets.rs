//! Asset definitions and cryptocurrency-specific types.

use serde::{Deserialize, Serialize, Serializer, Deserializer, de::Error};
use std::fmt;
use std::str::FromStr;

/// Represents different cryptocurrency assets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Serde serialization module for Asset
pub mod serde_asset {
    use super::*;
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(asset: &Asset, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(asset.symbol())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Asset, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Asset::from_str(&s).map_err(D::Error::custom)
    }
}

impl FromStr for Asset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BTC" => Ok(Asset::BTC),
            "ETH" => Ok(Asset::ETH),
            "USDT" => Ok(Asset::USDT),
            "USDC" => Ok(Asset::USDC),
            _ => Ok(Asset::Custom(Box::leak(s.to_string().into_boxed_str()))),
        }
    }
}

impl Serialize for Asset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.symbol())
    }
}

impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Asset::from_str(&s).map_err(D::Error::custom)
    }
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
    #[serde(with = "serde_asset")]
    pub base: Asset,
    #[serde(with = "serde_asset")]
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
