# AI Session Prompt

Guidelines for AI development sessions on this repo.

## Orientation

Real-time DEX analytics pipeline: Go ingester → Kafka → Flink aggregator → Kafka → Kotlin analytics-service.
See `project-context.md` for current status and decisions.

## Principles

- Favour practical delivery over speculative architecture.
- Do not duplicate content already in `ARCHITECTURE.md`, `DATA_MODEL.md`, or service READMEs.
- Update the most specific doc (e.g. service README) rather than a top-level doc.

## Non-Negotiables

- Avro is the canonical data contract.
- Dapr at service edges (ingester + analytics-service); Flink uses native Kafka connector.
- Aggregator prioritises correctness and contract clarity over feature sprawl.

## Documentation Boundaries

| File | Scope |
|------|-------|
| `README.md` | What it does, how to run |
| `ARCHITECTURE.md` | Design decisions, tradeoffs, infrastructure |
| `DATA_MODEL.md` | Field semantics, schema evolution |
| Service READMEs | Service-specific behaviour and config |
| `.ai-context/*` | AI-only guidance, NOT committed to Git |
