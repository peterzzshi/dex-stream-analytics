# Schema Organization

Canonical Avro schemas for the DEX Analytics platform.

## Philosophy: Embedded Schemas (No Schema Registry)

We use **embedded schemas** compiled directly into services instead of Schema Registry because:

- ✅ **Simplicity**: No external service to run or maintain
- ✅ **Version Control**: Schemas versioned alongside code changes
- ✅ **Code Review**: Schema changes go through PR review process
- ✅ **No Runtime Dependency**: Services work offline, faster builds
- ✅ **Compile-time Validation**: Catch schema mismatches before deployment
- ✅ **Single Consumer**: Only our services use these schemas

This approach is ideal for monorepo projects with coordinated deployments.

## Schema Files

### Dual-Topic Architecture

We use **dual Kafka topics by event frequency**:

- **`dex-trading-events`**: High-frequency swap events (~80-90% of total)
- **`dex-liquidity-events`**: Lower-frequency mint/burn events (~10-20% of total)

This approach:

- ✅ **Frequency separation**: Optimized processing for different velocity streams
- ✅ **Independent schemas**: Each topic contains homogeneous event types
- ✅ **Clear semantics**: Swap = trading, Mint/Burn = liquidity provisioning
- ✅ **Flink friendly**: Separate sources with different window strategies

### `avro/SwapEvent.avsc`
Token swap execution event (Uniswap V2 compatible).

**Key fields:**
- Event identification: `eventId`, `blockNumber`, `blockTimestamp`, `transactionHash`
- Pair info: `pairAddress`, `token0`, `token1`
- Swap details: `amount0In`, `amount1In`, `amount0Out`, `amount1Out`, `price`
- Gas metrics: `gasUsed`, `gasPrice`

**Analytics use case:** Trading metrics (TWAP, OHLC, volume, volatility) in 5-minute windows

**Used by:**
- Ingester (Go) - publishes to Kafka `dex-trading-events`
- Flink Aggregator (Java) - consumes for trading analytics stream

### `avro/MintEvent.avsc`
Liquidity provision event.

**Key fields:**
- Event identification: `eventId`, `blockNumber`, `blockTimestamp`, `transactionHash`
- Pair info: `pairAddress`, `token0`, `token1`
- Liquidity details: `sender`, `amount0`, `amount1`

**Analytics use case:** LP behavior analysis, liquidity depth metrics in 1-hour windows

**Used by:**
- Ingester (Go) - publishes to Kafka `dex-liquidity-events`
- Flink Aggregator (Java) - consumes for liquidity analytics stream

### `avro/BurnEvent.avsc`
Liquidity removal event.

**Key fields:**
- Event identification: `eventId`, `blockNumber`, `blockTimestamp`, `transactionHash`
- Pair info: `pairAddress`, `token0`, `token1`
- Liquidity details: `sender`, `recipient`, `amount0`, `amount1`

**Analytics use case:** LP exit patterns, liquidity risk analysis in 1-hour windows

**Used by:**
- Ingester (Go) - publishes to Kafka `dex-liquidity-events`
- Flink Aggregator (Java) - consumes for liquidity analytics stream

### `avro/AggregatedAnalytics.avsc`
Aggregated DEX metrics output from Flink stream processors.

**Trading Analytics (from Swap events, 5-min windows):**
- Window info: `windowId`, `windowStart`, `windowEnd`, `pairAddress`
- Price metrics: `twap`, `openPrice`, `closePrice`, `highPrice`, `lowPrice`, `priceVolatility`
- Volume metrics: `totalVolume0`, `totalVolume1`, `swapCount`, `uniqueTraders`
- Pattern detection: `arbitrageCount`, `repeatedTraders`, `largestSwapAddress`
- Gas metrics: `totalGasUsed`, `averageGasPrice`

**Used by:**
- Flink Aggregator (Java) - produces to Kafka `dex-trading-analytics`
- API Service (Go) - consumes from Kafka (planned)

## Usage in Services

### Ingester (Go)
Each event schema is embedded at compile time using `go:embed`:
```go
//go:embed swap_event.avsc
var swapEventSchema string

//go:embed mint_event.avsc
var mintEventSchema string

//go:embed burn_event.avsc
var burnEventSchema string

//go:embed sync_event.avsc
var syncEventSchema string
```

DAPR publisher adds schema metadata for multi-schema topic support.

### Flink Aggregator (Java)
Schemas are automatically generated as Java classes during Maven build via `avro-maven-plugin`.

**Maven configuration:**
```xml
<plugin>
    <groupId>org.apache.avro</groupId>
    <artifactId>avro-maven-plugin</artifactId>
    <configuration>
        <sourceDirectory>${project.basedir}/../schemas/avro</sourceDirectory>
        <outputDirectory>${project.build.directory}/generated-sources/avro</outputDirectory>
    </configuration>
</plugin>
```

Generated classes implement sealed interface hierarchy:
- `com.web3analytics.models.DexEvent` (sealed interface)
  - `com.web3analytics.models.SwapEvent` (record)
  - `com.web3analytics.models.SyncEvent` (record)
  - `com.web3analytics.models.MintEvent` (record)
  - `com.web3analytics.models.BurnEvent` (record)
- `com.web3analytics.models.AggregatedAnalytics` (record)

### API Service (Go)
Schema is embedded at compile time for deserializing analytics from Kafka.

## Making Schema Changes

1. **Edit schema** in `schemas/avro/[SchemaName].avsc`
2. **Rebuild services**:
   ```bash
   make build
   ```
3. **Test compatibility** with existing data in Kafka topics

## Schema Evolution Guidelines

When modifying schemas, follow Avro compatibility rules:

**Safe changes (backward compatible):**
- Add optional fields with defaults
- Remove fields (readers ignore unknown fields)

**Breaking changes (require coordination):**
- Remove required fields
- Change field types
- Rename fields

For breaking changes:
1. Deploy all services simultaneously
2. Clear Kafka topics or use new topic names
3. Update all consumers before producers
