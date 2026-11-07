# solana-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `balances`, `blockchain`, `config`, `decoder`, `event_builder`, `file_config`,
  `instruction_processor`, `oracle`, `rpc_retry`, `symbols`, `transaction_fetcher`,
  `transaction_handler`. `main.rs` holds the run loop.
- Pipeline layering, in the order a log line travels:
  `transaction_handler` (`TransactionHandler`) orchestrates and owns every I/O call —
  `transaction_fetcher` fetches, `symbols` and `oracle` enrich, then `event_builder` builds.
  `instruction_processor` decodes instructions and is pure: no RPC, no clock, no logging of
  events. `event_builder` is pure too, taking prices and symbols as arguments. Keep new I/O in
  the handler, not in the inner modules.
- Shared behavior (event structs, Avro, CloudEvent envelope, publisher with retry/DLQ, finality
  buffer) comes from `dex-ingester-common`.
- Dependencies:
    - Solana SDK 2.0+ (compatible with pyth-solana-receiver-sdk)
  - `pyth-solana-receiver-sdk` for price feeds, not legacy `pyth-sdk-solana`
  - `mpl-token-metadata` for SPL token symbols
  - Warp 0.4+ requires the `server` feature for `warp::serve()`
