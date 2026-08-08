//! arb-core: pure arbitrage logic. No network, no async, no I/O.
//! Everything here is a pure function of the pool state it is given.

pub mod config;
pub mod graph;
pub mod math;
pub mod pool;
pub mod token;
pub mod triangle;

pub use config::MarketConfig;
pub use graph::PoolGraph;
pub use pool::{Dex, Pool};
pub use token::Token;
pub use triangle::Opportunity;
