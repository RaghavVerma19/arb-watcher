# Solana Triangular Arbitrage Scanner — Project Context

## Goal
Rust fullstack web3 project for learning: scan Solana DEXs for triangular (and
general N-leg) arbitrage cycles, visualize live opportunities on a web dashboard.

## Learner profile & working style
- Learning web3 + Rust; decent but not advanced Rust.
- PRIMARY OBJECTIVE IS LEARNING, not shipping. The project is scaffolding for
  understanding async Rust, web3, Rust backends, and concurrency.
- Guide me through EVERY file, EVERY dependency, and EVERY line of code.
- Explain concepts in plain terms at the point they're used.
- Work phase-by-phase; confirm before starting each phase.
- Use todo lists to track progress.
- Teaching mode rules (agreed with user):
  - Explanations live in chat ONLY; code stays clean, no teaching comments.
  - Pause after each concept for a CHECKPOINT question; wait for the answer.
  - Ask missed checkpoints again if the learner skips them.

## Decisions made (from Q&A)
- Arb type: triangular arbitrage first, generalized to N-leg cycles (max_cycle_len).
- Chain: Solana.
- Fullstack: backend + web dashboard.
- Env: simulator + testnet first. NEVER real money. Paper-trade only.
- Frontend: SEPARATE app, Vite + TypeScript (own package.json, own dev server,
  proxies /api and /ws to backend in dev).
- Backend: axum pure API server, CORS enabled, no static files.

## Reality check (keep in mind)
- Live profitable arb on Solana mainnet is contested by MEV bots (Jito bundles,
  ~1ms latency, expensive infra). This project is a LEARNING VEHICLE, not a
  money printer. Default stays paper/simulator.

## Stack & dependencies (explained)
Phase 1 (arb-core):
- serde (derive): auto-convert structs <-> JSON/TOML via #[derive(Serialize, Deserialize)]
- serde_json: the JSON dialect for serde
- toml: the .toml dialect for serde (config file parsing)
- anyhow: simple error handling; anyhow::Result<T> = "T or some error, whatever it is"
- rand: random numbers (Phase 3 simulator only)

Later phases:
- tokio: async runtime (scanner + web server concurrently)
- axum 0.8: web framework (REST routes + WebSocket)
- reqwest 0.12: async HTTP client (Jupiter quote API + raw Solana JSON-RPC)
- bs58 0.5 + base64 0.22: pubkey base58 encode/decode + account data base64
- tower-http: middleware (CORS, static files)
- tracing: structured logging

## Architecture (cargo workspace, 3 crates + separate frontend)
rust_proj/
├── Cargo.toml            # [workspace]
├── config.toml           # tokens, pools, thresholds, RPC/Jupiter URLs
├── crates/
│   ├── arb-core/         # pure logic, no I/O: types, x*y=k math, graph, triangles
│   ├── arb-engine/       # simulator, Jupiter client, scanner loop, paper executor
│   └── arb-server/       # axum API: /api/status, /api/opportunities, /api/pools, /ws
└── frontend/             # Vite + TS, separate process

## Key design decisions
- Amounts: u64 base units + per-token decimals; math in u128; f64 only for display %.
- Cycle detection: DFS over token graph, configurable max_cycle_len (start at 3).
- Per-leg fees + price impact accounted in profit calc.
- Jupiter quote API is mainnet-only in practice (devnet returns "no route").

## Build phases
1. Foundation: workspace, .gitignore, config.toml, arb-core (token.rs, pool.rs,
   math.rs, graph.rs, triangle.rs, lib.rs) + unit tests. cargo test green.
2. Triangle detection: verify with hand-crafted profitable pools (known answers).
3. Simulator market: random-walk prices, scanner task -> broadcast channel, CLI mode.
4. Live data: Jupiter v6 quote client, 3-leg sequential quotes.
   Optional 3b: parse real Raydium/Orca pool accounts via solana-client.
5. Backend: axum REST + /ws WebSocket, CORS.
6. Frontend: Vite + TS dashboard (triangle table, status, profit chart, WS updates).
7. Execution (opt-in): paper-trade first; real tx via Jupiter swap-instructions,
   devnet only, kill switch.

## Progress log
- [x] Phase 1 DONE (cargo test: 10 passed, 0 warnings). Files created:
  - root Cargo.toml ([workspace], resolver 2, members, [workspace.dependencies])
  - .gitignore (target/, frontend/node_modules, frontend/dist, .env)
  - config.toml: 5 tokens (SOL, USDC, USDT, JUP, RAY), 6 pools, [scanner]
  - arb-core: Cargo.toml, lib.rs, token.rs, pool.rs, math.rs, graph.rs,
    triangle.rs, config.rs
  - arb-engine + arb-server: stub lib.rs only
  - Key learned lesson: position size vs pool depth. 10k USDC vs ~1M pools
    = 1% price impact per leg kills the edge. Fixed by 1,000 USDC scan
    amount -> hand-verified 132 bps profit on crafted market.
  - Known profitable cycle in config.toml: USDC -> USDT -> SOL -> USDC
    (pool2 1 USDC=1.005 USDT, pool3 1 SOL=98 USDT, pool1 1 SOL=100 USDC).
- [x] Phase 1 teaching walkthrough DONE (guided session through every file):
  - S1: workspace anatomy, dependencies, config.toml as data (base units,
    .env vs config, serde struct contract, fee math: ~100 bps needed to beat
    3x30bps fees + 10bps threshold).
  - S2: token.rs + pool.rs (structs, derive traits, impl, Option<T>, &str vs String).
  - S3: math.rs (constant product x*y=k, u128 widening vs u64 overflow, fee on
    input vs gross-up on output, f64 display-only rule, swap naming pattern
    "X_given_Y = compute X given Y").
  - S4: graph.rs (adjacency list, 2 directed edges per pool, Edge = pool_idx +
    direction, Entry API, Vec<&Edge> borrowed lookups, unwrap_or_default).
  - S5: triangle.rs (DFS + backtracking add/recurse/undo, used_pools not
    used_tokens, legs>=2 floor, unsigned-subtraction trap in calc_profit_bps).
  - S6: config.rs recap (pure parse_toml vs I/O from_file split).
  - Wrap-up mini-quiz (3 questions tying to Phases 2/3): learner chose to move
    on without answering; can re-ask at start of next session.
  - Phase 2 pre-quiz (recap of Phase 1): answered Q1 (used_tokens blocks
    returning to start) + Q2 (f64 precision); Q3 (u64 underflow on losing
    cycle -> debug panic / release wrap to huge gain) taught before Phase 2.
- [x] Phase 2 DONE (cargo test --workspace: 13 passed, 0 warnings). Files:
  - arb-engine scan.rs: `scan(&MarketConfig) -> Vec<Opportunity>` (PoolGraph +
    find_opportunities with scanner config + sort best-first) and
    `fmt_amount(symbol, units, tokens)` (u64 base units -> display string via
    Token.decimals; f64 display-only rule kept).
  - arb-engine src/main.rs (lib+bin, zero new deps): optional <CONFIG> arg
    default config.toml; prints market summary + opportunity table with
    per-leg detail (amounts in/out per leg, pool # + fee, net start->end).
  - Verified: `cargo run -p arb-engine` prints exactly 1 opportunity,
    132 bps: USDC -> USDT -> SOL -> USDC (1000 USDC -> 1013.235458 USDC),
    matching the hand-verified Phase 1 answer.
  - Key learned lesson: fmt_amount lives at the DISPLAY boundary only; profit
    math stays in u128. Scanner output confirms only the mispriced cycle is
    reported (the consistent SOL/JUP/USDC triangles are correctly silent).
- [x] Phase 6 DONE (frontend builds clean; verified live in a real browser).
  Files:
  - frontend/: separate Vite + TypeScript app, own package.json (dev deps only:
    vite 7, typescript 5.8). vite.config.ts proxies /api -> http://127.0.0.1:8080
    and /ws -> ws://127.0.0.1:8080 (ws: true) so the dashboard is same-origin.
  - index.html: static DOM skeleton (header card + opportunities table) — the
    IDs (#mode, #tick, #conn, #prices, #opps, ...) that main.ts renders into.
  - src/types.ts: TS mirror of the backend JSON (StatusResponse, Token,
    ScannerEvent, Opportunity, Leg). Amounts stay in u64 base units.
  - src/main.ts: fetch /api/status + /api/opportunities on load, then WebSocket
    to /ws; each ScannerEvent re-renders tick, price chips, and the opportunity
    table (route <details> expands per-leg breakdown, profit bps/pct colored,
    start/end/net in base-token units via token decimals).
  - src/style.css: dark GitHub-ish theme, badges, chips, table.
  - Verified live: arb-server + `npm run dev`, opened localhost:5173 in Chrome —
    header shows simulator/connected/tick, price chips update, two opportunities
    listed with expandable legs, tick advances live (666 -> 714+).
  - Gotcha: `npm` must be launched as npm.cmd on Windows. Blank black page
    initially because main.ts rendered into elements that didn't exist yet —
    fix was the static HTML skeleton.
- [x] Phase 5 DONE (cargo test --workspace: 24 passed, 1 ignored, 0 warnings). Files:
  - New deps (workspace): axum 0.8 (features ["ws"]), tower-http 0.6 (features
    ["cors"]). arb-engine ScannerEvent gained #[derive(Serialize)] so it can be
    JSON-broadcast.
  - arb-server is now a real binary+lib. lib.rs: AppState { tx (broadcast
    Sender), latest (Arc<RwLock<Option<ScannerEvent>>>), market, started, live };
    cache_latest() task keeps the newest event for the REST endpoints; app()
    builds the Router with CORS. api.rs: /api/status (mode, tick, uptime,
    scanner summary, tokens w/ decimals+mint, pool_count), /api/opportunities
    (latest event's opportunities), /api/pools (config pools). ws.rs: /ws
    upgrades to WebSocket, sends the latest event immediately then streams each
    ScannerEvent as JSON; tokio::select! multiplexes socket ping/close handling
    with rx.recv() so dead clients get cleaned up (send error -> return).
  - main.rs: optional <CONFIG> arg, --live flag, --port=NNNN (default 8080),
    binds 127.0.0.1, spawns scanner task (simulator or jupiter::run_live) +
    cache task, then axum::serve.
  - Verified live: cargo run -p arb-server -> /api/status tick advances,
    /api/opportunities returns full cycles with per-leg amounts, /ws streams a
    JSON event every 500ms (tested with a .NET ClientWebSocket client).
  - Tests: 3 route tests via tower::ServiceExt::oneshot (status/pools/empty
    opportunities). Dev-deps: tower, http-body-util (both already in the tree).
  - Build note: this phase added axum+tokio-tungstenite and stayed under the
    commit limit with CARGO_BUILD_JOBS=2; keep using it on this machine.
- [x] Phase 3b DONE (cargo test --workspace: 32 passed, 3 ignored, 0 warnings).
  This is the "parse real Raydium pool accounts on-chain" option from Phase 4's
  rate-limit lesson, and it SOLVES that lesson: no Jupiter 429s, free public RPC.
  Files/changes:
  - DECISION: dropped solana-client/solana-sdk 4.x entirely. solana-client 4.x
    pulls a ~400-crate tree (quic/tls/x509/borsh/async-trait) whose full build
    blew this machine's pagefile (os error 1455), corrupting target/ and
    cascading into bogus `relaxing a default bound` (syn) + E0786 invalid
    `core` metadata errors. Fix was cargo clean + -j 2, but the tree was too
    heavy to be safe. raydium.rs only needed get_account_data + Pubkey, so we
    replaced them with: reqwest (already a dep) for raw JSON-RPC + bs58 0.5 for
    base58 pubkeys + base64 0.22 for account data. Same code, ~400 fewer crates.
  - arb-core config.rs: OnchainConfig { rpc_url, refresh_ms } (default
    https://api.mainnet-beta.solana.com, 2000) with #[serde(default)];
    MarketConfig gained an `onchain` field. config.toml has [onchain] and each
    pool has its real Raydium AMM v4 `address` (SOL/USDC 58oQ..., USDC/USDT
    7TbG..., USDT/SOL 7Xaw..., SOL/JUP EYEr..., JUP/USDC 7RJ5..., RAY/SOL
    AVs9T...).
  - arb-engine raydium.rs: parse_liquidity_state (752-byte check; baseDecimal
    off 32, quoteDecimal off 40, baseVault 336, quoteVault 368, baseMint 400,
    quoteMint 432), parse_vault_amount (SPL token `amount` = LE u64 at off 64),
    RaydiumSource::fetch_pools, and a run() loop mirroring scanner/jupiter.
    JSON-RPC is a thin hand-rolled client (serde_json::json! body, getMultiple
    Accounts). BATCHING is the key lesson: naive 18 getAccountInfo calls/tick
    (6 pools x pool-state+2 vaults) hit the public RPC's rate limit within 2
    ticks; batching all states in one getMultipleAccounts + all vaults in a
    second = 2 calls/tick -> 9+ clean ticks, zero 429s. Same story as Jupiter:
    per-request scanning gets throttled; batch like real indexers.
  - Tests: 4 unit (synthetic layout, short-account reject, vault offset 64,
    REAL SOL/USDC fixture captured live 2026-08-06: 72.95 USDC/SOL, matching
    api-v3 ~73.0 and Jupiter ~73.5) + #[ignore]d live real_fetch_pools.
  - Verified live: real_fetch_pools fetches all 6 mainnet pools (SOL/USDC
    ~72.8, consistent SOL/JUP + JUP/USDC cross-rate); `--onchain` smoke run
    ticks every 2s with no opportunities (real market: no free money — the
    point).
  - Also fixed: sim.rs first_step_keeps_crafted_mispricing was flaky (random
    seed); pinned to with_seed(cfg, 0) (found via the ignored search_seed test).
  - Bin note: arb-engine has 2 binaries (arb-engine, paper) so onchain mode is
    `cargo run -p arb-engine --bin arb-engine -- --onchain`.
- [x] Phase 4 DONE (cargo test --workspace: 21 passed, 1 ignored (live
  network), 0 warnings). Files:
  - New deps (workspace): reqwest 0.12 (features ["json"]).
  - arb-core config.rs: JupiterConfig (base_url "https://api.jup.ag",
    refresh_ms 2000) with #[serde(default)] + Default impl; MarketConfig gained
    a `jupiter` field. Token gained `mint: Option<String>` (#[serde(default)]);
    config.toml tokens got real Solana mints + a [jupiter] section.
  - arb-engine jupiter.rs: QuoteResponse serde struct (inputMint, outputMint,
    inAmount/outAmount via a custom de_u64 string-number deserializer because
    Jupiter returns amounts as strings; contextSlot default). QuoteApi trait
    (`async fn quote(...)`) + JupiterClient that maps symbol->mint, builds
    `{base_url}/swap/v1/quote?inputMint=...&outputMint=...&amount=...&slippageBps=50`.
    build_candidates(cfg) enumerates every cycle shape once via PoolGraph +
    find_opportunities with i64::MIN threshold + dedup. live_scan<Q: QuoteApi>
    walks candidates leg-by-leg with SEQUENTIAL quotes (a triangle = 3 separate
    HTTP round-trips), builds Legs, reuses calc_profit_bps, filters on
    min_profit_bps, sorts best-first. run_live(cfg, tx, max_ticks) mirrors the
    simulator loop, emitting ScannerEvent every jupiter.refresh_ms.
  - arb-engine main.rs: --live flag dispatches to jupiter::run_live instead of
    the simulator. arb-engine Cargo.toml gained serde + serde_json
    (dev-dependency, tests only).
  - Tests: real-API JSON fixture parse; candidate coverage of both triangles;
    MockQuoter test proving live_scan finds the crafted profitable chain.
    real_quote_usdc_to_sol is #[ignore]'d (network); run it with
    `cargo test -p arb-engine -- --ignored real_quote_usdc_to_sol`.
  - JUPITER ENDPOINT MIGRATION (verified 2026-08-06): the old
    `quote-api.jup.ag` host is DEAD (no DNS records, even via 1.1.1.1/8.8.8.8).
    The "sandbox DNS failure" was really this dead hostname. Jupiter moved to
    `api.jup.ag/swap/v1/quote` (same JSON shape: string inAmount/outAmount,
    numeric contextSlot; unknown fields ignored by serde). All references
    updated (config.toml, config.rs default, jupiter.rs URL, exec.rs comment).
  - REAL PRICING SMOKE TEST PASSED: `real_quote_usdc_to_sol` hits the live API
    and returns real market data (1 USDC -> ~0.0136 SOL, i.e. 1 SOL ~ $73.5;
    matches PowerShell + curl back-to-back). Verified the Rust client parses
    the live response identically to the fixture.
  - RATE LIMIT FINDING (free tier, ~2026): roughly ~5 requests per 10-30s per
    IP (burst limit; observed 5 OK then 429s; after 25s only some pairs
    succeed; heavy pairs like USDC<->USDT throttled hardest). The current
    naive scanner fires 8 sequential quotes per tick -> reliably trips 429,
    so `cargo run -p arb-engine -- --live` currently prints error spam and
    "no opportunities". This is exactly the reality-check lesson: free quote
    APIs can't sustain multi-leg scans; real MEV bots parse on-chain pool
    accounts (Phase 3b, solana-client) instead. Fix options: (a) throttle
    quotes + slow refresh (still bottlenecked by ~5 req/10s), (b) reduce
    per-tick quote count, (c) on-chain pool-state parsing for live prices.
  - Build note: on this machine a full parallel build blew the Windows commit
    limit (os error 1455, ~8GB RAM + 9GB pagefile) and corrupted target/.
    Fix: `cargo clean` + `CARGO_BUILD_JOBS=2` (or `cargo build -j 2`).
- [x] Phase 3 DONE (cargo test --workspace: 18 passed, 0 warnings). Files:
  - New deps (workspace): tokio (rt-multi-thread, macros, sync, time), rand.
  - arb-core config.rs: SimulatorConfig (volatility, pool_volatility,
    mean_reversion, tick_interval_ms) with #[serde(default)] + Default impl;
    MarketConfig gained a `simulator` field. config.toml got a [simulator]
    section.
  - arb-engine sim.rs: Simulator. Initial token prices (USDC=1, SOL=100,
    USDT~0.995, JUP~1.667, RAY~3.333) derived from config pools via a
    propagation loop. Each tick: token prices random-walk, and EACH POOL keeps
    its own deviation (pool_noise) that random-walks and mean-reverts — seeded
    from config so the crafted 132 bps mispricing survives tick 1. step() then
    rebuilds pool reserves around a constant product k (u64 rounding, f64 only
    for market generation).
  - arb-engine scanner.rs: ScannerEvent { tick, prices, opportunities };
    tokio::sync::broadcast channel; async run(cfg, tx, max_ticks) loop that
    steps the sim, re-runs scan(), and sends each tick.
  - arb-engine main.rs: now #[tokio::main]; spawns the scanner task, receives
    on the broadcast channel, prints each tick's prices + opportunities
    best-first. The Phase 2 static table print was replaced.
  - Key learned lessons: (1) rand::thread_rng() is NOT Send (wraps Rc), so it
    can't cross tokio::spawn — use StdRng::from_entropy(). (2) A pure
    token-price simulator can NEVER produce arbitrage: all pools share one
    price per token so they can never disagree — per-pool deviation is what
    creates transient mispricings. (3) 3x30bps fees mean a cycle needs ~1%
    combined mispricing before the 10bps threshold trips — so small volatility
    shows "no opportunities" most ticks; pool_volatility 0.8% makes the demo
    lively. Verified: `cargo run -p arb-engine` shows cycles appearing and
    vanishing each tick (e.g. USDC->USDT->SOL->USDC at 121-442 bps, plus
    USDC->JUP->SOL->USDC).

- [x] Phase 7 DONE (cargo test --workspace: 28 passed, 2 ignored (live network
  + seed search), 0 warnings). Files:
  - New: arb-core config.rs PaperConfig (enabled, starting_capital,
    min_exec_bps, live_exec) with #[serde(default)] + Default impl; config.toml
    gained a [paper] section (10,000 USDC, min 50 bps, live_exec=false kill
    switch).
  - New: arb-engine exec.rs. Trade { tick, path, profit_bps, start_units,
    end_units, profit_units, balance_units }. PaperExecutor holds capital/
    starting_capital/min_exec_bps + Vec<Trade>/wins/losses; on_event() takes the
    BEST opportunity from the event, skips below min_exec_bps or unaffordable,
    books the simulated fill (fills at the DETECTED price, zero latency —
    teaching point: real MEV bots front-run this), tracks win/loss. total_pnl_
    units() + roi_bps() (i128 math). LiveExecutor stub: enabled from cfg.paper.
    live_exec; execute() anyhow::bail!s — kill switch is OFF by default and
    the path to a real swap is a stub.
  - New: arb-engine src/bin/paper.rs (second binary; lib.rs gains pub mod exec).
    Manual arg parsing (positional config path, --ticks=N, no clap). Spawns
    scanner::run(cfg, tx, Some(ticks)), rx.recv() loop feeds PaperExecutor,
    prints fills + final report (trades/wins/losses/start/end/realized PnL via
    fmt_amount + roi_bps, pnl.unsigned_abs() for display).
  - Also on disk from earlier work: sim.rs Simulator::with_seed + a broken
    #[ignore]d search_seed test — fixed its &[&str] vs &[String] comparison
    so the workspace compiles.
  - Verified: cargo test --workspace green; `cargo run -p arb-engine --bin paper
    -- --ticks=60` executes 32 fills, +382 bps realized (simulator always wins
    because fills assume the detected price with zero latency/slippage — a
    lesson, not a feature).

- [x] Frontend REDESIGN + DESIGN SYSTEM + ONCHAIN WIRING DONE (cargo test
  --workspace: 33 passed, 3 ignored (live network + seed search + real
  fetch), 0 warnings; npm run build clean). Files:
  - Backend: arb-server main.rs gained `--onchain` flag (3-way dispatch
    simulator / jupiter live / raydium onchain); AppState.live: bool → mode:
    &'static str (3 route tests updated); /api/status.mode now reports
    "simulator" | "live" | "onchain" (drop-in for the frontend tag).
  - arb-engine raydium.rs gained prices_from_pools(): derives a token-price
    map from LIVE pool reserves by propagating from the base token (price 1.0)
    across each pool's spot rate (same algorithm as sim::initial_prices);
    ScannerEvent.prices is now populated in onchain mode. Test
    prices_derived_from_crafted_config verifies USDC=1, USDT≈0.995,
    SOL=100 on the crafted config.
  - frontend/DESIGN.md: NEW 20-section design system document (philosophy,
    visual language, color/type/grid/spacing tokens, component rules,
    interactions, motion, a11y, responsive, dashboard layout, future
    trading+portfolio blueprints, token sheet, full component library with
    anatomy/state/keyboard/mobile/a11y/motion sheets, do's & don'ts, review
    checklist, scalability, Figma org). Dark institutional theme (Bloomberg +
    Apple / Linear + Coinbase): bg #07090D, surface #10141B, raised #161B23,
    border rgba(255,255,255,.08), accent #4B7BFF, profit #00C076, loss
    #F6465D, warning #F5A623. 8pt spacing incl 56/128, radii 4/8/12/16/20/
    999, motion cubic-bezier(.2,.8,.2,1) @150/200/250ms, Inter Variable +
    mono, 300–800 weights. Scope decision: real product components fully
    specced; order book/trade ticket/sidebar/charts = FUTURE blueprint only.
  - frontend style.css rethemed dark: tokenized colors/shadows/motion, 72px
    sticky glass nav (blur 20px), 16px card radius, 4px badge radius, 700
    profit cell, focus-visible 2px accent ring on expandable rows, hover wash
    overlay, reduced-motion disables pulse/flash/transitions.
  - index.html color-scheme → dark; new public/favicon.svg (indigo rounded
    tile + white triangle glyph) fixes the favicon 404.
  - main.ts unchanged (class-driven colors); grid/table/delta markup already
    matched the new system.
  - Verified live in browser: simulator mode (SIMULATOR amber tag, flashing
    new cycles, expandable legs, mobile net-column collapse) AND --onchain
    mode (MAINNET green tag, REAL mainnet prices SOL=72.80/JUP=0.18/RAY=
    0.61/USDT=0.997, empty state "watching 6 pools"). Computed-style audit
    confirmed every token value (bg #07090D, glass rgba(16,20,27,.85)+20px,
    profit/loss green/red, 16px radius, 72px nav). Mobile 390px: mode tag +
    conn pill stay, updated timestamp hides, chips wrap at 118px, no
    horizontal overflow.

- [x] Phase B DONE — CROSS-DEX (Orca Whirlpools) (cargo test --workspace: 40
  passed, 3 ignored (2 live network + seed search), 0 warnings). Files:
  - arb-core pool.rs: `Dex` enum (`raydium` | `orca`, serde lowercase,
    `Default = Raydium`, `as_str()`); `Pool.dex: Dex` with `#[serde(default)]`;
    `Pool::new` sets `Dex::Raydium`; lib.rs re-exports `Dex`.
  - config.toml: `dex = "raydium"` on all 6 existing pools; + 2 Orca
    Whirlpools: SOL/USDC `Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE`
    (fee_bps 4) and SOL/USDT `FwewVm8u6tFPGewAyHmWAqad9hmF7mvqxK4mJ7iNqqGC`
    (fee_bps 2), sim-consistent reserves (noise = 1, no tick-0 arb).
  - arb-engine onchain.rs REPLACES raydium.rs (older log entries referencing
    raydium.rs are historical). `OnchainSource::fetch_pools` dispatches per
    `pool.dex`; Raydium path unchanged (state + 2 vault reads); Orca path
    parses the Whirlpool state account and derives VIRTUAL reserves from
    liquidity + sqrt_price: `amount_a = liquidity * 2^64 / sqrt_price`,
    `amount_b = liquidity * sqrt_price / 2^64` — constant product = L^2, so the
    x*y=k math and profit calc work unchanged (CL depth is an approximation).
  - Whirlpool account layout (653 bytes, 8-byte discriminator): fee_rate u16
    @45 in HUNDREDTHS of a bp (400 = 4 bps), liquidity u128 @49, sqrt_price
    u128 Q64.64 @65, mint_a @101, vault_a @133, mint_b @181, vault_b @213.
  - Wiring: lib.rs `pub mod onchain`; arb-engine + arb-server main.rs use
    `onchain::run`; mode string "On-chain pool state (Raydium + Orca)".
  - Tests: pools.len() 6→8 (config.rs + arb-server route tests); new onchain
    tests: synthetic whirlpool layout parse, rejects (short data / zero
    sqrt_price / zero liquidity), REAL SOL/USDC + SOL/USDT fixtures captured
    live 2026-08-07 (SOL/USDC: L = 1_272_258_878_937_910, sqrt =
    5_005_843_578_419_737_334 → virtual SOL 4.688e15 / USDC 3.452e14 ≈ 73.6
    USDC/SOL; SOL/USDT: L = 776_393_878_771, sqrt = 5_007_727_163_625_209_098);
    virtual_reserves checks: spot price = a/b and k = L^2.
  - Verified live: real_fetch_pools (all 8 pools, both DEXes, orca ~73.6
    consistent with raydium); `--onchain --ticks=2`: prices JUP 0.1830 RAY
    0.6179 SOL 73.5799 USDC 1.0000 USDT 0.9972; near-miss -10 bps
    USDC→SOL→USDC (buy SOL on Orca @73.33, sell on Raydium @73.49: gross
    +18 bps, 4 + 25 bps fees eat it).
  - CROSS-DEX LESSON: same-pair 2-leg cycles are now found — the two live
    SOL/USDC pools genuinely disagree by ~18 bps because the 29 bps combined
    fee band is wider than the gap (no free arb exists, and the scanner
    honestly reports it as a near-miss).

- [x] Phase 1 SAFETY & ROBUSTNESS DONE (tests: 65 passed, 3 ignored, 0 warnings).
  - 1.1 Panic-to-error in onchain.rs: `read_u16/64/128/pubkey` helpers now
    return `Result` instead of panicking on short/corrupted account data.
    `parse_liquidity_state`, `parse_whirlpool_state`, `parse_vault_amount`
    propagate errors gracefully. Single pool parse/vault failures log to stderr
    and skip that pool instead of aborting the whole scan.
  - 1.3 Defensive math clamps: `swap_out_given_in` and `swap_in_given_out` clamp
    `fee_bps` to `BPS - 1` (9,999) to prevent division-by-zero if a pool ever
    reports 10,000 bps. Added test `fee_bps_clamped_to_bps_minus_one`.
  - 1.2 Retry module: new `crates/arb-engine/src/retry.rs` — generic
    `with_backoff` with exponential backoff + jitter. Wired into `onchain.rs`
    (`get_multiple_accounts`, `get_slot`) and `jupiter.rs` (`fetch_quote`).
    Retries up to 3× on HTTP 429 / 5xx. Added 3 retry tests.

- [x] Phase 2 SCANNER ACCURACY DONE.
  - 2.1 Separate mainnet config: created `config.mainnet.toml` with real mainnet
    pool addresses (no crafted 132 bps mispricing). Replaced invalid addresses
    with verified live ones (SOL/USDC, USDC/USDT, USDT/SOL, SOL/JUP, JUP/USDC,
    RAY/SOL, 2 Orca Whirlpools). Reduced `refresh_ms` to 5000 to avoid rate
    limits. `ScannerEvent` gained `is_simulated` field.
  - 2.2 Quote staleness guard: `ScannerEvent` gained `quoted_at` and `stale`
    fields. Jupiter mode records scan start time; if scan exceeds `refresh_ms`,
    event is marked `stale: true`.
  - 2.3 Slot consistency: removed over-aggressive post-fetch `get_slot()` re-check
    that discarded entire batches when slot advanced by 1 (was skipping ~every
    scan on public RPC).
  - 2.4 Deterministic simulator seed: `SimulatorConfig` gained `seed: Option<u64>`.
    `Simulator::new` uses config seed or logs random seed for reproducibility.

- [x] Phase 3 TEST COVERAGE DONE.
  - 3.1 Property tests for swap math: `round_trip_never_exceeds_input`,
    `product_invariant_under_swap`, `spot_price_monotonic_with_reserves`,
    `fee_bps_clamped_to_bps_minus_one`.
  - 3.2 Server integration tests: `history_populates_after_broadcast_events`,
    `ws_route_is_wired`, `executor_returns_paper_state`.
  - 3.3 On-chain resilience tests: `rejects_corrupted_liquidity_state`,
    `rejects_corrupted_whirlpool_state`, `rejects_short_vault_data`,
    `vault_parse_errors_are_not_panics`.

- [x] Phase 5 ARCHITECTURE POLISH DONE.
  - Ring buffer: replaced `Vec` + O(n) `drain` in `AppState.history` with
    `RingBuffer<HistoricalOpportunity>` (O(1) push, fixed capacity 1000).
  - Graceful shutdown: `arb-server` and `arb-engine` both listen for `SIGINT`
    via `tokio::signal::ctrl_c()`. Server drops broadcast receiver to unblock
    scanner, then awaits both handles.
  - Config validation: `MarketConfig::parse_toml` validates that every pool
    references known tokens and that mints are valid base58. Zero reserves log
    warnings.
  - CORS tightened: replaced `CorsLayer::permissive()` with `AllowOrigin::exact`
    read from `[server] allowed_origin` in config (default
    `http://localhost:5173`).
  - PaperExecutor wired into arb-server: `AppState` gains `executor:
    Arc<RwLock<PaperExecutor>>`. `cache_latest` feeds `executor.on_event()`.
    New REST endpoint: `/api/executor` returns executor state.

- [x] Phase 4 FRONTEND DONE.
  - History chart: Canvas bar chart in the History view showing profit bps per
    opportunity (muted slate for profitable, gray for near-miss).
  - PnL panel: Execution sidebar shows paper-trading PnL when executor has
    activity (capital, realized PnL, ROI, trades, wins, losses).
  - Trade log: `/api/executor` endpoint wired; frontend fetches it on boot and
    updates the PnL panel live.
  - History tab bug fix: `renderHistoryShell()` now returns `{ module, render }`
    with closed-over DOM references. Both standalone History tab and terminal
    dashboard use their own instance.
  - Removed favicon and logo: deleted `public/favicon.svg`, removed `<link
    rel="icon">`, removed SVG logo from nav button, removed `.brand-mark` CSS.
  - Muted color scheme: replaced bright greens/accents with muted slate/gray
    tones. Removed `--shadow-glow`, `body::after` gradient overlay,
    `flash-up`/`flash-down` profit animations. `.btn-primary` gradient → flat
    muted background. `.hero-metric-num` no longer green.
  - Scroll boundary: `overscroll-behavior: auto` on `.feed-list` and
    `.history-body` so scroll chaining works (page scrolls when cursor is over
    a panel).
  - Route network zoom/pan: scroll wheel zooms (0.4x–4x), click+drag pans,
    double-click resets. Updated hover hint.
  - History auto-flush: frontend trims entries older than 30 minutes and caps
    at 500 entries. Cleanup runs on boot then every 60 seconds.
  - Execution panel simplified: shows only Mode, Pools watched, Best edge.
    Removed Ticks seen, Last update, Backend latency, Scan size, Minimum edge,
    Uptime, Confidence meter, Max size/Capacity, and PnL section.

- [x] MAINNET CONFIG CLEANUP DONE.
  - `config.mainnet.toml` has 8 verified working pools (6 Raydium AMM v4 + 2
    Orca Whirlpools). Removed 4 invalid/deprecated pool addresses.
  - Added log throttle: `OnchainSource::log_skip` caches per-pool skip messages
    in a `Mutex<HashSet>`. Each pool address logged once ever. Summary line per
    scan: `onchain: 8/8 pools fetched (0 skipped)`.

## Current file inventory (post-Phase B + improvements)
- `Cargo.toml` (workspace root)
- `config.toml` (simulator/crafted)
- `config.mainnet.toml` (real mainnet, consistent reserves)
- `crates/arb-core/`: Cargo.toml, lib.rs, token.rs, pool.rs, math.rs, graph.rs,
  triangle.rs, config.rs
- `crates/arb-engine/`: Cargo.toml, lib.rs, main.rs, bin/paper.rs, scan.rs,
  scanner.rs, sim.rs, jupiter.rs, onchain.rs, exec.rs, retry.rs
- `crates/arb-server/`: Cargo.toml, lib.rs, main.rs, api.rs, ws.rs,
  ring_buffer.rs
- `frontend/`: package.json, vite.config.ts, index.html, src/main.ts,
  src/types.ts, src/state.ts, src/router.ts, src/terminal.ts, src/effects.ts,
  src/format.ts, src/analysis.ts, src/viewState.ts, src/views/{landing,history,
  feed,exec,intel,sculpture}.ts, src/style.css, public/ (no favicon)

## Test status
- 65 passed, 3 ignored (live network + seed search), 0 warnings
- Frontend `npm run build` clean

## Next step when resuming
Phase C: Docker + AWS EC2 deployment.
- Create Dockerfiles for arb-server and frontend
- Create docker-compose.yml
- Test build locally
- Deploy to EC2 (t3.small, Ubuntu 22.04)
- Security group: inbound 80, 443, 22
- Config: mount `config.mainnet.toml` as volume
- Costs: ~$15/month for t3.small

Design work: any future panel (watchlist/chart/paper PnL) must be added to
frontend/DESIGN.md §16 before implementation per the review checklist.

