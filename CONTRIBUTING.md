# Contributing

Thanks for your interest in this project. It is primarily a learning vehicle for async Rust, Solana DEX mechanics, and fullstack web3. If you want to experiment, fix bugs, or extend the scanner, this guide will help you land changes cleanly.

## Ground rules

- **Paper trading only.** `paper.live_exec` must remain `false` in every shipped config. Real swap execution is a stub and must stay that way.
- **No secrets in code or config.** Never commit real RPC keys, Jupiter API keys, or private keys. Use environment variables or mounted config files.
- **Clean code only.** Explanations belong in chat / PR descriptions, not in code comments. Follow the existing style in each file.
- **Frontend changes require design updates.** Any new panel, view, or component must be added to `frontend/DESIGN.md` under §16 (Component Library) before implementation.

## Setup

```bash
git clone <repo-url>
cd rust_proj

# Rust
rustup default stable
cargo test --workspace

# Frontend
cd frontend
npm install
npm run build
```

Windows note: if a full parallel build exhausts RAM, use `cargo build -j 2`.

## Branching and commits

- Create a branch from `main` with a descriptive name (`feat/simulator-seed`, `fix/onchain-vault-offset`).
- Keep commits focused. If a PR touches both backend and frontend, split it or explain the coupling in the PR body.
- Write commit messages in imperative mood, scoped to the affected crate when possible:

  ```
  feat(arb-engine): add deterministic simulator seed
  fix(arb-server): tighten CORS to exact origin
  ```

## Code style

- Match the surrounding code exactly: indentation, naming, module layout, and imports.
- Use `anyhow::Result<T>` in engine/server code. Return errors instead of panicking.
- Keep `f64` out of profit math. `f64` is allowed only at the display boundary.
- Prefer `u128` for intermediate arithmetic on `u64` reserves/amounts.
- Use `#[derive(Serialize, Deserialize)]` for types that cross the API boundary.

## Testing

Every PR should keep the workspace green:

```bash
cargo test --workspace
cd frontend && npm run build
```

If you add a new source of live data (new DEX, new quote API), include:
- at least one unit test with synthetic data,
- one ignored integration test for the live endpoint,
- a short note on rate limits or costs.

## Frontend design system

The dashboard uses a strict dark design system documented in `frontend/DESIGN.md`. Before adding UI:

1. Add the new component or panel spec to §16.
2. Match existing motion, spacing, color tokens, and a11y patterns.
3. Do not introduce new global colors or shadow tokens without discussion.

## Deployment notes

- Docker images are defined in `crates/arb-server/Dockerfile` and `frontend/Dockerfile`.
- `docker-compose.yml` mounts `config.mainnet.toml` read-only and exposes backend `:8080` and frontend `:3000`.
- For EC2 or any remote host, restrict inbound to `22`, `80`, and `443`. Use a reverse proxy for TLS.

## Questions

Open an issue or start a discussion. If the change is learning-oriented, say so — this repo values teaching clarity over cleverness.
