use std::time::Duration;

use serde::Serialize;
use tokio::sync::broadcast;

use arb_core::{MarketConfig, Opportunity};

use crate::scan::scan;
use crate::sim::Simulator;

#[derive(Clone, Debug, Serialize)]
pub struct ScannerEvent {
    pub tick: u64,
    pub prices: Vec<(String, f64)>,
    pub opportunities: Vec<Opportunity>,
    /// Solana slot the data was read at (onchain mode only; None otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<u64>,
    /// True when prices/opportunities come from the simulator, not live data.
    pub is_simulated: bool,
    /// Wall-clock time the quote/scan was taken (Jupiter/live mode only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quoted_at: Option<u64>,
    /// True when the scan took longer than the refresh interval and may be stale.
    pub stale: bool,
}

pub fn channel(
    capacity: usize,
) -> (
    broadcast::Sender<ScannerEvent>,
    broadcast::Receiver<ScannerEvent>,
) {
    broadcast::channel(capacity)
}

pub async fn run(
    mut cfg: MarketConfig,
    tx: broadcast::Sender<ScannerEvent>,
    max_ticks: Option<u64>,
) -> anyhow::Result<()> {
    let mut sim = Simulator::new(&cfg);
    let interval = Duration::from_millis(cfg.simulator.tick_interval_ms);

    loop {
        if let Some(max) = max_ticks {
            if sim.tick >= max {
                return Ok(());
            }
        }

        sim.step(&mut cfg);

        let event = ScannerEvent {
            tick: sim.tick,
            prices: sim.prices(),
            opportunities: scan(&cfg),
            slot: None,
            is_simulated: true,
            quoted_at: None,
            stale: false,
        };
        tx.send(event)?;

        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load() -> MarketConfig {
        MarketConfig::from_file("../../config.toml").unwrap()
    }

    #[tokio::test]
    async fn emits_three_ticks_then_stops() {
        let mut cfg = load();
        cfg.simulator.tick_interval_ms = 1;
        let (tx, mut rx) = channel(16);

        let handle = tokio::spawn(async move { run(cfg, tx, Some(3)).await });
        handle.await.unwrap().unwrap();

        let mut ticks = Vec::new();
        while let Ok(event) = rx.recv().await {
            ticks.push(event.tick);
            assert_eq!(event.prices.len(), 7);
        }
        assert_eq!(ticks, vec![1, 2, 3]);
    }
}
