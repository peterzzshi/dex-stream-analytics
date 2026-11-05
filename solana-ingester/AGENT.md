# solana-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `balances`, `blockchain`, `config`, `decoder`, `events`, `file_config`, `oracle`,
  `rpc_retry`, `symbols`. `main.rs` holds the run loop.
- Shared behavior (event structs, Avro, CloudEvent envelope, publisher with retry/DLQ, finality
  buffer) comes from `dex-ingester-common`. `events.rs` is a re-export shim.
- Dependencies:
    - Solana SDK 2.0+ (compatible with pyth-solana-receiver-sdk)
  - `pyth-solana-receiver-sdk` for price feeds, not legacy `pyth-sdk-solana`
  - `mpl-token-metadata` for SPL token symbols
  - Warp 0.4+ requires the `server` feature for `warp::serve()`
