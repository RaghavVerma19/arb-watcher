use std::collections::HashMap;

use crate::Pool;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub pool_idx: usize,
    pub token_in: String,
    pub token_out: String,
}

pub struct PoolGraph {
    pub pools: Vec<Pool>,
    edges: HashMap<String, Vec<Edge>>,
}

impl PoolGraph {
    pub fn from_pools(pools: Vec<Pool>) -> Self {
        let mut edges: HashMap<String, Vec<Edge>> = HashMap::new();

        for (pool_idx, pool) in pools.iter().enumerate() {
            let a_to_b = Edge {
                pool_idx,
                token_in: pool.token_a.clone(),
                token_out: pool.token_b.clone(),
            };
            let b_to_a = Edge {
                pool_idx,
                token_in: pool.token_b.clone(),
                token_out: pool.token_a.clone(),
            };
            edges
                .entry(pool.token_a.clone())
                .or_default()
                .push(a_to_b);
            edges
                .entry(pool.token_b.clone())
                .or_default()
                .push(b_to_a);
        }

        Self { pools, edges }
    }

    pub fn edges_from(&self, symbol: &str) -> Vec<&Edge> {
        self.edges.get(symbol).map(|v| v.iter().collect()).unwrap_or_default()
    }

    pub fn all_symbols(&self) -> Vec<&str> {
        self.edges.keys().map(String::as_str).collect()
    }
}
