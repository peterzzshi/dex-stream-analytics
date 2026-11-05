# dex-stream-analytics

Real-time Flink pipeline detecting MEV and scoring DEX pool health from live DEX events (Uniswap V2 on EVM, Raydium AMM
v4 on Solana).

## Commands

Independent toolchains, no root build/test. Run lint before finishing.

- **evm-ingester / solana-ingester** (Rust): `cd <dir> && cargo build | cargo test`, lint with
  `cargo fmt --check && cargo clippy -- -D warnings`
- **aggregator** (Java 21, Maven): `cd aggregator && mvn -DskipTests package | mvn test`. No lint configured.
- **analytics-service** (Kotlin, Gradle): `cd analytics-service && ./gradlew build | ./gradlew test`, lint with
  `./gradlew ktlintCheck`
- Full stack: `docker compose up -d --build`

## Architecture

`evm-ingester/` and `solana-ingester/` (Rust) → `aggregator/` (Flink/Java) → `analytics-service/` (Kotlin/Ktor),
connected by Kafka with Dapr pub/sub. Contracts are the Avro schemas (root `schemas/`) and Kafka topic names. See
`ARCHITECTURE.md` for the diagram and design decisions, `DATA_MODEL.md` for schemas.

Invariants:

- Pool identity is `(chain, pairAddress)`, never bare `pairAddress`
- Enrichment (USD price, token symbols) happens in the ingester at ingest time for deterministic Flink replay
- The aggregator validates the CloudEvent `type` before Avro decode; the liquidity topic carries mixed schemas

## Conventions

- Comment only the *why*, never the *what*
- Pure functions in core; side effects (RPC, Kafka, Redis) at edges
- Domain vocabulary lives in `CONTEXT.md`; check it before introducing new terms
- Component-specific conventions: `<component>/AGENT.md`
- Secrets (RPC URLs with API keys) are environment variables only, never in files. Non-secret reference data (chain,
  pools, tokens, oracle feeds) lives in `solana.toml` / `evm.toml`, parsed with `deny_unknown_fields` and validated at
  load. Required config has no fallback defaults and fails at startup.
- Testing and git rules: `CLAUDE.md`. Mock only what the behavior needs.
- Docker Compose has no `profiles:` key. Both ingesters start under `docker compose up -d` with
  `restart: unless-stopped`
  and crash-loop by design when RPC URLs are missing (fail-fast). Do not add profiles.

### Rust ingesters (`evm-ingester`, `solana-ingester`)

- Follow Rust best practices; `cargo fmt` enforced. Apply `/functional-programming-patterns`: explicit data flow,
  side effects confined to module boundaries.
- Flat modules under `src/`, no nested submodules.
- Built-in test framework with `#[cfg(test)]` modules. Inject lookups and clocks as parameters (see `Config::load`).
- No database. Event buffering is an in-memory `FinalityBuffer` (block-based on EVM, slot-based on Solana).

## Workflow

- Small commits, plain imperative messages (match history: `Add Kotlin analytics-service`)
- Ask before adding dependencies; name the ecosystem (`Cargo.toml`, `pom.xml`, `build.gradle.kts`)
- If doc and code disagree, surface the conflict before changing either
- Current state and next steps live in `SESSION-HANDOFF.md`; known gaps in `GAPS.md`; decisions in
  `ARCHITECTURE.md#design-decisions`
