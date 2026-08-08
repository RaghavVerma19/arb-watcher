use anyhow::Result;
use arb_core::MarketConfig;
use arb_engine::exec::{LiveExecutor, PaperExecutor};
use arb_engine::{fmt_amount, scanner};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "config.toml".to_string());
    let ticks: u64 = args
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);

    let cfg = MarketConfig::from_file(&config_path)?;
    let base = cfg.scanner.base_token.clone();
    let tokens = cfg.tokens.clone();

    let live = LiveExecutor::new(&cfg);
    println!("=== Paper executor ===");
    println!(
        "starting capital: {}  min exec: {} bps  ticks: {ticks}",
        fmt_amount(&base, cfg.paper.starting_capital, &tokens),
        cfg.paper.min_exec_bps,
    );
    println!(
        "live execution kill switch: {}",
        if live.enabled { "ON (STUB)" } else { "OFF" }
    );
    println!();

    let (tx, mut rx) = scanner::channel(64);
    let mut paper = PaperExecutor::new(&cfg);
    let starting_capital = cfg.paper.starting_capital;
    let scanner_task = tokio::spawn(scanner::run(cfg, tx, Some(ticks)));

    while let Ok(event) = rx.recv().await {
        if let Some(trade) = paper.on_event(&event) {
            println!(
                "tick {:>4}: {:>5} bps  {} -> {}  balance {}",
                trade.tick,
                trade.profit_bps,
                fmt_amount(&base, trade.start_units, &tokens),
                fmt_amount(&base, trade.end_units, &tokens),
                fmt_amount(&base, trade.balance_units, &tokens),
            );
        }
    }

    let _ = scanner_task.await?;

    let pnl = paper.total_pnl_units();
    let pnl_sign = if pnl >= 0 { "+" } else { "-" };
    let pnl_display = fmt_amount(&base, pnl.unsigned_abs(), &tokens);
    println!();
    println!("=== Final report ===");
    println!("trades executed: {}", paper.trades.len());
    println!("wins: {}  losses: {}", paper.wins, paper.losses);
    println!(
        "start: {}  end: {}",
        fmt_amount(&base, starting_capital, &tokens),
        fmt_amount(&base, paper.capital(), &tokens),
    );
    println!(
        "realized PnL: {pnl_sign}{pnl_display} ({} bps)",
        paper.roi_bps()
    );
    Ok(())
}
