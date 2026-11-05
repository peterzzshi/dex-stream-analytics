# evm-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `config`, `decoder`, `events`, `file_config`, `listener`, `oracle`, `rpc_retry`.
  `lib.rs` exposes them for tests; `main.rs` wires them.
- Shared behavior (event structs, Avro, CloudEvent envelope, publisher with retry/DLQ, finality
  buffer) comes from `dex-ingester-common`. Do not add a local copy.
- Dependencies:
    - ethers 2.0+ with the `ws` feature for the WebSocket subscription
    - `reqwest` for the Chainlink oracle
  - Versions align with `solana-ingester`; both are workspace-excluded members and resolve
    against `common/`
