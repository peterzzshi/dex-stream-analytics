# AGENT.md

Conventions and commands for this repository. Plans, current state and milestone status live in
`SESSION-HANDOFF.md`; gaps in `GAPS.md`; decisions in `ARCHITECTURE.md`.

## Commands

Independent toolchains, no root build/test. Run lint before finishing.

- **evm-ingester / solana-ingester** (Rust): `cd <dir> && cargo build | cargo test`, lint with
  `cargo fmt --check && cargo clippy -- -D warnings`
- **aggregator** (Java 21, Maven): `cd aggregator && mvn -DskipTests package | mvn test`. No lint
  configured; checkstyle.xml is present but not wired into the build.
- **alerting-service** (Kotlin, Gradle): `cd alerting-service && ./gradlew build | ./gradlew test`,
  lint with `./gradlew ktlintCheck`
- Full stack: `docker compose up -d --build`

## Architecture

`evm-ingester/` and `solana-ingester/` (Rust) → `aggregator/` (Flink/Java) → `alerting-service/`
(Kotlin/Ktor), connected by Kafka with Dapr pub/sub. Contracts are the Avro schemas in root
`schemas/` and the Kafka topic names in `docker-compose.yml`.

Invariants:

- Bridge identity is `(chain, bridgeAddress)`, never bare `bridgeAddress`. The one exception is the
  Cross-Chain Flow pipeline, which keys on the bridge alone to compare chains.
- Enrichment (token symbols) happens in the ingester at ingest time, so Flink replay reproduces the
  same values. It is currently unimplemented — symbols are null.
- Each ingest topic carries exactly one record type. `bridge-transfer-events` is
  `BridgeTransferEvent`; `bridge-supply-events` is `BridgeSupplyEvent`.
- Alert values on Kafka are Avro binary inside a CloudEvent, not JSON.
- Window lengths are required env vars with no defaults.

## Conventions

- Comment only the *why*, never the *what*.
- Pure functions in core; side effects (RPC, Kafka, Redis) at edges.
- Component-specific conventions: `<component>/AGENT.md`
- Secrets (RPC URLs with API keys) are environment variables only, never in files. Non-secret
  reference data (chain identity, bridge addresses, token tables) lives in `evm-bridges.toml` /
  `solana-bridges.toml`, parsed with `deny_unknown_fields` and validated at load. Required config has
  no fallback defaults and fails at startup.
- Testing and git rules: `CLAUDE.md`. Mock only what the behavior needs.
- Docker Compose has no `profiles:` key. Both ingesters start under `docker compose up -d` with
  `restart: unless-stopped` and crash-loop by design when RPC URLs are missing (fail-fast). Do not
  add profiles.

### Rust ingesters (`evm-ingester`, `solana-ingester`)

- Follow Rust best practices; `cargo fmt` enforced. Explicit data flow, side effects confined to
  module boundaries.
- Flat modules under `src/`, no nested submodules. `lib.rs` exposes them for tests; `main.rs` wires.
- Built-in test framework with `#[cfg(test)]` modules. Inject lookups and clocks as parameters (see
  `Config::load`, `BridgeFileConfig::parse`).
- No database. Event buffering is an in-memory `FinalityBuffer` (block-depth on EVM, slot-based on
  Solana).

## Workflow

- Small commits, plain imperative messages (match history: `Add gas fees in ingesters`)
- Ask before adding dependencies; name the ecosystem (`Cargo.toml`, `pom.xml`, `build.gradle.kts`)
- If doc and code disagree, surface the conflict before changing either
