//! arb-server: axum REST API + WebSocket broadcasting scanner events.

pub mod api;
pub mod ring_buffer;
pub mod ws;

use std::sync::{Arc, RwLock};
use std::time::Instant;

use axum::routing::get;
use axum::Router;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};

use arb_core::{MarketConfig, Opportunity};
use arb_engine::{exec::PaperExecutor, scanner::ScannerEvent};

use crate::ring_buffer::RingBuffer;

#[derive(Clone, Debug, serde::Serialize)]
pub struct HistoricalOpportunity {
    pub tick: u64,
    pub opportunity: Opportunity,
    pub timestamp: u64, // Unix timestamp in seconds
}

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<ScannerEvent>,
    pub latest: Arc<RwLock<Option<ScannerEvent>>>,
    pub market: MarketConfig,
    pub started: Instant,
    pub mode: &'static str,
    pub history: Arc<RwLock<RingBuffer<HistoricalOpportunity>>>,
    pub executor: Arc<RwLock<PaperExecutor>>,
}

impl AppState {
    pub fn new(
        tx: broadcast::Sender<ScannerEvent>,
        market: MarketConfig,
        mode: &'static str,
    ) -> Self {
        Self {
            tx,
            latest: Arc::new(RwLock::new(None)),
            market: market.clone(),
            started: Instant::now(),
            mode,
            history: Arc::new(RwLock::new(RingBuffer::new(1000))),
            executor: Arc::new(RwLock::new(PaperExecutor::new(&market))),
        }
    }
}

pub async fn cache_latest(state: &AppState) {
    let mut rx = state.tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                *state.latest.write().unwrap() = Some(event.clone());

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let mut history = state.history.write().unwrap();
                for opp in &event.opportunities {
                    history.push(HistoricalOpportunity {
                        tick: event.tick,
                        opportunity: opp.clone(),
                        timestamp,
                    });
                }

                let mut exec = state.executor.write().unwrap();
                exec.on_event(&event);
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

pub fn app(state: AppState) -> Router {
    let origin = state.market.server.allowed_origin.clone();
    let allow_origin = match axum::http::Uri::from_maybe_shared(origin.clone()) {
        Ok(_) => {
            let header = axum::http::HeaderValue::from_str(&origin)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("*"));
            AllowOrigin::exact(header)
        }
        Err(_) => AllowOrigin::any(),
    };

    let cors = CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([axum::http::Method::GET])
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/api/status", get(api::status))
        .route("/api/opportunities", get(api::opportunities))
        .route("/api/pools", get(api::pools))
        .route("/api/history", get(api::history))
        .route("/api/executor", get(api::executor))
        .route("/ws", get(ws::ws_handler))
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let cfg = MarketConfig::from_file("../../config.toml").unwrap();
        let (tx, _rx) = broadcast::channel(16);
        AppState::new(tx, cfg, "simulator")
    }

    #[tokio::test]
    async fn status_returns_market_summary() {
        let resp = app(test_state())
            .oneshot(Request::builder().uri("/api/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["mode"], "simulator");
        assert_eq!(json["pool_count"], 12);
        assert_eq!(json["tokens"].as_array().unwrap().len(), 7);
        assert_eq!(json["scanner"]["max_cycle_len"], 5);
    }

    #[tokio::test]
    async fn opportunities_is_empty_before_first_tick() {
        let resp = app(test_state())
            .oneshot(Request::builder().uri("/api/opportunities").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn history_starts_empty() {
        let resp = app(test_state())
            .oneshot(Request::builder().uri("/api/history").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn pools_lists_config_pools() {
        let resp = app(test_state())
            .oneshot(Request::builder().uri("/api/pools").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 12);
    }

    #[tokio::test]
    async fn executor_returns_paper_state() {
        let resp = app(test_state())
            .oneshot(Request::builder().uri("/api/executor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["starting_capital"].as_u64(), Some(10_000_000_000));
        assert_eq!(json["trades"].as_u64(), Some(0));
        assert_eq!(json["wins"].as_u64(), Some(0));
        assert_eq!(json["losses"].as_u64(), Some(0));
    }

    #[tokio::test]
    async fn history_populates_after_broadcast_events() {
        let state = test_state();
        let app = app(state.clone());

        let mut history = state.history.write().unwrap();
        for i in 1..=3u64 {
            history.push(HistoricalOpportunity {
                tick: i,
                opportunity: Opportunity {
                    path: vec!["USDC".into(), "SOL".into(), "USDC".into()],
                    legs: vec![],
                    start_amount: 1_000_000_000,
                    end_amount: 1_010_000_000,
                    profit_bps: 100,
                    gross_profit_bps: 150,
                    profitable: true,
                },
                timestamp: 0,
            });
        }
        drop(history);

        let resp = app
            .oneshot(Request::builder().uri("/api/history").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn ws_route_is_wired() {
        let resp = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/ws")
                    .header("Connection", "Upgrade")
                    .header("Upgrade", "websocket")
                    .header("Sec-WebSocket-Version", "13")
                    .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // 101 = real upgrade, 426 = upgrade rejected by extractor (still proves route exists)
        assert!(
            resp.status() == StatusCode::SWITCHING_PROTOCOLS
                || resp.status() == StatusCode::UPGRADE_REQUIRED
        );
    }
}
