use anyhow::{Context, Result};
use arb_core::{MarketConfig, Token};
use arb_engine::scanner::ScannerEvent;
use arb_engine::{fmt_amount, jupiter, onchain, scanner};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let live = args.iter().any(|a| a == "--live");
    let onchain = args.iter().any(|a| a == "--onchain");
    let ticks: Option<u64> = args
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .and_then(|s| s.parse().ok());
    let config_path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "config.toml".to_string());

    let cfg = MarketConfig::from_file(&config_path)
        .with_context(|| format!("loading config file {config_path}"))?;

    let tokens = cfg.tokens.clone();

    let mode = if onchain {
        "On-chain pool state (Raydium + Orca)"
    } else if live {
        "Live scan (Jupiter quotes)"
    } else {
        "Simulator"
    };
    println!("=== {mode} ===");
    if !onchain && !live {
        println!(
            "tick interval: {} ms, token volatility: +/-{:.2}%, pool deviation: +/-{:.2}% (revert {:.2})",
            cfg.simulator.tick_interval_ms,
            cfg.simulator.volatility * 100.0,
            cfg.simulator.pool_volatility * 100.0,
            cfg.simulator.mean_reversion,
        );
    }
    println!("config: {config_path}");
    println!(
        "scan: {} per tick, min profit {} bps, max cycle len {}",
        fmt_amount(
            &cfg.scanner.base_token,
            cfg.scanner.base_amount,
            &tokens
        ),
        cfg.scanner.min_profit_bps,
        cfg.scanner.max_cycle_len,
    );
    println!();

    let (tx, mut rx) = scanner::channel(64);

    let _task = if onchain {
        tokio::spawn(async move {
            if let Err(err) = onchain::run(cfg, tx, ticks).await {
                eprintln!("onchain scanner stopped: {err}");
            }
        })
    } else if live {
        tokio::spawn(async move {
            if let Err(err) = jupiter::run_live(cfg, tx, ticks).await {
                eprintln!("live scanner stopped: {err}");
            }
        })
    } else {
        tokio::spawn(async move {
            if let Err(err) = scanner::run(cfg, tx, ticks).await {
                eprintln!("scanner stopped: {err}");
            }
        })
    };

    loop {
        tokio::select! {
            recv = rx.recv() => {
                match recv {
                    Ok(event) => print_event(&event, &tokens),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("[consumer lagged {n} ticks]");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nshutting down...");
                break;
            }
        }
    }

    Ok(())
}

fn print_event(event: &ScannerEvent, tokens: &[Token]) {
    if event.prices.is_empty() {
        println!("tick {:>4}:", event.tick);
    } else {
        let prices = event
            .prices
            .iter()
            .map(|(s, p)| format!("{s}={p:.4}"))
            .collect::<Vec<_>>()
            .join("  ");
        println!("tick {:>4}: {prices}", event.tick);
    }

    if event.opportunities.is_empty() {
        println!("            no opportunities");
        return;
    }

    for opp in &event.opportunities {
        let start = fmt_amount(&opp.path[0], opp.start_amount, tokens);
        let end = fmt_amount(&opp.path[0], opp.end_amount, tokens);
        println!(
            "            {} bps ({}%)  {} -> {}  [{}]",
            opp.profit_bps,
            opp.profit_pct(),
            start,
            end,
            opp.path.join(" -> ")
        );
    }
}
