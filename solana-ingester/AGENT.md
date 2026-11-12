# solana-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `blockchain`, `bridge_decoder`, `bridge_file_config`, `bridge_handler`,
  `bridge_normalizer`, `config`, `rpc_retry`. `main.rs` holds the run loop and the health server.
- Data flow: `bridge_handler` owns every I/O call for one bridge — fetch the transaction, walk its
  instructions, hand the decoded result on. `bridge_decoder` is pure (bytes in, decoded event out);
  `bridge_normalizer` is pure too, taking the event plus the fetch result. Keep new I/O in the
  handler.
- Instructions arrive as `PartiallyDecoded` from routers, and bridge programs are usually reached
  via CPI rather than a top-level instruction — match on program id, not on instruction position.
- Finality is external: the subscription uses `SOLANA_COMMITMENT` and a separate poller advances
  `FinalityBuffer` to the finalized slot (`FINALITY_POLL_INTERVAL_SECS`). There is no block-depth
  equivalent to the EVM ingester.
- Shared behavior (event structs, Avro, CloudEvent envelope, retry/DLQ publisher, `FinalityBuffer`)
  comes from `dex-ingester-common` (crate `dex_ingester_common`, path `../common`). Do not add a
  local copy.
- Dependencies: `solana-client` / `solana-sdk` / `solana-transaction-status` (the latter needs the
  `agave-unstable-api` feature for instruction parsing); `warp` with the `server` feature for
  `warp::serve()`. No price or token-metadata crate — enrichment is not implemented.
- Protocol values are `wormhole` and `allbridge` only; both are validated at load.
