# evm-ingester conventions

Extends root `AGENT.md`.

- Idiom: follow Rust best practices; `rustfmt` enforced via `cargo fmt`.
- Apply `/functional-programming-patterns` — pure functions, explicit data flow, side effects confined to module
  boundaries.
- Module layout: flat modules under `src/` (`avro`, `decoder`, `oracle`, `publisher`, `finality`, etc.). No nested
  submodules.
- Testing: use built-in Rust test framework with `#[cfg(test)]` modules; pair with `anyhow::Result` for error handling.
- Dependencies:
    - ethers 2.0+ with `ws` feature for WebSocket support
    - Apache Avro 0.16+ (older version than solana-ingester; uses `Writer::append_value_ref` API)
    - Use `tokio` for async runtime; `reqwest` for HTTP clients (Chainlink oracle)
- No database today. Event buffering uses in-memory `FinalityBuffer` with block-based finality tracking.
