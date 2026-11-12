# evm-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `bridge_decoder`, `bridge_file_config`, `bridge_listener`, `bridge_normalizer`,
  `config`, `rpc_retry`. `lib.rs` exposes them for tests; `main.rs` wires them.
- Data flow: `bridge_listener` subscribes and owns the WebSocket connection, `bridge_decoder`
  turns a raw `Log` into a decoded event, `bridge_normalizer` assembles the ingest record. The
  normalizer takes the block timestamp and gas fields as arguments — it does not call the RPC
  itself.
- Shared behavior (event structs, Avro, CloudEvent envelope, retry/DLQ publisher, `FinalityBuffer`)
  comes from `dex-ingester-common` (crate `dex_ingester_common`, path `../common`). Do not add a
  local copy.
- One `Bridge` per `[[bridge]]` entry; each runs its own listener task under one process. The RPC
  URL and `RetryConfig` are per-process, the address and protocol per bridge.
- `confirmations` is a constant (12) in `config.rs`, not configurable from TOML. Changing it means
  changing code.
- Dependencies: ethers 2.0 with the `ws` feature; `apache-avro` for encoding. No oracle or price
  crate — enrichment is not implemented.
