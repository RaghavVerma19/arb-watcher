use serde::Deserialize;

use crate::Pool;
use crate::Token;

#[derive(Deserialize, Clone, Debug)]
pub struct ScannerConfig {
    pub base_token: String,
    pub base_amount: u64,
    pub min_profit_bps: i64,
    pub max_cycle_len: usize,
    /// Cycles whose zero-fee (gross) edge clears this bar are reported too,
    /// even when net profit is below `min_profit_bps` (near-misses).
    #[serde(default)]
    pub report_min_gross_bps: i64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SimulatorConfig {
    pub volatility: f64,
    pub pool_volatility: f64,
    pub mean_reversion: f64,
    pub tick_interval_ms: u64,
    /// Optional fixed seed for reproducible simulator runs.
    #[serde(default)]
    pub seed: Option<u64>,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            volatility: 0.002,
            pool_volatility: 0.002,
            mean_reversion: 0.9,
            tick_interval_ms: 500,
            seed: None,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct JupiterConfig {
    pub base_url: String,
    pub refresh_ms: u64,
}

impl Default for JupiterConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.jup.ag".to_string(),
            refresh_ms: 2000,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct OnchainConfig {
    pub rpc_url: String,
    /// Minimum wall-clock delay between scans (also the RPC rate-limit floor).
    pub refresh_ms: u64,
    /// How often we poll `getSlot` for a new block while idle.
    pub slot_poll_ms: u64,
    /// When true, scans fire on a NEW slot (chain-driven) instead of a blind
    /// timer. Still subject to `refresh_ms` as a floor.
    pub slot_polling: bool,
}

fn default_slot_poll_ms() -> u64 {
    250
}

fn default_slot_polling() -> bool {
    true
}

impl Default for OnchainConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            refresh_ms: 2000,
            slot_poll_ms: default_slot_poll_ms(),
            slot_polling: default_slot_polling(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct PaperConfig {
    pub enabled: bool,
    pub starting_capital: u64,
    pub min_exec_bps: i64,
    pub live_exec: bool,
}

impl Default for PaperConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            starting_capital: 10_000_000_000,
            min_exec_bps: 50,
            live_exec: false,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ServerConfig {
    pub allowed_origin: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            allowed_origin: "http://localhost:5173".to_string(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct MarketConfig {
    pub tokens: Vec<Token>,
    pub pools: Vec<Pool>,
    pub scanner: ScannerConfig,
    #[serde(default)]
    pub simulator: SimulatorConfig,
    #[serde(default)]
    pub jupiter: JupiterConfig,
    #[serde(default)]
    pub onchain: OnchainConfig,
    #[serde(default)]
    pub paper: PaperConfig,
    #[serde(default)]
    pub server: ServerConfig,
}

impl MarketConfig {
    pub fn parse_toml(text: &str) -> anyhow::Result<Self> {
        let cfg: MarketConfig = toml::from_str(text)?;
        Self::validate(&cfg)?;
        Ok(cfg)
    }

    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::parse_toml(&text)
    }

    fn validate(cfg: &Self) -> anyhow::Result<()> {
        let symbols: std::collections::HashSet<String> =
            cfg.tokens.iter().map(|t| t.symbol.clone()).collect();

        for pool in &cfg.pools {
            if !symbols.contains(&pool.token_a) {
                anyhow::bail!(
                    "pool references unknown token_a '{}' (not in [tokens])",
                    pool.token_a
                );
            }
            if !symbols.contains(&pool.token_b) {
                anyhow::bail!(
                    "pool references unknown token_b '{}' (not in [tokens])",
                    pool.token_b
                );
            }
            if pool.reserve_a == 0 {
                eprintln!("warning: pool {}/{} has zero reserve_a", pool.token_a, pool.token_b);
            }
            if pool.reserve_b == 0 {
                eprintln!("warning: pool {}/{} has zero reserve_b", pool.token_a, pool.token_b);
            }
        }

        for token in &cfg.tokens {
            if let Some(ref mint) = token.mint {
                if bs58::decode(mint).into_vec().is_err() {
                    anyhow::bail!("token '{}' has invalid base58 mint '{}'", token.symbol, mint);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_our_config_file() {
        let cfg = MarketConfig::from_file("../../config.toml").expect("load config.toml");

        assert_eq!(cfg.tokens.len(), 7);
        assert_eq!(cfg.pools.len(), 12);
        assert_eq!(cfg.scanner.base_token, "USDC");
        assert_eq!(cfg.scanner.max_cycle_len, 5);

        let sol = cfg.tokens.iter().find(|t| t.symbol == "SOL").unwrap();
        assert_eq!(sol.decimals, 9);
    }
}
