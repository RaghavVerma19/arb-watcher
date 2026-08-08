use axum::extract::State;
use axum::Json;
use serde::Serialize;

use arb_core::{Opportunity, Pool, Token};

use crate::{AppState, HistoricalOpportunity};

#[derive(Serialize)]
pub struct ExecutorResponse {
    pub capital: u64,
    pub starting_capital: u64,
    pub trades: usize,
    pub wins: u64,
    pub losses: u64,
    pub roi_bps: i64,
    pub total_pnl: i64,
}

#[derive(Serialize)]
pub struct ScannerSummary {
    pub base_token: String,
    pub base_amount: u64,
    pub min_profit_bps: i64,
    pub max_cycle_len: usize,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub mode: &'static str,
    pub tick: u64,
    pub uptime_secs: u64,
    pub scanner: ScannerSummary,
    pub tokens: Vec<Token>,
    pub pool_count: usize,
}

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let tick = state
        .latest
        .read()
        .unwrap()
        .as_ref()
        .map(|e| e.tick)
        .unwrap_or(0);

    Json(StatusResponse {
        mode: state.mode,
        tick,
        uptime_secs: state.started.elapsed().as_secs(),
        scanner: ScannerSummary {
            base_token: state.market.scanner.base_token.clone(),
            base_amount: state.market.scanner.base_amount,
            min_profit_bps: state.market.scanner.min_profit_bps,
            max_cycle_len: state.market.scanner.max_cycle_len,
        },
        tokens: state.market.tokens.clone(),
        pool_count: state.market.pools.len(),
    })
}

pub async fn opportunities(State(state): State<AppState>) -> Json<Vec<Opportunity>> {
    let opps = state
        .latest
        .read()
        .unwrap()
        .as_ref()
        .map(|e| e.opportunities.clone())
        .unwrap_or_default();
    Json(opps)
}

pub async fn pools(State(state): State<AppState>) -> Json<Vec<Pool>> {
    Json(state.market.pools.clone())
}

pub async fn history(State(state): State<AppState>) -> Json<Vec<HistoricalOpportunity>> {
    let history = state.history.read().unwrap();
    Json(history.iter().cloned().collect())
}

pub async fn executor(State(state): State<AppState>) -> Json<ExecutorResponse> {
    let exec = state.executor.read().unwrap();
    Json(ExecutorResponse {
        capital: exec.capital(),
        starting_capital: exec.starting_capital(),
        trades: exec.trades.len(),
        wins: exec.wins,
        losses: exec.losses,
        roi_bps: exec.roi_bps(),
        total_pnl: exec.total_pnl_units(),
    })
}
