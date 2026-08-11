# Solana Triangular Arbitrage Scanner

A Rust fullstack web3 project for learning async Rust, Solana DEX mechanics, constant-product AMM math, and live dashboarding. It scans Raydium + Orca pools for triangular (and N-leg) arbitrage cycles and visualizes opportunities in real time.

**Paper trading only.** No real funds. Live execution is disabled by default.

## What it does

- Builds a token graph from configured pools (Raydium AMM v4 + Orca Whirlpools)
- Runs DFS cycle detection (configurable `max_cycle_len`, default 3)
- Computes profit in basis points after per-leg fees and price impact
- Streams opportunities via WebSocket to a separate Vite + TypeScript dashboard

## Architecture

```
rust_proj/
├── Cargo.toml              # workspace root
├── config.toml             # simulator / crafted market
├── config.mainnet.toml     # real mainnet pool addresses
├── crates/
│   ├── arb-core/           # pure logic, no I/O
│   │   ├── token.rs        # Token (symbol, decimals, mint)
│   │   ├── pool.rs         # Pool (reserves, fee_bps, DEX, address)
│   │   ├── math.rs         # constant-product swaps, u128 math
│   │   ├── graph.rs        # adjacency list, directed edges
│   │   ├── triangle.rs     # DFS cycle detection
│   └── config.rs           # TOML config parsing
│   ├── arb-engine/         # scanner + data sources + paper executor
│   │   ├── sim.rs          # simulator market (random-walk prices)
│   │   ├── onchain.rs      # live pool state via Solana JSON-RPC
│   │   ├── jupiter.rs      # Jupiter v6 quote API client
│   │   ├── scan.rs         # core scan loop (graph + math)
│   │   ├── scanner.rs      # tokio broadcast channel, tick loop
│   │   ├── exec.rs         # PaperExecutor (paper trading only)
│   │   └── retry.rs        # exponential backoff + jitter
│   └── arb-server/         # axum API + WebSocket
│       ├── api.rs          # /api/status, /api/opportunities, /api/pools, /api/executor
│       ├── ws.rs           # /ws upgrade + live JSON stream
│       └── ring_buffer.rs  # fixed-capacity O(1) history
└── frontend/               # Vite + TypeScript, separate process
    ├── src/main.ts         # entry, WS + fetch wiring
    ├── src/views/          # landing, history, terminal, exec, intel, sculpture
    ├── src/style.css       # dark design system
    └── vite.config.ts      # proxies /api and /ws to backend
```

## Prerequisites

- Rust 1.75+ (edition 2021)
- Node.js 20+ and npm
- Docker + Docker Compose (for deployment)

## Run locally

### 1. Backend (simulator)

```bash
cargo run -p arb-engine
```

Starts the scanner loop with the simulator market. Opportunities appear and vanish as prices random-walk. Profit math uses `u128`; `f64` is display-only.

### 2. Backend (on-chain mainnet, paper scan)

```bash
cargo run -p arb-server -- --onchain --config config.mainnet.toml
```

Fetches real Raydium + Orca pool state via `getMultipleAccounts` (batched, no rate-limit issues). Streams `ScannerEvent` on `/ws`.

### 3. Paper trading CLI

```bash
cargo run -p arb-engine --bin paper -- --ticks=60
```

Runs the simulator for N ticks and executes the best opportunity each tick via `PaperExecutor`. Prints fills + final PnL/ROI.

### 4. Frontend dev server

```bash
cd frontend
npm install
npm run dev
```

Vite proxies `/api` → `http://127.0.0.1:8080` and `/ws` → `ws://127.0.0.1:8080`.

Open `http://localhost:5173`.

## Run tests

```bash
cargo test --workspace
```

65 passed, 3 ignored (live network + seed search), 0 warnings.

```bash
cd frontend && npm run build
```

Frontend build must also be clean.

## Docker

### Build locally

```bash
docker compose build
```

### Run

```bash
docker compose up
```

- Backend: `http://localhost:8080`
- Frontend: `http://localhost:3000` (nginx)

The scanner image mounts `config.mainnet.toml` read-only and runs in `--onchain` mode.

## Key concepts

### Constant product (`x * y = k`)

Raydium v4 and Orca Whirlpools are constant-product AMMs. A swap of `dx` for `dy`:

```
dy = (y * dx * (BPS - fee_bps)) / (x + dx)
```

`fee_bps` is in basis points (1 bp = 0.01%). 30 bps = 0.3%.

### u128 math

Reserves and amounts are `u64` base units (e.g. 1 USDC = 1_000_000 for 6 decimals). Intermediate widening uses `u128` to avoid overflow. `f64` is only used at the display boundary (profit %).

### Cycle detection

DFS over a directed token graph. Each undirected pool becomes 2 directed edges. `used_pools` prevents reusing the same pool in one cycle. `legs >= 2` ensures we close back to the start token.

### Per-pool deviation (simulator)

A pure token-price simulator can never create arb (all pools share one price). Each pool gets its own `pool_noise` that random-walks and mean-reverts. This transient mispricing is what the scanner catches.

## Safety

- `paper.live_exec = false` by default. Real swap execution is a stub that `anyhow::bail!`s.
- Defensive math clamps `fee_bps` to `BPS - 1` (9,999) to prevent division by zero.
- On-chain account parsing returns `Result`; corrupted or short data skips that pool instead of panicking.
- Retry module (`retry.rs`) wraps HTTP calls with exponential backoff + jitter.

## Config

| File | Purpose |
|------|---------|
| `config.toml` | Simulator / crafted market with a known 132 bps profitable cycle |
| `config.mainnet.toml` | Real mainnet pool addresses (verified 2026-08-08) |

Config sections:

- `[[tokens]]` — symbol, decimals, mint (base58)
- `[[pools]]` — token_a, token_b, reserves, fee_bps, dex (`raydium` / `orca`), address
- `[scanner]` — base_token, base_amount, min_profit_bps, max_cycle_len
- `[simulator]` — volatility, pool_volatility, mean_reversion, tick_interval_ms, seed
- `[onchain]` — rpc_url, refresh_ms, slot_poll_ms, slot_polling
- `[jupiter]` — base_url, refresh_ms
- `[paper]` — enabled, starting_capital, min_exec_bps, live_exec
- `[server]` — allowed_origin (CORS)

## Costs

- t3.small EC2 (Ubuntu 22.04) ≈ $15/month.
- Public Solana RPC is free but rate-limited. The scanner batches `getMultipleAccounts` to stay under limits.
- Jupiter free tier is also rate-limited (~5 requests / 10-30s). On-chain parsing avoids this.

## Learning notes

- MEV bots on Solana use Jito bundles + 1ms latency + on-chain pool parsing. This project is a **learning vehicle**, not a production arb bot.
- The simulator always "wins" because fills assume the detected price with zero latency and zero slippage — a useful teaching point, not a feature.
- Same-pair cross-DEX cycles (e.g. SOL/USDC on Raydium vs Orca) are found and honestly reported as near-misses when fees exceed the spread.
