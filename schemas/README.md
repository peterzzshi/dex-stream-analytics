# Schemas

Canonical Avro schemas for the pipeline live in `schemas/avro/`.

## Scope

- Define wire contracts between `ingester`, `aggregator`, and `api`.
- Keep schema evolution explicit and version-controlled in this repo.
- Avoid runtime schema-registry dependency for local and interview-friendly setup.

## Active Schemas

- `SwapEvent.avsc` -> `dex-trading-events`
- `MintEvent.avsc` -> `dex-liquidity-events`
- `BurnEvent.avsc` -> `dex-liquidity-events`
- `TransferEvent.avsc` -> `dex-liquidity-events`
- `AggregatedAnalytics.avsc` -> `dex-trading-analytics`

## Contract Notes

- Ingest path uses DAPR pub/sub, so Kafka values arrive wrapped as CloudEvents.
- Event type metadata (CloudEvent `type`) should select schema before payload decode.
- `schemas/avro/*.avsc` is the source of truth for payload shape.

## Change Workflow

1. Update the schema file in `schemas/avro/`.
2. Update dependent decoders/encoders in `ingester` and `aggregator`.
3. Run service tests and integration checks against Kafka topics.
4. Document compatibility impact in PR notes.

## Evolution Rules

Safe changes:
- Add optional fields with defaults.
- Add new record types on new topics.

Breaking changes:
- Remove required fields.
- Rename fields.
- Change field types.

For breaking changes, coordinate producer and consumer updates together.
