use arb_core::triangle::find_opportunities;
use arb_core::{MarketConfig, Opportunity, PoolGraph, Token};

pub fn scan(cfg: &MarketConfig) -> Vec<Opportunity> {
    let graph = PoolGraph::from_pools(cfg.pools.clone());
    let s = &cfg.scanner;

    let mut opps = find_opportunities(
        &graph,
        &s.base_token,
        s.base_amount,
        s.max_cycle_len,
        s.min_profit_bps,
        s.report_min_gross_bps,
    );

    // Net-profitable cycles first (best net at the top), then near-misses by
    // gross edge. A near-miss's net is always below the bar by construction.
    opps.sort_by(|a, b| {
        b.profitable
            .cmp(&a.profitable)
            .then_with(|| {
                let key = |o: &Opportunity| {
                    if o.profitable {
                        o.profit_bps
                    } else {
                        o.gross_profit_bps
                    }
                };
                key(b).cmp(&key(a))
            })
    });
    opps
}

pub fn fmt_amount(symbol: &str, units: u64, tokens: &[Token]) -> String {
    let Some(token) = tokens.iter().find(|t| t.symbol == symbol) else {
        return format!("{units} {symbol}");
    };
    let value = units as f64 / token.decimals_pow() as f64;
    format!("{value:.prec$} {symbol}", prec = token.decimals as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_amount_uses_token_decimals() {
        let tokens = vec![
            Token {
                symbol: "SOL".into(),
                decimals: 9,
                mint: None,
            },
            Token {
                symbol: "USDC".into(),
                decimals: 6,
                mint: None,
            },
        ];
        assert_eq!(
            fmt_amount("SOL", 1_500_000_000, &tokens),
            "1.500000000 SOL"
        );
        assert_eq!(
            fmt_amount("USDC", 1_000_000_000, &tokens),
            "1000.000000 USDC"
        );
    }

    #[test]
    fn fmt_amount_unknown_token_falls_back() {
        assert_eq!(fmt_amount("NOPE", 42, &[]), "42 NOPE");
    }

    #[test]
    fn scan_finds_crafted_profit_cycle() {
        let cfg = MarketConfig::from_file("../../config.toml").unwrap();
        let opps = scan(&cfg);
        let best = opps.first().expect("expected opportunities");

        // The crafted cycle should still be the best in the simulator config
        assert!(best.profit_bps > 0);
        println!("best: {} bps ({}%)", best.profit_bps, best.profit_pct());
    }
}
