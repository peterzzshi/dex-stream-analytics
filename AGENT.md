# dex-stream-analytics

Real-time Flink pipeline detecting MEV and scoring DEX pool health from live Uniswap V2 events.

## Commands

Three independent toolchains, no root build/test.

**evm-ingester** (Rust)
- Build: `cd evm-ingester && cargo build`
- Test: `cd evm-ingester && cargo test`
- Lint: `cd evm-ingester && cargo fmt --check && cargo clippy -- -D warnings` ← run before finishing

**aggregator** (Java 21 / Flink, Maven)
- Build: `cd aggregator && mvn -DskipTests package`
- Test: `cd aggregator && mvn test`
- Lint: none — Checkstyle commented out in `pom.xml`, don't re-enable

**analytics-service** (Kotlin / Ktor, Gradle)
- Build: `cd analytics-service && ./gradlew build`
- Test: `cd analytics-service && ./gradlew test`
- Lint: `cd analytics-service && ./gradlew ktlintCheck` ← run before finishing

Full stack: `docker compose up -d --build`

## Architecture

`evm-ingester/` (Rust) → `aggregator/` (Flink/Java) → `analytics-service/` (Kotlin/Ktor), connected by Kafka with Dapr
pub/sub. Cross-component contracts are Avro schemas and Kafka topic names. See `ARCHITECTURE.md` for diagram,
`DATA_MODEL.md` for schemas.

Invariants:

- Pool identity is `(chain, pairAddress)`, never bare `pairAddress`
- Enrichment (Chainlink price, token symbols) happens in ingester at ingest time for deterministic Flink replay
- Aggregator validates CloudEvent `type` field before Avro decode — liquidity topic carries mixed schemas
  (Mint/Burn/Transfer)

## Conventions

- Comment only the *why*, never the *what*
- Pure functions in core; side effects (RPC, Kafka, Redis) at edges
- Cross-cutting domain vocabulary lives in `CONTEXT.md` — check before introducing new terms
- Component-specific conventions in `evm-ingester/AGENT.md`, `aggregator/AGENT.md`, `analytics-service/AGENT.md`

### Configuration

- No defaults for required config — fail fast at startup, no `.unwrap_or()` fallbacks
- No redundant validation — trust type system and stdlib

### Testing

- Assert behavior, not data — test algorithms (gap detection, precedence), not stdlib (string parsing, HashMap)
- Minimal mocking — only mock what's needed for the specific behavior

## Workflow

- Small commits, plain imperative messages (match history: `Add Kotlin analytics-service`)
- Ask before adding dependencies; specify which ecosystem (`Cargo.toml`, `pom.xml`, `build.gradle.kts`)
- If doc and code disagree, surface the conflict before changing either
- After each task, update `SESSION-HANDOFF.md` with current state and what's next

### Carrying context across sessions

- `PLAN.md` (root, create only for multi-session work): plan with checkboxes, delete when done
- Decision records go in `ARCHITECTURE.md#design-decisions` — don't start a parallel log
- `SESSION-HANDOFF.md` (root): current state + next steps, read at session start
- Git history: small commits make `git log`/`git diff` double as context
