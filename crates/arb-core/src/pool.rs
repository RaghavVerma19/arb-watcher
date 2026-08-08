use serde::{Deserialize, Serialize};

/// The on-chain program family a pool lives on. Each DEX has a different
/// account layout and swap curve, so the parser dispatches on this field.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Dex {
    /// Raydium AMM v4: constant-product (x*y=k), reserves = vault balances.
    Raydium,
    /// Orca Whirlpools: concentrated liquidity, curve from liquidity + sqrt_price.
    Orca,
}

impl Default for Dex {
    fn default() -> Self {
        Dex::Raydium
    }
}

impl Dex {
    pub fn as_str(&self) -> &'static str {
        match self {
            Dex::Raydium => "raydium",
            Dex::Orca => "orca",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pool {
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub dex: Dex,
}

impl Pool {
    pub fn new(
        token_a: String,
        token_b: String,
        reserve_a: u64,
        reserve_b: u64,
        fee_bps: u16,
    ) -> Self {
        Self {
            token_a,
            token_b,
            reserve_a,
            reserve_b,
            fee_bps,
            address: None,
            dex: Dex::Raydium,
        }
    }

    pub fn contains(&self, symbol: &str) -> bool {
        self.token_a == symbol || self.token_b == symbol
    }

    pub fn reserve_of(&self, symbol: &str) -> Option<u64> {
        if self.token_a == symbol {
            Some(self.reserve_a)
        } else if self.token_b == symbol {
            Some(self.reserve_b)
        } else {
            None
        }
    }
}
