# evm-ingester conventions

Extends root `AGENT.md` (including its Rust ingesters section).

- Modules: `avro`, `decoder`, `oracle`, `publisher`, `finality`, `listener`, `rpc_retry`, `dlq` (to be removed).
- Dependencies:
    - ethers 2.0+ with the `ws` feature for the WebSocket subscription
    - `reqwest` for the Chainlink oracle
    - Versions are aligned with solana-ingester in `EXTRACTION-PLAN.md`
