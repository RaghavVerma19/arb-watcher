use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub symbol: String,
    pub decimals: u8,
    #[serde(default)]
    pub mint: Option<String>,
}

impl Token {
    pub fn decimals_pow(&self) -> u64 {
        10u64.pow(self.decimals as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_pow_uses_base_units() {
        let sol = Token {
            symbol: "SOL".into(),
            decimals: 9,
            mint: None,
        };
        assert_eq!(sol.decimals_pow(), 1_000_000_000);
    }
}
