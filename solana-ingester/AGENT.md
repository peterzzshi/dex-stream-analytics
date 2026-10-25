# solana-ingester conventions

Extends root `AGENT.md`.

- Idiom: follow Rust best practices; `rustfmt` enforced via `cargo fmt`.
- Apply `/functional-programming-patterns` — pure functions, explicit data flow, side effects confined to module
  boundaries.
- Module layout: flat modules under `src/` (`balances`, `decoder`, `oracle`, `publisher`, etc.). No nested submodules.
- Testing: use built-in Rust test framework with `#[cfg(test)]` modules; pair with `anyhow::Result` for error handling.
- Dependencies:
    - Solana SDK 2.0+ (compatible with pyth-solana-receiver-sdk)
    - Use `pyth-solana-receiver-sdk` for price feeds (not legacy `pyth-sdk-solana`)
    - Apache Avro 0.22+ (uses `append_value_ref`, `Writer::new()` returns `Result`, `into_inner()` returns `Result`)
    - Warp 0.4+ requires `server` feature for `warp::serve()`
- No database today. Event buffering uses in-memory `FinalityBuffer` with slot-based finality tracking.
