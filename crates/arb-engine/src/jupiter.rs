use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::broadcast;

use arb_core::triangle::{calc_profit_bps, find_opportunities, Leg, Opportunity};
use arb_core::{MarketConfig, PoolGraph};

use crate::retry;
use crate::scanner::ScannerEvent;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount", deserialize_with = "de_u64")]
    pub in_amount: u64,
    #[serde(rename = "outAmount", deserialize_with = "de_u64")]
    pub out_amount: u64,
    #[serde(rename = "contextSlot", default)]
    pub context_slot: u64,
}

fn de_u64<'de, D>(de: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = u64;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a u64 or a numeric string")
        }

        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<u64, E> {
            Ok(v)
        }

        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<u64, E> {
            u64::try_from(v).map_err(|_| E::custom("negative amount"))
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<u64, E> {
            v.parse().map_err(E::custom)
        }
    }

    de.deserialize_any(Visitor)
}

#[allow(async_fn_in_trait)]
pub trait QuoteApi: Send + Sync {
    async fn quote(&self, input: &str, output: &str, amount: u64)
        -> anyhow::Result<QuoteResponse>;
}

pub struct JupiterClient {
    http: reqwest::Client,
    base_url: String,
    mints: HashMap<String, String>,
}

impl JupiterClient {
    pub fn new(cfg: &MarketConfig) -> anyhow::Result<Self> {
        let mints = cfg
            .tokens
            .iter()
            .filter_map(|t| Some((t.symbol.clone(), t.mint.clone()?)))
            .collect();
        Ok(Self {
            http: reqwest::Client::new(),
            base_url: cfg.jupiter.base_url.trim_end_matches('/').to_string(),
            mints,
        })
    }

    fn mint(&self, symbol: &str) -> anyhow::Result<&str> {
        self.mints
            .get(symbol)
            .map(String::as_str)
            .ok_or_else(|| anyhow::anyhow!("no mint configured for {symbol}"))
    }

    async fn fetch_quote(
        &self,
        input: &str,
        output: &str,
        amount: u64,
    ) -> anyhow::Result<QuoteResponse> {
        let in_mint = self.mint(input)?;
        let out_mint = self.mint(output)?;
        let url = format!(
            "{}/swap/v1/quote?inputMint={}&outputMint={}&amount={}&slippageBps=50",
            self.base_url, in_mint, out_mint, amount
        );
        let http = self.http.clone();
        let url = url.clone();

        retry::with_backoff(
            move || {
                let http = http.clone();
                let url = url.clone();
                async move {
                    let resp = http.get(&url).send().await?;
                    if !resp.status().is_success() {
                        anyhow::bail!("quote api returned {}", resp.status());
                    }
                    let quote: QuoteResponse = resp.json().await?;
                    Ok(quote)
                }
            },
            3,
            Duration::from_millis(500),
            |err| err.to_string().contains("quote api returned 429") || err.to_string().contains("quote api returned 5"),
        )
        .await
    }
}

impl QuoteApi for JupiterClient {
    async fn quote(
        &self,
        input: &str,
        output: &str,
        amount: u64,
    ) -> anyhow::Result<QuoteResponse> {
        self.fetch_quote(input, output, amount).await
    }
}

pub fn build_candidates(cfg: &MarketConfig) -> Vec<Vec<String>> {
    let graph = PoolGraph::from_pools(cfg.pools.clone());
    let all = find_opportunities(
        &graph,
        &cfg.scanner.base_token,
        cfg.scanner.base_amount,
        cfg.scanner.max_cycle_len,
        i64::MIN,
        i64::MIN,
    );

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for opp in all {
        if seen.insert(opp.path.clone()) {
            out.push(opp.path);
        }
    }
    out
}

pub async fn live_scan<Q: QuoteApi>(
    cfg: &MarketConfig,
    quoter: &Q,
    base_amount: u64,
    min_profit_bps: i64,
) -> anyhow::Result<Vec<Opportunity>> {
    let mut out = Vec::new();

    for path in build_candidates(cfg) {
        let mut amount = base_amount;
        let mut legs = Vec::new();
        let mut ok = true;

        for i in 0..path.len() - 1 {
            match quoter.quote(&path[i], &path[i + 1], amount).await {
                Ok(q) if q.out_amount > 0 => {
                    legs.push(Leg {
                        pool_idx: 0,
                        token_in: path[i].clone(),
                        token_out: path[i + 1].clone(),
                        amount_in: amount,
                        amount_out: q.out_amount,
                    });
                    amount = q.out_amount;
                }
                Ok(_) => {
                    eprintln!("quote {}-{} returned zero", path[i], path[i + 1]);
                    ok = false;
                    break;
                }
                Err(err) => {
                    eprintln!("quote {}-{} failed: {err}", path[i], path[i + 1]);
                    ok = false;
                    break;
                }
            }
        }

        if !ok {
            continue;
        }

        let profit_bps = calc_profit_bps(base_amount, amount);
        if profit_bps >= min_profit_bps {
            out.push(Opportunity {
                path,
                legs,
                start_amount: base_amount,
                end_amount: amount,
                profit_bps,
                // Jupiter quotes are already net of fees; there is no separate
                // gross chain to compute, so report them as identical.
                gross_profit_bps: profit_bps,
                profitable: profit_bps >= min_profit_bps,
            });
        }
    }

    out.sort_by(|a, b| b.profit_bps.cmp(&a.profit_bps));
    Ok(out)
}

pub async fn run_live(
    cfg: MarketConfig,
    tx: broadcast::Sender<ScannerEvent>,
    max_ticks: Option<u64>,
) -> anyhow::Result<()> {
    let client = JupiterClient::new(&cfg)?;
    let interval = Duration::from_millis(cfg.jupiter.refresh_ms);
    let base_amount = cfg.scanner.base_amount;
    let min_profit_bps = cfg.scanner.min_profit_bps;
    let mut tick = 0u64;

    loop {
        if let Some(max) = max_ticks {
            if tick >= max {
                return Ok(());
            }
        }
        tick += 1;

        let scan_start = Instant::now();
        let event = match live_scan(&cfg, &client, base_amount, min_profit_bps).await {
            Ok(opportunities) => {
                let elapsed = scan_start.elapsed();
                let stale = elapsed > interval;
                ScannerEvent {
                    tick,
                    prices: Vec::new(),
                    opportunities,
                    slot: None,
                    is_simulated: false,
                    quoted_at: Some(elapsed.as_millis() as u64),
                    stale,
                }
            }
            Err(err) => {
                eprintln!("live scan error: {err}");
                continue;
            }
        };
        tx.send(event)?;

        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "inputMint": "So11111111111111111111111111111111111111112",
        "inAmount": "1000000000",
        "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "outAmount": "99521234",
        "otherAmountThreshold": "99464000",
        "contextSlot": 123456,
        "routePlan": []
    }"#;

    fn load() -> MarketConfig {
        MarketConfig::from_file("../../config.toml").unwrap()
    }

    #[test]
    fn parses_quote_response_fixture() {
        let q: QuoteResponse = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(q.in_amount, 1_000_000_000);
        assert_eq!(q.out_amount, 99_521_234);
        assert_eq!(q.context_slot, 123_456);
    }

    #[test]
    fn candidates_cover_all_cycles() {
        let cfg = load();
        let cands = build_candidates(&cfg);
        assert!(cands.iter().any(|p| *p == vec!["USDC", "USDT", "SOL", "USDC"]));
        assert!(cands.iter().any(|p| *p == vec!["USDC", "SOL", "JUP", "USDC"]));
    }

    struct MockQuoter {
        rates: HashMap<(String, String), f64>,
    }

    impl MockQuoter {
        fn new() -> Self {
            let mut rates = HashMap::new();
            rates.insert(("USDC".into(), "USDT".into()), 1.005);
            rates.insert(("USDT".into(), "SOL".into()), 1.0 / 98.0);
            rates.insert(("SOL".into(), "USDC".into()), 100.0);
            Self { rates }
        }
    }

    impl QuoteApi for MockQuoter {
        async fn quote(
            &self,
            input: &str,
            output: &str,
            amount: u64,
        ) -> anyhow::Result<QuoteResponse> {
            let key = (input.to_string(), output.to_string());
            let rate = self
                .rates
                .get(&key)
                .ok_or_else(|| anyhow::anyhow!("no rate for {key:?}"))?;
            Ok(QuoteResponse {
                input_mint: input.to_string(),
                in_amount: amount,
                output_mint: output.to_string(),
                out_amount: (amount as f64 * rate).round() as u64,
                context_slot: 0,
            })
        }
    }

    #[tokio::test]
    async fn live_scan_finds_profitable_chain() {
        let cfg = load();
        let quoter = MockQuoter::new();
        let opps = live_scan(&cfg, &quoter, 1_000_000_000, 10).await.unwrap();

        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].path, vec!["USDC", "USDT", "SOL", "USDC"]);
        assert!(opps[0].profit_bps > 0);
    }

    #[tokio::test]
    #[ignore = "requires live network access to api.jup.ag"]
    async fn real_quote_usdc_to_sol() {
        let cfg = load();
        let client = JupiterClient::new(&cfg).unwrap();
        let q = client.quote("USDC", "SOL", 1_000_000).await.unwrap();
        assert!(q.out_amount > 0);
        println!(
            "1 USDC -> {:.6} SOL (inAmount {})",
            q.out_amount as f64 / 1e9,
            q.in_amount
        );
    }
}
