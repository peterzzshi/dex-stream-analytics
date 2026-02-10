# Architecture

## System Overview

The system processes blockchain swap events through a streaming pipeline:

```
Polygon QuickSwap → Ingester → Kafka → Aggregator → Kafka → API → Users
```

## Components

### Ingester

**Folder:** `ingester/`  
**Technology:** Go + DAPR  
**Purpose:** Listens to blockchain events via WebSocket and publishes to Kafka

Key responsibilities:
- Connect to Polygon RPC
- Subscribe to QuickSwap swap events
- Transform events to Avro format
- Publish to Kafka via DAPR

### Aggregator

**Folder:** `aggregator/`  
**Technology:** Apache Flink 2.0.1 + Java 17  
**Purpose:** Windowed stream processing and aggregation

Key responsibilities:
- Consume events from Kafka (native connector)
- Apply 5-minute tumbling windows
- Calculate TWAP, volume, and volatility
- Detect patterns (arbitrage, whale activity)
- Publish to Kafka (native connector)

**Note:** Uses native Kafka connector (not DAPR) to preserve exactly-once semantics and checkpointing.

### API

**Folder:** `api/`  
**Technology:** Go + DAPR  
**Purpose:** REST API exposing analytics data

Key responsibilities:
- Consume aggregated analytics from Kafka via DAPR
- Store in memory
- Expose REST endpoints
- Health checks

## Data Flow

1. Blockchain events captured by Ingester
2. Events serialised to Avro and published to `dex-events` topic
3. Aggregator consumes events and applies 5-minute windows
4. Aggregated analytics published to `dex-analytics` topic
5. API consumes analytics and exposes via REST

## Infrastructure

- **Kafka** (KRaft mode) - Message broker
- **Schema Registry** - Avro schema management
- **DAPR** - Service mesh for Ingester and API
- **Flink** - Stream processing engine
- **Docker Compose** - Local orchestration

## Key Design Decisions

- Flink uses native Kafka connectors for exactly-once semantics
- DAPR used only for Ingester and API (not Flink)
- 5-minute tumbling windows for aggregation
- Event-time processing with watermarks
- Avro for schema serialisation
