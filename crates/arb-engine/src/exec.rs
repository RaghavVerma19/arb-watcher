use arb_core::{MarketConfig, Opportunity};

use crate::scanner::ScannerEvent;

#[derive(Clone, Debug)]
pub struct Trade {
    pub tick: u64,
    pub path: Vec<String>,
    pub profit_bps: i64,
    pub start_units: u64,
    pub end_units: u64,
    pub profit_units: i64,
    pub balance_units: u64,
}

pub struct PaperExecutor {
    capital: u64,
    starting_capital: u64,
    min_exec_bps: i64,
    pub trades: Vec<Trade>,
    pub wins: u64,
    pub losses: u64,
}

impl PaperExecutor {
    pub fn new(cfg: &MarketConfig) -> Self {
        Self {
            capital: cfg.paper.starting_capital,
            starting_capital: cfg.paper.starting_capital,
            min_exec_bps: cfg.paper.min_exec_bps,
            trades: Vec::new(),
            wins: 0,
            losses: 0,
        }
    }

    pub fn capital(&self) -> u64 {
        self.capital
    }

    pub fn starting_capital(&self) -> u64 {
        self.starting_capital
    }

    pub fn on_event(&mut self, event: &ScannerEvent) -> Option<Trade> {
        let best = event.opportunities.iter().next()?;
        if best.profit_bps < self.min_exec_bps {
            return None;
        }
        if best.start_amount > self.capital {
            return None;
        }

        let profit_units = best.end_amount as i64 - best.start_amount as i64;
        let new_capital = (self.capital as i128 + profit_units as i128) as u64;
        self.capital = new_capital;

        if profit_units >= 0 {
            self.wins += 1;
        } else {
            self.losses += 1;
        }

        let trade = Trade {
            tick: event.tick,
            path: best.path.clone(),
            profit_bps: best.profit_bps,
            start_units: best.start_amount,
            end_units: best.end_amount,
            profit_units,
            balance_units: new_capital,
        };
        self.trades.push(trade.clone());
        Some(trade)
    }

    pub fn total_pnl_units(&self) -> i64 {
        self.capital as i64 - self.starting_capital as i64
    }

    pub fn roi_bps(&self) -> i64 {
        if self.starting_capital == 0 {
            return 0;
        }
        (self.total_pnl_units() as i128 * 10_000 / self.starting_capital as i128) as i64
    }
}

pub struct LiveExecutor {
    pub enabled: bool,
}

impl LiveExecutor {
    pub fn new(cfg: &MarketConfig) -> Self {
        Self {
            enabled: cfg.paper.live_exec,
        }
    }

    pub fn execute(&self, _opp: &Opportunity) -> anyhow::Result<()> {
        if !self.enabled {
            anyhow::bail!("live execution disabled: kill switch is OFF");
        }
        anyhow::bail!(
            "live execution is a stub: a real bot would call Jupiter /swap/v1/swap-instructions here (devnet only, never real money)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arb_core::triangle::Opportunity;

    fn cfg_with(starting_capital: u64, min_bps: i64) -> MarketConfig {
        let mut cfg = MarketConfig::from_file("../../config.toml").unwrap();
        cfg.paper.starting_capital = starting_capital;
        cfg.paper.min_exec_bps = min_bps;
        cfg
    }

    fn event(tick: u64, opp: Opportunity) -> ScannerEvent {
        ScannerEvent {
            tick,
            prices: Vec::new(),
            opportunities: vec![opp],
            slot: None,
            is_simulated: true,
            quoted_at: None,
            stale: false,
        }
    }

    fn opp(profit_bps: i64) -> Opportunity {
        let start = 1_000_000_000u64;
        let delta = (start as i128 * profit_bps as i128 / 10_000) as i64;
        Opportunity {
            path: vec!["USDC".into(), "SOL".into(), "USDC".into()],
            legs: Vec::new(),
            start_amount: start,
            end_amount: (start as i128 + delta as i128) as u64,
            profit_bps,
            gross_profit_bps: profit_bps,
            profitable: profit_bps >= 0,
        }
    }

    #[test]
    fn tracks_pnl_across_trades() {
        let cfg = cfg_with(10_000_000_000, i64::MIN);
        let mut paper = PaperExecutor::new(&cfg);

        let t1 = paper.on_event(&event(1, opp(250))).unwrap();
        assert_eq!(t1.profit_units, 25_000_000);
        assert_eq!(paper.capital(), 10_025_000_000);
        assert_eq!(paper.wins, 1);
        assert_eq!(paper.losses, 0);

        let t2 = paper.on_event(&event(2, opp(-100))).unwrap();
        assert_eq!(t2.profit_units, -10_000_000);
        assert_eq!(paper.capital(), 10_015_000_000);
        assert_eq!(paper.wins, 1);
        assert_eq!(paper.losses, 1);

        assert_eq!(paper.total_pnl_units(), 15_000_000);
        assert_eq!(paper.roi_bps(), 15);
    }

    #[test]
    fn skips_trades_below_min_bps() {
        let cfg = cfg_with(10_000_000_000, 50);
        let mut paper = PaperExecutor::new(&cfg);

        let low = paper.on_event(&event(1, opp(40)));
        assert!(low.is_none());
        assert!(paper.trades.is_empty());
        assert_eq!(paper.capital(), 10_000_000_000);
    }

    #[test]
    fn skips_trades_it_cannot_afford() {
        let cfg = cfg_with(500_000_000, 0);
        let mut paper = PaperExecutor::new(&cfg);

        let t = paper.on_event(&event(1, opp(250)));
        assert!(t.is_none());
        assert!(paper.trades.is_empty());
    }

    #[test]
    fn live_exec_kill_switch_blocks() {
        let mut cfg = cfg_with(10_000_000_000, 0);
        cfg.paper.live_exec = false;
        let off = LiveExecutor::new(&cfg);
        assert!(off.execute(&opp(250)).is_err());

        cfg.paper.live_exec = true;
        let on = LiveExecutor::new(&cfg);
        let err = on.execute(&opp(250)).unwrap_err().to_string();
        assert!(err.contains("stub"), "expected stub error, got: {err}");
    }
}
