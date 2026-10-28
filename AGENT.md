# dex-stream-analytics

Real-time Flink pipeline that detects MEV and scores DEX pool health from live Uniswap V2 events.

## Commands

Three independent toolchains, no root build/test command.

**evm-ingester** (Rust)

- Build: `cd evm-ingester && cargo build`
- Test: `cd evm-ingester && cargo test` (single: `cargo test finality::tests::buffer_holds_events_until_confirmed`)
- Lint: `cd evm-ingester && cargo fmt --check && cargo clippy -- -D warnings` ← run before finishing any evm-ingester
  task

**aggregator** (Java 21 / Flink, Maven)
- Build: `cd aggregator && mvn -DskipTests package`
- Test: `cd aggregator && mvn test` (single: `mvn test -Dtest=SwapAggregatorTest#twap_singleSwap_equalToPrice`)
- Lint: none — Checkstyle is wired in `pom.xml` but commented out ("incompatible with current setup"); don't re-enable as a drive-by fix

**analytics-service** (Kotlin / Ktor, Gradle)
- Build: `cd analytics-service && ./gradlew build`
- Test: `cd analytics-service && ./gradlew test` (single: `./gradlew test --tests "PoolHealthScorerTest"`)
- Lint: `cd analytics-service && ./gradlew ktlintCheck` (auto-fix: `ktlintFormat`) ← run before finishing any
  analytics-service task

Full stack: `docker compose up -d --build` (see root `README.md` for the live-chain `--profile live` path).

## Architecture

`evm-ingester/` (Rust) → `aggregator/` (Flink/Java) → `analytics-service/` (Kotlin/Ktor), connected by Kafka with Dapr
pub/sub at the edges. Cross-component contracts are the Avro schemas and Kafka topic names, not shared code. Full
diagram: `ARCHITECTURE.md`; schemas: `DATA_MODEL.md`.

Invariants:
- **Pool identity is `(chain, pairAddress)`, never bare `pairAddress`.** Multi-chain is actively being built — see `ARCHITECTURE.md#design-decisions`.
- **Enrichment (Chainlink price, token symbols) happens in the ingester at ingest time, not downstream.** Freezes enriched fields so Flink replay is deterministic.
- **The aggregator validates the CloudEvent `type` field before Avro decode** — the liquidity topic carries mixed schemas (Mint/Burn/Transfer); decoding by shape-guessing would silently misdecode.

## Conventions

- Comment only the *why*, never the *what*; Code should be self explanatory, without comment verbosity.
- Pure functions in core; side effects (RPC, Kafka, Redis) at the edges.
- Tests assert behaviour, not implementation.
- Cross-cutting domain vocabulary (chain, pool, window, finality gate) lives in root `CONTEXT.md` — check it before introducing new terms.
- Language/tool-specific conventions (idiom guide, package layout, mocking, functional-pattern usage) live in each
  component's own `AGENT.md`: `evm-ingester/AGENT.md`, `aggregator/AGENT.md`, `analytics-service/AGENT.md`.

## Workflow

- Small commits, plain imperative messages (match existing history: `Add Kotlin analytics-service`, not Conventional Commits).
- Ask before adding a dependency; name which ecosystem (`evm-ingester/Cargo.toml`, `aggregator/pom.xml`,
  `analytics-service/build.gradle.kts`) — each is independent.
- If a doc and the code it describes disagree, surface the conflict before changing either.
- **After completing each task, update `SESSION-HANDOFF.md` with current state, what changed, and what's next.**

### Carrying context across sessions

- **`PLAN.md`** (repo root, create only when multi-session work is in flight): plan with checkboxes, updated as work progresses. Delete when done.
- **Decision records**: `ARCHITECTURE.md#design-decisions` already holds the big ones (pool identity, Flink vs. Kafka Streams, Dapr's scope). Add new ones there; don't start a parallel log.
- **`SESSION-HANDOFF.md`** (repo root, exists now): current state + next steps, read at session start. Update as work
  progresses; consolidate before context clears.
- **Git history**: small, descriptive commits make `git log`/`git diff` double as context.
