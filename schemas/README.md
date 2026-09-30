# Schemas

Canonical Avro schemas for the pipeline live in `schemas/avro/`.

## Scope

- Define wire contracts between `ingester`, `aggregator`, and `analytics-service`.
- Keep schema evolution explicit and version-controlled in this repo.
- Avoid runtime schema-registry dependency for a simple local setup.

## Active Schemas

| Schema | Topic | Producer | Consumer |
|---|---|---|---|
| `SwapEvent.avsc` | `dex-trading-events` | ingester | aggregator |
| `MintEvent.avsc` | `dex-liquidity-events` | ingester | aggregator |
| `BurnEvent.avsc` | `dex-liquidity-events` | ingester | aggregator |
| `TransferEvent.avsc` | `dex-liquidity-events` | ingester | aggregator |

All four output topics (`dex-trading-analytics`, `dex-liquidity-analytics`, `dex-pattern-analytics`, `dex-market-trends`) are plain JSON, produced by the aggregator's `JsonSerializationSchema`. `AggregatedAnalytics.avsc` documents the trading-analytics shape only — it is not used on the wire. The analytics-service consumes outputs via Dapr subscriptions with `rawPayload: true` (no CloudEvent envelope).

## Contract Notes

- Input topic values arrive wrapped as Dapr CloudEvents (Avro binary in `data_base64`).
- CloudEvent `type` selects the schema before payload decode.
- `schemas/avro/*.avsc` is the source of truth for Avro payload shape.
- Field semantics are documented in `DATA_MODEL.md`.

## Change Workflow

1. Update the schema file in `schemas/avro/`.
2. Update dependent decoders/encoders in `ingester` and `aggregator`.
3. Run service tests and integration checks against Kafka topics.
4. Document compatibility impact in PR notes.

## Evolution Rules

**Safe changes:** add optional fields with defaults, add new record types on new topics.

**Breaking changes:** remove/rename fields, change field types. Coordinate producer and consumer updates together.
