# solana-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `balances`, `decoder`, `oracle`, `publisher`, `finality`, `blockchain`, `symbols`, `rpc_retry`.
- Dependencies:
    - Solana SDK 2.0+ (compatible with pyth-solana-receiver-sdk)
  - `pyth-solana-receiver-sdk` for price feeds, not legacy `pyth-sdk-solana`
  - Apache Avro 0.22+ (`Writer::new()` and `into_inner()` return `Result`)
  - Warp 0.4+ requires the `server` feature for `warp::serve()`
