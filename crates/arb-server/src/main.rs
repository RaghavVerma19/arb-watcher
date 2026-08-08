use anyhow::Result;
use arb_core::MarketConfig;
use arb_engine::{jupiter, onchain, scanner};
use arb_server::{app, cache_latest, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let live = args.iter().any(|a| a == "--live");
    let onchain = args.iter().any(|a| a == "--onchain");
    let port = args
        .iter()
        .find_map(|a| a.strip_prefix("--port="))
        .map(str::parse::<u16>)
        .transpose()?
        .unwrap_or(8080);
    let config_path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "config.toml".to_string());

    let mode = if onchain {
        "onchain"
    } else if live {
        "live"
    } else {
        "simulator"
    };

    let cfg = MarketConfig::from_file(&config_path)?;
    let (tx, rx) = scanner::channel(64);
    let state = AppState::new(tx.clone(), cfg.clone(), mode);

    let cache_state = state.clone();
    let scanner_handle = tokio::spawn(async move {
        let result = if onchain {
            onchain::run(cfg, tx, None).await
        } else if live {
            jupiter::run_live(cfg, tx, None).await
        } else {
            scanner::run(cfg, tx, None).await
        };
        if let Err(err) = result {
            eprintln!("scanner stopped: {err}");
        }
    });
    let cache_handle = tokio::spawn(async move { cache_latest(&cache_state).await });

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    println!(
        "arb-server listening on http://0.0.0.0:{port} ({mode})"
    );

    tokio::select! {
        res = axum::serve(listener, app(state)) => {
            if let Err(err) = res {
                eprintln!("server error: {err}");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nshutting down...");
        }
    }

    drop(rx);
    let _ = scanner_handle.await;
    let _ = cache_handle.await;
    println!("goodbye");
    Ok(())
}
