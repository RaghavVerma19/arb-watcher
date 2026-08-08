use std::sync::Mutex;
use std::time::Duration;

use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use tokio::sync::broadcast;

use arb_core::{Dex, MarketConfig, Pool};

use crate::retry;
use crate::scan::scan;
use crate::scanner::ScannerEvent;

// --- Raydium AMM v4 "LiquidityStateV4" account layout (752 bytes total). ---
// Field order is documented in raydium-sdk-v1/src/liquidity/layout.ts.
const BASE_DECIMAL_OFF: usize = 32;
const QUOTE_DECIMAL_OFF: usize = 40;
const BASE_VAULT_OFF: usize = 336;
const QUOTE_VAULT_OFF: usize = 368;
const BASE_MINT_OFF: usize = 400;
const QUOTE_MINT_OFF: usize = 432;
// "Fees" block of LiquidityStateV4: swap fee = numerator / denominator.
const SWAP_FEE_NUMERATOR_OFF: usize = 176;
const SWAP_FEE_DENOMINATOR_OFF: usize = 184;
const LIQUIDITY_STATE_SIZE: usize = 752;

// SPL Token account layout: `amount` is an LE u64 at offset 64.
const TOKEN_ACCOUNT_AMOUNT_OFF: usize = 64;

// --- Orca Whirlpool account layout (653 bytes incl. 8-byte discriminator). ---
// Offsets from programs/whirlpool/src/state/whirlpool.rs data_layout_tests.
const WHIRLPOOL_LEN: usize = 653;
const WHIRLPOOL_FEE_RATE_OFF: usize = 45;
const WHIRLPOOL_LIQUIDITY_OFF: usize = 49;
const WHIRLPOOL_SQRT_PRICE_OFF: usize = 65;
const WHIRLPOOL_MINT_A_OFF: usize = 101;
const WHIRLPOOL_VAULT_A_OFF: usize = 133;
const WHIRLPOOL_MINT_B_OFF: usize = 181;
const WHIRLPOOL_VAULT_B_OFF: usize = 213;

pub struct LiquidityState {
    pub base_decimal: u64,
    pub quote_decimal: u64,
    pub base_mint: [u8; 32],
    pub quote_mint: [u8; 32],
    pub base_vault: [u8; 32],
    pub quote_vault: [u8; 32],
    /// Swap fee in basis points, read from the on-chain fee fields.
    pub fee_bps: u16,
}

pub fn parse_liquidity_state(data: &[u8]) -> Result<LiquidityState> {
    if data.len() < LIQUIDITY_STATE_SIZE {
        bail!(
            "raydium account too short: {} bytes (need {LIQUIDITY_STATE_SIZE})",
            data.len()
        );
    }
    Ok(LiquidityState {
        base_decimal: read_u64(data, BASE_DECIMAL_OFF)?,
        quote_decimal: read_u64(data, QUOTE_DECIMAL_OFF)?,
        base_mint: read_pubkey(data, BASE_MINT_OFF)?,
        quote_mint: read_pubkey(data, QUOTE_MINT_OFF)?,
        base_vault: read_pubkey(data, BASE_VAULT_OFF)?,
        quote_vault: read_pubkey(data, QUOTE_VAULT_OFF)?,
        fee_bps: swap_fee_bps(data)?,
    })
}

fn swap_fee_bps(data: &[u8]) -> Result<u16> {
    let numerator = read_u64(data, SWAP_FEE_NUMERATOR_OFF)?;
    let denominator = read_u64(data, SWAP_FEE_DENOMINATOR_OFF)?;
    if denominator == 0 {
        return Ok(0);
    }
    Ok(((numerator as u128 * 10_000 / denominator as u128).min(u16::MAX as u128)) as u16)
}

pub struct WhirlpoolState {
    pub mint_a: [u8; 32],
    pub vault_a: [u8; 32],
    pub mint_b: [u8; 32],
    pub vault_b: [u8; 32],
    /// Swap fee in basis points. On-chain `fee_rate` is stored in HUNDREDTHS of
    /// a basis point, so 400 on-chain means 4 bps (0.04%).
    pub fee_bps: u16,
    pub liquidity: u128,
    /// Q64.64 fixed point: sqrt(price of A in B, in raw token units).
    pub sqrt_price: u128,
}

pub fn parse_whirlpool_state(data: &[u8]) -> Result<WhirlpoolState> {
    if data.len() < WHIRLPOOL_LEN {
        bail!(
            "orca account too short: {} bytes (need {WHIRLPOOL_LEN})",
            data.len()
        );
    }
    Ok(WhirlpoolState {
        mint_a: read_pubkey(data, WHIRLPOOL_MINT_A_OFF)?,
        vault_a: read_pubkey(data, WHIRLPOOL_VAULT_A_OFF)?,
        mint_b: read_pubkey(data, WHIRLPOOL_MINT_B_OFF)?,
        vault_b: read_pubkey(data, WHIRLPOOL_VAULT_B_OFF)?,
        fee_bps: read_u16(data, WHIRLPOOL_FEE_RATE_OFF)? / 100,
        liquidity: read_u128(data, WHIRLPOOL_LIQUIDITY_OFF)?,
        sqrt_price: read_u128(data, WHIRLPOOL_SQRT_PRICE_OFF)?,
    })
}

/// Concentrated liquidity (Orca) stores `liquidity` + `sqrt_price`, not plain
/// reserves. The virtual constant-product curve is:
///   amount_a = liquidity / sqrt_price     (in raw units of mint A)
///   amount_b = liquidity * sqrt_price     (in raw units of mint B)
/// so `amount_a * amount_b = liquidity^2` and the scanner's x*y=k math still
/// applies at the current price. Real CL is deeper right around the current
/// price than this approximation, but the spot rate is exactly right.
fn virtual_reserves(liquidity: u128, sqrt_price: u128) -> Result<(u64, u64)> {
    if sqrt_price == 0 {
        bail!("orca pool sqrt_price is zero");
    }
    let scale = 1u128 << 64;
    // liquidity * 2^64 may overflow u128 for absurd liquidity; saturate instead.
    let amount_a = match liquidity.checked_shl(64) {
        Some(l) => l / sqrt_price,
        None => u128::MAX / sqrt_price,
    };
    let amount_b = match liquidity.checked_mul(sqrt_price) {
        Some(p) => p / scale,
        None => u128::MAX / scale,
    };
    let clamp = |v: u128| v.min(u64::MAX as u128) as u64;
    Ok((clamp(amount_a), clamp(amount_b)))
}

pub fn parse_vault_amount(data: &[u8]) -> Result<u64> {
    read_u64(data, TOKEN_ACCOUNT_AMOUNT_OFF)
}

fn read_u16(data: &[u8], off: usize) -> Result<u16> {
    data.get(off..off + 2)
        .and_then(|s| s.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| anyhow::anyhow!("short read for u16 at offset {off}"))
}

fn read_u64(data: &[u8], off: usize) -> Result<u64> {
    data.get(off..off + 8)
        .and_then(|s| s.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| anyhow::anyhow!("short read for u64 at offset {off}"))
}

fn read_u128(data: &[u8], off: usize) -> Result<u128> {
    data.get(off..off + 16)
        .and_then(|s| s.try_into().ok())
        .map(u128::from_le_bytes)
        .ok_or_else(|| anyhow::anyhow!("short read for u128 at offset {off}"))
}

fn read_pubkey(data: &[u8], off: usize) -> Result<[u8; 32]> {
    data.get(off..off + 32)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| anyhow::anyhow!("short read for pubkey at offset {off}"))
}

// Solana JSON-RPC response shape (we only deserialize the fields we use).
#[derive(serde::Deserialize)]
struct AccountValue {
    data: Option<Vec<String>>,
}

struct PoolRef {
    pool_idx: usize,
    address: String,
}

pub struct OnchainSource {
    http: reqwest::Client,
    rpc_url: String,
    pool_refs: Vec<PoolRef>,
    not_found: Mutex<std::collections::HashSet<String>>,
}

impl OnchainSource {
    pub fn new(cfg: &MarketConfig) -> Result<Self> {
        let mut pool_refs = Vec::new();
        for (idx, pool) in cfg.pools.iter().enumerate() {
            if let Some(addr) = &pool.address {
                pool_refs.push(PoolRef {
                    pool_idx: idx,
                    address: addr.clone(),
                });
            }
        }
        Ok(Self {
            http: reqwest::Client::new(),
            rpc_url: cfg.onchain.rpc_url.clone(),
            pool_refs,
            not_found: Mutex::new(std::collections::HashSet::new()),
        })
    }

    fn log_skip(&self, addr: &str, msg: &str) {
        let mut cache = self.not_found.lock().unwrap();
        if cache.insert(addr.to_string()) {
            eprintln!("pool {addr}: {msg}");
        }
    }

    pub async fn fetch_pools(&self, cfg: &MarketConfig) -> Result<Vec<Pool>> {
        // Phase 1: fetch every pool's state account (one batched call), then
        // parse per-DEX. Raydium reserves come from its vault balances; Orca's
        // come from liquidity + sqrt_price, so no vault reads are needed there.
        let state_addrs: Vec<String> = self.pool_refs.iter().map(|p| p.address.clone()).collect();
        let state_data = self.get_multiple_accounts(&state_addrs).await?;

        let mut pools = cfg.pools.clone();
        let mut layout = Vec::with_capacity(self.pool_refs.len()); // (pool_idx, base_is_token_a)
        let mut vault_addrs: Vec<String> = Vec::new();
        let mut vault_targets = Vec::new(); // (vault_index_in_cfg, base_or_quote)
        let mut found = 0usize;

        for (i, pref) in self.pool_refs.iter().enumerate() {
            let Some(data) = state_data
                .get(i)
                .and_then(|opt| opt.as_deref())
            else {
                self.log_skip(&pref.address, "not found on-chain, skipping");
                continue;
            };
            found += 1;
            let dex = pools[pref.pool_idx].dex;

            match dex {
                Dex::Raydium => {
                    let state = match parse_liquidity_state(data) {
                        Ok(s) => s,
                        Err(err) => {
                            self.log_skip(&pref.address, &format!("skipping raydium pool: {err}"));
                            continue;
                        }
                    };

                    let pool = &pools[pref.pool_idx];
                    let base_sym = mint_to_symbol(cfg, &state.base_mint);
                    let quote_sym = mint_to_symbol(cfg, &state.quote_mint);
                    let base_is_a = base_sym.as_deref() == Some(pool.token_a.as_str())
                        && quote_sym.as_deref() == Some(pool.token_b.as_str());
                    let base_is_b = base_sym.as_deref() == Some(pool.token_b.as_str())
                        && quote_sym.as_deref() == Some(pool.token_a.as_str());
                    if !base_is_a && !base_is_b {
                        self.log_skip(&pref.address, &format!("on-chain mints don't match config {}/{}", pool.token_a, pool.token_b));
                        continue;
                    }

                    if state.fee_bps > 0 {
                        pools[pref.pool_idx].fee_bps = state.fee_bps;
                    }

                    layout.push((pref.pool_idx, base_is_a));
                    vault_targets.push((pref.pool_idx, false));
                    vault_targets.push((pref.pool_idx, true));
                    vault_addrs.push(bs58::encode(state.base_vault).into_string());
                    vault_addrs.push(bs58::encode(state.quote_vault).into_string());
                }
                Dex::Orca => {
                    let state = match parse_whirlpool_state(data) {
                        Ok(s) => s,
                        Err(err) => {
                            self.log_skip(&pref.address, &format!("skipping orca pool: {err}"));
                            continue;
                        }
                    };

                    let pool = &pools[pref.pool_idx];
                    let a_sym = mint_to_symbol(cfg, &state.mint_a);
                    let b_sym = mint_to_symbol(cfg, &state.mint_b);
                    let a_is_a = a_sym.as_deref() == Some(pool.token_a.as_str())
                        && b_sym.as_deref() == Some(pool.token_b.as_str());
                    let a_is_b = a_sym.as_deref() == Some(pool.token_b.as_str())
                        && b_sym.as_deref() == Some(pool.token_a.as_str());
                    if !a_is_a && !a_is_b {
                        self.log_skip(&pref.address, &format!("on-chain mints don't match config {}/{}", pool.token_a, pool.token_b));
                        continue;
                    }

                    if state.fee_bps > 0 {
                        pools[pref.pool_idx].fee_bps = state.fee_bps;
                    }

                    let (reserve_a, reserve_b) = match virtual_reserves(state.liquidity, state.sqrt_price) {
                        Ok(r) => r,
                        Err(err) => {
                            self.log_skip(&pref.address, &format!("orca virtual reserves: {err}"));
                            continue;
                        }
                    };
                    if a_is_a {
                        pools[pref.pool_idx].reserve_a = reserve_a;
                        pools[pref.pool_idx].reserve_b = reserve_b;
                    } else {
                        pools[pref.pool_idx].reserve_a = reserve_b;
                        pools[pref.pool_idx].reserve_b = reserve_a;
                    }
                }
            }
        }

        // Phase 2: fetch Raydium vault balances (one batched call), then assign.
        if !vault_addrs.is_empty() {
            let vault_data = self.get_multiple_accounts(&vault_addrs).await?;
            let mut base_units = vec![0u64; self.pool_refs.len()];
            let mut quote_units = vec![0u64; self.pool_refs.len()];
            let mut vaults_ok = vec![true; self.pool_refs.len()];
            for (k, (pool_idx, is_quote)) in vault_targets.iter().enumerate() {
                let Some(data) = vault_data
                    .get(k)
                    .and_then(|opt| opt.as_deref())
                 else {
                    self.log_skip(&vault_addrs[k], &format!("vault not found, skipping pool {pool_idx}"));
                    vaults_ok[*pool_idx] = false;
                    continue;
                };
                let Ok(amount) = parse_vault_amount(data) else {
                    self.log_skip(&vault_addrs[k], &format!("vault parse failed, skipping pool {pool_idx}"));
                    vaults_ok[*pool_idx] = false;
                    continue;
                };
                if *is_quote {
                    quote_units[*pool_idx] = amount;
                } else {
                    base_units[*pool_idx] = amount;
                }
            }

            for (pool_idx, base_is_a) in layout {
                if !vaults_ok[pool_idx] {
                    self.log_skip(cfg.pools[pool_idx].address.as_deref().unwrap_or("unknown"), &format!("pool {pool_idx} skipped: incomplete vault data"));
                    continue;
                }
                let pool = &mut pools[pool_idx];
                if base_is_a {
                    pool.reserve_a = base_units[pool_idx];
                    pool.reserve_b = quote_units[pool_idx];
                } else {
                    pool.reserve_a = quote_units[pool_idx];
                    pool.reserve_b = base_units[pool_idx];
                }
            }
        }
        if found < self.pool_refs.len() {
            eprintln!(
                "onchain: {found}/{} pools fetched ({} skipped)",
                self.pool_refs.len(),
                self.pool_refs.len() - found
            );
        }
        Ok(pools)
    }

    async fn get_multiple_accounts(&self, addresses: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getMultipleAccounts",
            "params": [addresses, { "encoding": "base64" }],
        });
        let http = self.http.clone();
        let rpc_url = self.rpc_url.clone();
        let body = body.clone();

        retry::with_backoff(
            move || {
                let http = http.clone();
                let rpc_url = rpc_url.clone();
                let body = body.clone();
                async move {
                    let resp = http.post(&rpc_url).json(&body).send().await?;
                    if !resp.status().is_success() {
                        bail!("rpc returned {}", resp.status());
                    }
                    #[derive(serde::Deserialize)]
                    struct BatchResponse {
                        result: BatchResult,
                    }
                    #[derive(serde::Deserialize)]
                    struct BatchResult {
                        value: Vec<Option<AccountValue>>,
                    }
                    let batch: BatchResponse = resp.json().await?;
                    let mut out = Vec::with_capacity(batch.result.value.len());
                    for account in batch.result.value {
                        let data = match account {
                            Some(a) => a
                                .data
                                .and_then(|d| d.first().cloned())
                                .map(|b64| STANDARD.decode(b64).map_err(anyhow::Error::from))
                                .transpose()?,
                            None => None,
                        };
                        out.push(data);
                    }
                    Ok(out)
                }
            },
            3,
            Duration::from_millis(500),
            |err| err.to_string().contains("rpc returned 429") || err.to_string().contains("rpc returned 5"),
        )
        .await
    }

    /// The latest confirmed slot. Used to fire scans on NEW blocks rather than
    /// on a blind timer (C4 lesson: chain-driven refresh, slot-aware).
    pub async fn get_slot(&self) -> Result<u64> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot",
            "params": [],
        });
        let http = self.http.clone();
        let rpc_url = self.rpc_url.clone();
        let body = body.clone();

        retry::with_backoff(
            move || {
                let http = http.clone();
                let rpc_url = rpc_url.clone();
                let body = body.clone();
                async move {
                    let resp = http.post(&rpc_url).json(&body).send().await?;
                    if !resp.status().is_success() {
                        bail!("rpc returned {}", resp.status());
                    }
                    #[derive(serde::Deserialize)]
                    struct SlotResponse {
                        result: u64,
                    }
                    let resp: SlotResponse = resp.json().await?;
                    Ok(resp.result)
                }
            },
            3,
            Duration::from_millis(500),
            |err| err.to_string().contains("rpc returned 429") || err.to_string().contains("rpc returned 5"),
        )
        .await
    }
}

fn mint_to_symbol<'a>(cfg: &'a MarketConfig, mint: &[u8; 32]) -> Option<&'a str> {
    let addr = bs58::encode(mint).into_string();
    cfg.tokens
        .iter()
        .find(|t| t.mint.as_deref() == Some(addr.as_str()))
        .map(|t| t.symbol.as_str())
}

// Derive a token-price map from live pool reserves by propagating from the
// base token (price 1.0) across each pool's spot rate, same propagation
// approach as the simulator's initial prices. Returns prices sorted by symbol.
pub fn prices_from_pools(cfg: &MarketConfig) -> Vec<(String, f64)> {
    let decimals: std::collections::HashMap<String, i32> = cfg
        .tokens
        .iter()
        .map(|t| (t.symbol.clone(), t.decimals as i32))
        .collect();
    let mut prices: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    prices.insert(cfg.scanner.base_token.clone(), 1.0);

    loop {
        let before = prices.len();
        for pool in &cfg.pools {
            let dec = |sym: &str| decimals[sym];
            match (
                prices.get(&pool.token_a).copied(),
                prices.get(&pool.token_b).copied(),
            ) {
                (Some(pa), None) => {
                    let pb = pa
                        * 10f64.powi(dec(&pool.token_b) - dec(&pool.token_a))
                        * pool.reserve_a as f64
                        / pool.reserve_b as f64;
                    prices.insert(pool.token_b.clone(), pb);
                }
                (None, Some(pb)) => {
                    let pa = pb
                        * 10f64.powi(dec(&pool.token_a) - dec(&pool.token_b))
                        * pool.reserve_b as f64
                        / pool.reserve_a as f64;
                    prices.insert(pool.token_a.clone(), pa);
                }
                _ => {}
            }
        }
        if prices.len() == before {
            break;
        }
    }

    let mut out: Vec<(String, f64)> = prices.into_iter().collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Should a scan fire this pass? Slot-aware: yes only when the chain has moved
/// to a NEW slot (or we're in timer-only mode with no slot data), AND the
/// minimum interval floor has passed. `None` slot = timer mode (blind refresh).
fn should_scan(
    last_slot: Option<u64>,
    slot: Option<u64>,
    elapsed_ms: u128,
    min_interval_ms: u128,
) -> bool {
    match (last_slot, slot) {
        (Some(prev), Some(cur)) => cur != prev && elapsed_ms >= min_interval_ms,
        (None, Some(_)) | (_, None) => elapsed_ms >= min_interval_ms,
    }
}

pub async fn run(
    mut cfg: MarketConfig,
    tx: broadcast::Sender<ScannerEvent>,
    max_ticks: Option<u64>,
) -> Result<()> {
    let source = OnchainSource::new(&cfg)?;
    let slot_poll = Duration::from_millis(cfg.onchain.slot_poll_ms);
    let min_interval = Duration::from_millis(cfg.onchain.refresh_ms);
    let mut tick = 0u64;
    let mut last_scan = std::time::Instant::now() - min_interval; // scan immediately
    let mut last_slot = None;

    loop {
        if let Some(max) = max_ticks {
            if tick >= max {
                return Ok(());
            }
        }

        let slot = if cfg.onchain.slot_polling {
            match source.get_slot().await {
                Ok(s) => Some(s),
                Err(err) => {
                    // RPC hiccup: fall back to the timer for this pass.
                    eprintln!("slot poll error: {err}");
                    None
                }
            }
        } else {
            None
        };

        if should_scan(last_slot, slot, last_scan.elapsed().as_millis(), min_interval.as_millis()) {
            last_slot = slot;
            last_scan = std::time::Instant::now();
            tick += 1;

            let event = match source.fetch_pools(&cfg).await {
                Ok(pools) => {
                    cfg.pools = pools;
                    ScannerEvent {
                        tick,
                        prices: prices_from_pools(&cfg),
                        opportunities: scan(&cfg),
                        slot,
                        is_simulated: false,
                        quoted_at: None,
                        stale: false,
                    }
                }
                Err(err) => {
                    eprintln!("onchain scan error: {err}");
                    continue;
                }
            };
            tx.send(event)?;
        }

        tokio::time::sleep(slot_poll).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pubkey(str: &str) -> [u8; 32] {
        bs58::decode(str).into_vec().unwrap().try_into().unwrap()
    }

    fn pool_account_fixture() -> Vec<u8> {
        let mut data = vec![0u8; LIQUIDITY_STATE_SIZE];
        data[BASE_DECIMAL_OFF..BASE_DECIMAL_OFF + 8].copy_from_slice(&9u64.to_le_bytes());
        data[QUOTE_DECIMAL_OFF..QUOTE_DECIMAL_OFF + 8].copy_from_slice(&6u64.to_le_bytes());
        let base_mint = pubkey("So11111111111111111111111111111111111111112");
        let quote_mint = pubkey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        let base_vault = pubkey("DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSb9JtteuvPpz");
        let quote_vault = pubkey("HLmqeL62xR1QoZ1HKKbXRrdN1p3phKpxRMb2VVopvBBz");
        data[BASE_MINT_OFF..BASE_MINT_OFF + 32].copy_from_slice(&base_mint);
        data[QUOTE_MINT_OFF..QUOTE_MINT_OFF + 32].copy_from_slice(&quote_mint);
        data[BASE_VAULT_OFF..BASE_VAULT_OFF + 32].copy_from_slice(&base_vault);
        data[QUOTE_VAULT_OFF..QUOTE_VAULT_OFF + 32].copy_from_slice(&quote_vault);
        data
    }

    #[test]
    fn parses_liquidity_state_layout() {
        let state = parse_liquidity_state(&pool_account_fixture()).unwrap();
        assert_eq!(state.base_decimal, 9);
        assert_eq!(state.quote_decimal, 6);
        assert_eq!(
            bs58::encode(state.base_mint).into_string(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            bs58::encode(state.quote_mint).into_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
    }

    #[test]
    fn rejects_short_account() {
        assert!(parse_liquidity_state(&[0u8; 100]).is_err());
    }

    #[test]
    fn rejects_corrupted_liquidity_state() {
        let data = vec![0u8; LIQUIDITY_STATE_SIZE];
        // Write garbage into the base_mint offset (not a valid pubkey length issue,
        // but random bytes should still parse without panic — we just check it
        // returns Err for a clearly truncated read).
        assert!(parse_liquidity_state(&data[..10]).is_err());
    }

    #[test]
    fn rejects_corrupted_whirlpool_state() {
        assert!(parse_whirlpool_state(&[0u8; 100]).is_err());
    }

    #[test]
    fn rejects_short_vault_data() {
        assert!(parse_vault_amount(&[0u8; 60]).is_err());
    }

    #[test]
    fn vault_parse_errors_are_not_panics() {
        // Even with exactly TOKEN_ACCOUNT_AMOUNT_OFF + 8 bytes, if the slice is
        // somehow malformed, read_u64 should return Err not panic.
        let data = vec![0u8; TOKEN_ACCOUNT_AMOUNT_OFF + 8];
        assert!(parse_vault_amount(&data).is_ok());
    }

    #[test]
    fn parses_vault_amount_at_offset_64() {
        let mut data = vec![0u8; 165];
        data[64..72].copy_from_slice(&1_234_567_890u64.to_le_bytes());
        assert_eq!(parse_vault_amount(&data).unwrap(), 1_234_567_890);
        assert!(parse_vault_amount(&[0u8; 60]).is_err());
    }

    fn whirlpool_fixture() -> Vec<u8> {
        let mut data = vec![0u8; WHIRLPOOL_LEN];
        data[0..8].copy_from_slice(&[0x3f, 0x95, 0xd1, 0x0c, 0xe1, 0x80, 0x63, 0x09]);
        data[WHIRLPOOL_FEE_RATE_OFF..WHIRLPOOL_FEE_RATE_OFF + 2]
            .copy_from_slice(&400u16.to_le_bytes());
        data[WHIRLPOOL_LIQUIDITY_OFF..WHIRLPOOL_LIQUIDITY_OFF + 16]
            .copy_from_slice(&1_272_258_878_937_910u128.to_le_bytes());
        data[WHIRLPOOL_SQRT_PRICE_OFF..WHIRLPOOL_SQRT_PRICE_OFF + 16]
            .copy_from_slice(&5_005_843_578_419_737_334u128.to_le_bytes());
        data[WHIRLPOOL_MINT_A_OFF..WHIRLPOOL_MINT_A_OFF + 32]
            .copy_from_slice(&pubkey("So11111111111111111111111111111111111111112"));
        data[WHIRLPOOL_MINT_B_OFF..WHIRLPOOL_MINT_B_OFF + 32]
            .copy_from_slice(&pubkey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        data
    }

    #[test]
    fn parses_whirlpool_layout() {
        let state = parse_whirlpool_state(&whirlpool_fixture()).unwrap();
        assert_eq!(state.fee_bps, 4, "400 hundredths of a bp = 4 bps");
        assert_eq!(state.liquidity, 1_272_258_878_937_910);
        assert_eq!(state.sqrt_price, 5_005_843_578_419_737_334);
        assert_eq!(
            bs58::encode(state.mint_a).into_string(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            bs58::encode(state.mint_b).into_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
    }

    #[test]
    fn rejects_short_whirlpool() {
        assert!(parse_whirlpool_state(&[0u8; 100]).is_err());
    }

    #[test]
    fn virtual_reserves_match_spot_price() {
        let liquidity = 1_272_258_878_937_910u128;
        let sqrt_price = 5_005_843_578_419_737_334u128;
        let (ra, rb) = virtual_reserves(liquidity, sqrt_price).unwrap();

        // amount_b / amount_a must equal sqrt_price^2 (raw price of A in B).
        let sqrt_p = sqrt_price as f64 / (1u128 << 64) as f64;
        let expected_ratio = sqrt_p * sqrt_p;
        let ratio = rb as f64 / ra as f64;
        assert!(
            (ratio - expected_ratio).abs() < 1e-6,
            "ratio {ratio} vs {expected_ratio}"
        );

        // amount_a * amount_b must equal liquidity^2 (the x*y=k constant).
        let k = ra as u128 * rb as u128;
        let l2 = liquidity * liquidity;
        assert!(
            ((k as f64 - l2 as f64) / l2 as f64).abs() < 1e-6,
            "k drifted"
        );
    }

    #[test]
    fn virtual_reserves_reject_zero_price() {
        assert!(virtual_reserves(1, 0).is_err());
    }

    // Captured live from mainnet 2026-08-07 via getMultipleAccounts.
    // Whirlpool accounts are static; only liquidity/sqrt_price drift over time.
    const SOL_USDC_WHIRLPOOL_DATA_B64: &str = "P5XRDOGAYwkT5EH4ORPKaLBjT7Al/eqohzfoQRDRJV41ezN33e4czf8EAAQAkAEUBbZQLdUchQQAAAAAAAAAAAD28uBkNlR4RQAAAAAAAAAAGZr//5RUjQ8AAAAA6V1kAAAAAAAGm4hX/quBhPtof2NGGMA12sQ53BrrO1WYoPAAAAAAAchN8kM4mDvkqFswl7r0C8lXEQjSiawAs2jfF11Edc96rZWI5KLJprQAAAAAAAAAAMb6evO+2606PWXzaqvJdDGxu+TC0vbg5HymAgNFL11hFl+VcsWpaqUC3VEQVKJqbSWO98HW1sGu4SkZFNxRAjIvXbGhlWcwFwAAAAAAAAAAdQ52agAAAAAMANCv64YU2n8Zq6AtQPGMaSWF9lAg387T1eX5qcDE4Q8bkJQIzrVDfhKReyB9qZTQ6FenQB4SLAPfa/fG1/wqui6/LwKaI7GKR1R798LZ7CubYjLuw+NoR9dh+omDPGYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const SOL_USDT_WHIRLPOOL_DATA_B64: &str = "P5XRDOGAYwkT5EH4ORPKaLBjT7Al/eqohzfoQRDRJV41ezN33e4czf8CAAIAyAAUBfNsrsS0AAAAAAAAAAAAAAAKffS/UgV/RQAAAAAAAAAAIJr//0O+9gEAAAAAayYmAAAAAAAGm4hX/quBhPtof2NGGMA12sQ53BrrO1WYoPAAAAAAAZg1wH9OZNloWpRBOhpHbPoIwQTWIYGucccNR+UnvWRhwJkw4EDVVH0AAAAAAAAAAM4BDmCv7bInF71jGS9UFFo/llozu4LSxwKess4eIIJklMthC2WflKoBh1VmZ4e4IJH/3p3doAciTYfdF4M2+MbscmA2otf9EgAAAAAAAAAAZQ52agAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAui6/LwKaI7GKR1R798LZ7CubYjLuw+NoR9dh+omDPGYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    #[test]
    fn parses_real_sol_usdc_whirlpool_fixture() {
        let data = STANDARD.decode(SOL_USDC_WHIRLPOOL_DATA_B64).unwrap();
        assert_eq!(data.len(), WHIRLPOOL_LEN);
        let state = parse_whirlpool_state(&data).unwrap();
        assert_eq!(state.fee_bps, 4);
        assert_eq!(
            bs58::encode(state.mint_a).into_string(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            bs58::encode(state.mint_b).into_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
        let (sol_raw, usdc_raw) = virtual_reserves(state.liquidity, state.sqrt_price).unwrap();
        let sol = sol_raw as f64 / 1e9;
        let usdc = usdc_raw as f64 / 1e6;
        assert!(sol > 1000.0, "virtual SOL reserve too small: {sol}");
        assert!(usdc > 100_000.0, "virtual USDC reserve too small: {usdc}");
        let price = usdc / sol;
        println!("orca SOL/USDC: {sol:.0} SOL / {usdc:.0} USDC (1 SOL = {price:.2} USDC), fee {} bps", state.fee_bps);
        assert!((price - 73.6).abs() < 2.0, "price {price}");
    }

    #[test]
    fn parses_real_sol_usdt_whirlpool_fixture() {
        let data = STANDARD.decode(SOL_USDT_WHIRLPOOL_DATA_B64).unwrap();
        assert_eq!(data.len(), WHIRLPOOL_LEN);
        let state = parse_whirlpool_state(&data).unwrap();
        assert_eq!(state.fee_bps, 2);
        assert_eq!(
            bs58::encode(state.mint_a).into_string(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            bs58::encode(state.mint_b).into_string(),
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
        );
        let (sol_raw, usdt_raw) = virtual_reserves(state.liquidity, state.sqrt_price).unwrap();
        let sol = sol_raw as f64 / 1e9;
        let usdt = usdt_raw as f64 / 1e6;
        let price = usdt / sol;
        println!("orca SOL/USDT: {sol:.0} SOL / {usdt:.0} USDT (1 SOL = {price:.2} USDT), fee {} bps", state.fee_bps);
        assert!((price - 73.6).abs() < 2.0, "price {price}");
    }

    // Captured live from mainnet on 2026-08-06. The pool + both vaults are
    // static addresses; only the vault AMOUNTS drift over time.
    const SOL_USDC_POOL_DATA_B64: &str = "BgAAAAAAAAD+AAAAAAAAAAcAAAAAAAAAAwAAAAAAAAAJAAAAAAAAAAYAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAEBCDwAAAAAA9AEAAAAAAAAAAAAAAAAAAEBCDwAAAAAAQEIPAAAAAAABAAAAAAAAAADKmjsAAAAAAMqaOwAAAAAFAAAAAAAAABAnAAAAAAAAGQAAAAAAAAAQJwAAAAAAAAwAAAAAAAAAZAAAAAAAAAAZAAAAAAAAABAnAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE64FyutAwAALkI4/Jo0AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACzqVos/TzgAAAAAAAAAAABzVLPxIFgFAAAAAAAAAAAAMWNjiHMDAACZN03ShGQFAAAAAAAAAAAAQEusDaXyOAAAAAAAAAAAAEiYZsIJJAAAuHDhLdN5iRVh0un6jyZDGDTrc28vJPwqKk3/H9XcpN/yy7m3YO3bGFcGMDBjrTPXtXKW6gLU4DNeMc6vpMxC3QabiFf+q4GE+2h/Y0YYwDXaxDncGus7VZig8AAAAAABxvp6877brTo9ZfNqq8l0MbG75MLS9uDkfKYCA0UvXWFsT5PYWOiP+v6gjENnRJfo5qkywMgxSCYqGuPMx4KexvkvOQ/5YJ6K1De7jkwfGqQ6wF0kMIzKd96FEsVQkpLTasTDzvqfGb9UyNwPXk0c7uUyfSZIKynSsTy6pDRHIY0NB1GoKC2mEwX+KZw3uZjlhHHbETUDcxD4vhBFpgr27qvkPHweIeqm+XyL01XiG9EnlnR1bByOEGxucSuhFtlwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOW2K2XLO72m9WiI5m/ujmTcVWAZnA+IsR/ic70FnoqhilHRXBAyAABO2XAAAAAAAPUDAAAAAAAAAAAAAAAAAAA=";
    const SOL_USDC_BASE_VAULT_DATA_B64: &str = "BpuIV/6rgYT7aH9jRhjANdrEOdwa6ztVmKDwAAAAAAFBV7BYDzHF/ORKYlgtvPnXjudZQ6CEo5OzUDaNIomTCM7/J46JPQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQEAAADwHR8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const SOL_USDC_QUOTE_VAULT_DATA_B64: &str = "xvp6877brTo9ZfNqq8l0MbG75MLS9uDkfKYCA0UvXWFBV7BYDzHF/ORKYlgtvPnXjudZQ6CEo5OzUDaNIomTCPepVEt9BAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    #[test]
    fn parses_real_sol_usdc_pool_fixture() {
        let data = STANDARD.decode(SOL_USDC_POOL_DATA_B64).unwrap();
        let state = parse_liquidity_state(&data).unwrap();
        assert_eq!(
            bs58::encode(state.base_mint).into_string(),
            "So11111111111111111111111111111111111111112"
        );
        assert_eq!(
            bs58::encode(state.quote_mint).into_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );

        let base = parse_vault_amount(&STANDARD.decode(SOL_USDC_BASE_VAULT_DATA_B64).unwrap())
            .unwrap();
        let quote = parse_vault_amount(&STANDARD.decode(SOL_USDC_QUOTE_VAULT_DATA_B64).unwrap())
            .unwrap();
        let sol = base as f64 / 1e9;
        let usdc = quote as f64 / 1e6;
        assert!(sol > 1000.0, "SOL reserve looks wrong: {sol}");
        assert!(usdc > 100_000.0, "USDC reserve looks wrong: {usdc}");
        println!("fixture: {sol:.2} SOL / {usdc:.2} USDC (1 SOL = {:.2} USDC)", usdc / sol);
        println!("fixture on-chain fee: {} bps", state.fee_bps);
    }

    #[test]
    fn should_scan_gates_on_slot_and_interval() {
        use super::should_scan;
        // Timer mode (slot data unavailable): pure interval.
        assert!(should_scan(None, None, 2000, 2000));
        assert!(!should_scan(None, None, 500, 2000));
        // Same slot, even past the floor: no scan.
        assert!(!should_scan(Some(100), Some(100), 5000, 2000));
        // New slot, past the floor: scan.
        assert!(should_scan(Some(100), Some(101), 2000, 2000));
        // New slot but inside the floor: wait.
        assert!(!should_scan(Some(100), Some(101), 500, 2000));
        // First observation of any slot: allowed once the floor has passed.
        assert!(should_scan(None, Some(100), 2000, 2000));
    }

    #[test]
    fn prices_derived_from_crafted_config() {
        let cfg = MarketConfig::from_file("../../config.toml").unwrap();
        let prices = prices_from_pools(&cfg);
        let get = |sym: &str| {
            prices
                .iter()
                .find(|(s, _)| s == sym)
                .map(|(_, p)| *p)
                .unwrap()
        };
        assert!((get("USDC") - 1.0).abs() < 1e-9);
        assert!((get("USDT") - 0.995).abs() < 0.01, "USDT={}", get("USDT"));
        assert!((get("SOL") - 100.0).abs() < 1.0, "SOL={}", get("SOL"));
        assert!(
            prices.iter().all(|(_, p)| p.is_finite() && *p > 0.0),
            "bad: {prices:?}"
        );
    }

    #[tokio::test]
    #[ignore = "requires live network access to a Solana RPC"]
    async fn real_fetch_pools() {
        let cfg = MarketConfig::from_file("../../config.toml").unwrap();
        let source = OnchainSource::new(&cfg).unwrap();
        let pools = source.fetch_pools(&cfg).await.unwrap();

        let cfg_pools = cfg.pools.clone();
        for (i, pool) in pools.iter().enumerate() {
            let cfg_pool = &cfg_pools[i];
            let base = pool.reserve_of(&cfg_pool.token_a).unwrap();
            let quote = pool.reserve_of(&cfg_pool.token_b).unwrap();
            println!(
                "{}/{} ({}): {} {} / {} {}",
                cfg_pool.token_a,
                cfg_pool.token_b,
                pool.dex.as_str(),
                base,
                cfg_pool.token_a,
                quote,
                cfg_pool.token_b,
            );
            assert!(pool.reserve_a > 0);
            assert!(pool.reserve_b > 0);
        }
    }
}
