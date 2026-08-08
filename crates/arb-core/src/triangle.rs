use std::collections::HashSet;

use serde::Serialize;

use crate::graph::PoolGraph;
use crate::math;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Leg {
    pub pool_idx: usize,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: u64,
    pub amount_out: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Opportunity {
    pub path: Vec<String>,
    pub legs: Vec<Leg>,
    pub start_amount: u64,
    pub end_amount: u64,
    pub profit_bps: i64,
    /// Profit of the same cycle computed with a zero fee on every leg
    /// (price impact still applies). The "raw" cross-rate edge before fees.
    pub gross_profit_bps: i64,
    /// True when `profit_bps >= min_profit_bps`, i.e. the cycle clears fees
    /// and the scanner's net-profit bar. Near-misses are `profitable == false`.
    pub profitable: bool,
}

impl Opportunity {
    pub fn profit_pct(&self) -> f64 {
        self.profit_bps as f64 / 100.0
    }
}

pub fn find_opportunities(
    graph: &PoolGraph,
    start: &str,
    start_amount: u64,
    max_len: usize,
    min_profit_bps: i64,
    report_min_gross_bps: i64,
) -> Vec<Opportunity> {
    let mut out = Vec::new();
    let mut path = vec![start.to_string()];
    let mut used_pools = HashSet::new();
    let mut used_tokens = HashSet::new();
    let mut legs = Vec::new();

    dfs(
        graph,
        start,
        &mut path,
        start_amount,
        start_amount,
        start_amount,
        max_len,
        min_profit_bps,
        report_min_gross_bps,
        &mut used_pools,
        &mut used_tokens,
        &mut legs,
        &mut out,
    );

    out
}

#[allow(clippy::too_many_arguments)]
fn dfs(
    graph: &PoolGraph,
    start: &str,
    path: &mut Vec<String>,
    amount: u64,
    gross_amount: u64,
    start_amount: u64,
    max_len: usize,
    min_profit_bps: i64,
    report_min_gross_bps: i64,
    used_pools: &mut HashSet<usize>,
    used_tokens: &mut HashSet<String>,
    legs: &mut Vec<Leg>,
    out: &mut Vec<Opportunity>,
) {
    let legs_taken = path.len() - 1;

    if legs_taken >= 2 && path.last() == Some(&start.to_string()) {
        let profit_bps = calc_profit_bps(start_amount, amount);
        let gross_profit_bps = calc_profit_bps(start_amount, gross_amount);
        let profitable = profit_bps >= min_profit_bps;
        if profitable || gross_profit_bps >= report_min_gross_bps {
            out.push(Opportunity {
                path: path.clone(),
                legs: legs.clone(),
                start_amount,
                end_amount: amount,
                profit_bps,
                gross_profit_bps,
                profitable,
            });
        }
        return;
    }

    if legs_taken == max_len {
        return;
    }

    let current = path.last().cloned().unwrap();

    for edge in graph.edges_from(&current) {
        if used_pools.contains(&edge.pool_idx) {
            continue;
        }

        // Closing leg back to the start token is always allowed. Any OTHER
        // token may be visited at most once: with used_pools alone, 4+ leg
        // cycles could "lollipop" back through an intermediate token (e.g.
        // A -> B -> A -> C -> A). Those are degenerate, so ban the revisit.
        let closing = edge.token_out == start;
        if !closing && used_tokens.contains(&edge.token_out) {
            continue;
        }

        let pool = &graph.pools[edge.pool_idx];
        let reserve_in = pool.reserve_of(&edge.token_in).unwrap();
        let reserve_out = pool.reserve_of(&edge.token_out).unwrap();

        let amount_out = math::swap_out_given_in(amount, reserve_in, reserve_out, pool.fee_bps);
        let gross_out = math::swap_out_given_in(gross_amount, reserve_in, reserve_out, 0);
        if amount_out == 0 && gross_out == 0 {
            continue;
        }

        used_pools.insert(edge.pool_idx);
        if !closing {
            used_tokens.insert(edge.token_out.clone());
        }
        path.push(edge.token_out.clone());
        legs.push(Leg {
            pool_idx: edge.pool_idx,
            token_in: edge.token_in.clone(),
            token_out: edge.token_out.clone(),
            amount_in: amount,
            amount_out,
        });

        dfs(
            graph,
            start,
            path,
            amount_out,
            gross_out,
            start_amount,
            max_len,
            min_profit_bps,
            report_min_gross_bps,
            used_pools,
            used_tokens,
            legs,
            out,
        );

        used_pools.remove(&edge.pool_idx);
        if !closing {
            used_tokens.remove(&edge.token_out);
        }
        path.pop();
        legs.pop();
    }
}

pub fn calc_profit_bps(start_amount: u64, end_amount: u64) -> i64 {
    if start_amount == 0 {
        return 0;
    }
    if end_amount >= start_amount {
        ((end_amount - start_amount) as u128 * 10_000 / start_amount as u128) as i64
    } else {
        -((start_amount - end_amount) as i128 * 10_000 / start_amount as i128) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pool;

    fn triangle_pools() -> Vec<Pool> {
        vec![
            Pool::new(
                "USDC".into(),
                "SOL".into(),
                1_000_000_000_000,
                10_000_000_000_000,
                30,
            ),
            Pool::new(
                "USDC".into(),
                "USDT".into(),
                1_000_000_000_000,
                1_005_000_000_000,
                30,
            ),
            Pool::new(
                "USDT".into(),
                "SOL".into(),
                980_000_000_000,
                10_000_000_000_000,
                30,
            ),
        ]
    }

    #[test]
    fn finds_profitable_cycle_in_crafted_market() {
        let graph = PoolGraph::from_pools(triangle_pools());
        let opps = find_opportunities(&graph, "USDC", 1_000_000_000, 3, 0, 0);

        assert!(
            !opps.is_empty(),
            "expected at least one profitable cycle, got none"
        );

        let best = &opps[0];
        assert_eq!(best.path, vec!["USDC", "USDT", "SOL", "USDC"]);
        assert!(best.end_amount > best.start_amount);
        assert!(best.profit_bps > 0);
        assert!(best.gross_profit_bps > best.profit_bps);
        assert!(best.profitable);
        println!("profit: {} bps ({}%)", best.profit_bps, best.profit_pct());
    }

    #[test]
    fn respects_min_profit_threshold() {
        let graph = PoolGraph::from_pools(triangle_pools());
        let strict = find_opportunities(&graph, "USDC", 10_000_000_000, 3, 100_000, 100_000);
        assert!(strict.is_empty());
    }

    #[test]
    fn reports_near_miss_when_gross_passes_but_net_does_not() {
        let graph = PoolGraph::from_pools(triangle_pools());
        // Net profit is ~132 bps, so demand 1000 net (blocks net) but 100 gross
        // (passes): the same cycle must come back as a near-miss.
        let opps = find_opportunities(&graph, "USDC", 1_000_000_000, 3, 100_000, 100);
        assert_eq!(opps.len(), 1);
        assert!(!opps[0].profitable);
        assert!(opps[0].gross_profit_bps >= 100);
        assert!(opps[0].profit_bps < 100_000);
    }

    #[test]
    fn profit_bps_math() {
        assert_eq!(calc_profit_bps(1_000, 1_100), 1_000);
        assert_eq!(calc_profit_bps(1_000, 900), -1_000);
        assert_eq!(calc_profit_bps(1_000, 1_000), 0);
    }

    /// Four pools forming a quadrilateral A -> B -> C -> D -> A. The last leg
    /// returns 1.1 A per D, so the whole cycle nets ~+10% (fees are zero so
    /// gross == net). The DFS must find this only when max_len >= 4.
    fn quad_pools() -> Vec<Pool> {
        vec![
            Pool::new("A".into(), "B".into(), 1_000_000_000, 1_000_000_000, 0),
            Pool::new("B".into(), "C".into(), 1_000_000_000, 1_000_000_000, 0),
            Pool::new("C".into(), "D".into(), 1_000_000_000, 1_000_000_000, 0),
            Pool::new("A".into(), "D".into(), 1_100_000_000, 1_000_000_000, 0),
        ]
    }

    #[test]
    fn finds_four_leg_cycle_when_allowed() {
        let graph = PoolGraph::from_pools(quad_pools());
        let opps = find_opportunities(&graph, "A", 1_000_000, 4, 0, 0);
        let quad = opps.iter().find(|o| o.path == vec!["A", "B", "C", "D", "A"]);
        let quad = quad.expect("4-leg cycle not found at max_len=4");
        assert!(quad.profitable);
        assert!(quad.profit_bps > 900, "expected ~+10%, got {}", quad.profit_bps);
    }

    #[test]
    fn four_leg_cycle_hidden_at_max_len_three() {
        let graph = PoolGraph::from_pools(quad_pools());
        let opps = find_opportunities(&graph, "A", 1_000_000, 3, 0, 0);
        assert!(
            !opps.iter().any(|o| o.path == vec!["A", "B", "C", "D", "A"]),
            "4-leg cycle leaked into max_len=3"
        );
    }

    fn assert_no_revisited_intermediates(opps: &[Opportunity]) {
        for o in opps {
            let mut seen = std::collections::HashSet::new();
            for token in &o.path[1..o.path.len() - 1] {
                assert!(
                    seen.insert(token.clone()),
                    "intermediate token {token} revisited in {:?}",
                    o.path
                );
            }
        }
    }

    #[test]
    fn four_leg_outputs_have_distinct_intermediates() {
        let graph = PoolGraph::from_pools(quad_pools());
        let opps = find_opportunities(&graph, "A", 1_000_000, 4, 0, 0);
        assert_no_revisited_intermediates(&opps);

        let cfg = crate::config::MarketConfig::from_file("../../config.toml").unwrap();
        let crafted = PoolGraph::from_pools(cfg.pools);
        let opps = find_opportunities(&crafted, &cfg.scanner.base_token, 1_000_000_000, 4, 0, 0);
        assert_no_revisited_intermediates(&opps);
    }

    /// A market whose ONLY profitable 4-leg is degenerate (A -> B -> C -> B -> A
    /// revisits B). The used_tokens guard must reject it while still allowing
    /// the legitimate 2-leg A -> B -> A through the same pools.
    #[test]
    fn degenerate_lollipop_cycles_are_blocked() {
        let pools = vec![
            Pool::new("A".into(), "B".into(), 1_000_000_000, 1_000_000_000, 0),
            Pool::new("B".into(), "C".into(), 1_000_000_000, 1_000_000_000, 0),
            // second B/C pool in the reverse direction: C -> B returns 1.1 B
            Pool::new("B".into(), "C".into(), 1_100_000_000, 1_000_000_000, 0),
            // second A/B pool: B -> A returns 1.05 A
            Pool::new("A".into(), "B".into(), 1_050_000_000, 1_000_000_000, 0),
        ];
        let graph = PoolGraph::from_pools(pools);
        let opps = find_opportunities(&graph, "A", 1_000_000, 4, 0, 0);

        assert!(
            !opps.iter().any(|o| o.path == vec!["A", "B", "C", "B", "A"]),
            "degenerate cycle revisiting B must be rejected, got {opps:?}"
        );
        assert!(
            opps.iter().any(|o| o.path == vec!["A", "B", "A"]),
            "legitimate 2-leg cycle must survive the guard"
        );
    }
}
