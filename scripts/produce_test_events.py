#!/usr/bin/env python3
"""
Produce test CloudEvent messages (Avro binary in data_base64) to Kafka
for e2e verification of the aggregator pipeline.

Produces:
- 10 SwapEvents to dex-trading-events (same pair, timestamps within 5-min window)
- 2 MintEvents + 1 BurnEvent + 2 TransferEvents to dex-liquidity-events
"""
import base64
import io
import json
import time
import uuid

import fastavro
from kafka import KafkaProducer

KAFKA_BOOTSTRAP = "localhost:29092"

# --- Avro schemas (inline, matching schemas/avro/*.avsc) ---
SWAP_SCHEMA = fastavro.parse_schema({
    "namespace": "com.web3analytics.events",
    "type": "record",
    "name": "SwapEvent",
    "fields": [
        {"name": "eventId", "type": "string"},
        {"name": "blockNumber", "type": "long"},
        {"name": "blockTimestamp", "type": "long"},
        {"name": "transactionHash", "type": "string"},
        {"name": "logIndex", "type": "int"},
        {"name": "pairAddress", "type": "string"},
        {"name": "token0", "type": "string"},
        {"name": "token1", "type": "string"},
        {"name": "token0Symbol", "type": ["null", "string"], "default": None},
        {"name": "token1Symbol", "type": ["null", "string"], "default": None},
        {"name": "sender", "type": "string"},
        {"name": "recipient", "type": "string"},
        {"name": "amount0In", "type": "string"},
        {"name": "amount1In", "type": "string"},
        {"name": "amount0Out", "type": "string"},
        {"name": "amount1Out", "type": "string"},
        {"name": "price", "type": "double"},
        {"name": "volumeUSD", "type": ["null", "double"], "default": None},
        {"name": "gasUsed", "type": "long"},
        {"name": "gasPrice", "type": "string"},
        {"name": "eventTimestamp", "type": "long"},
    ]
})

MINT_SCHEMA = fastavro.parse_schema({
    "namespace": "com.web3analytics.events",
    "type": "record",
    "name": "MintEvent",
    "fields": [
        {"name": "eventId", "type": "string"},
        {"name": "blockNumber", "type": "long"},
        {"name": "blockTimestamp", "type": "long"},
        {"name": "transactionHash", "type": "string"},
        {"name": "logIndex", "type": "int"},
        {"name": "pairAddress", "type": "string"},
        {"name": "token0", "type": "string"},
        {"name": "token1", "type": "string"},
        {"name": "token0Symbol", "type": ["null", "string"], "default": None},
        {"name": "token1Symbol", "type": ["null", "string"], "default": None},
        {"name": "sender", "type": "string"},
        {"name": "amount0", "type": "string"},
        {"name": "amount1", "type": "string"},
        {"name": "eventTimestamp", "type": "long"},
    ]
})

BURN_SCHEMA = fastavro.parse_schema({
    "namespace": "com.web3analytics.events",
    "type": "record",
    "name": "BurnEvent",
    "fields": [
        {"name": "eventId", "type": "string"},
        {"name": "blockNumber", "type": "long"},
        {"name": "blockTimestamp", "type": "long"},
        {"name": "transactionHash", "type": "string"},
        {"name": "logIndex", "type": "int"},
        {"name": "pairAddress", "type": "string"},
        {"name": "token0", "type": "string"},
        {"name": "token1", "type": "string"},
        {"name": "token0Symbol", "type": ["null", "string"], "default": None},
        {"name": "token1Symbol", "type": ["null", "string"], "default": None},
        {"name": "sender", "type": "string"},
        {"name": "to", "type": "string"},
        {"name": "amount0", "type": "string"},
        {"name": "amount1", "type": "string"},
        {"name": "eventTimestamp", "type": "long"},
    ]
})

TRANSFER_SCHEMA = fastavro.parse_schema({
    "namespace": "com.web3analytics.events",
    "type": "record",
    "name": "TransferEvent",
    "fields": [
        {"name": "eventId", "type": "string"},
        {"name": "blockNumber", "type": "long"},
        {"name": "blockTimestamp", "type": "long"},
        {"name": "transactionHash", "type": "string"},
        {"name": "logIndex", "type": "int"},
        {"name": "pairAddress", "type": "string"},
        {"name": "token0", "type": "string"},
        {"name": "token1", "type": "string"},
        {"name": "token0Symbol", "type": ["null", "string"], "default": None},
        {"name": "token1Symbol", "type": ["null", "string"], "default": None},
        {"name": "from", "type": "string"},
        {"name": "to", "type": "string"},
        {"name": "value", "type": "string"},
        {"name": "eventTimestamp", "type": "long"},
    ]
})


def avro_encode(schema, record):
    buf = io.BytesIO()
    fastavro.schemaless_writer(buf, schema, record)
    return buf.getvalue()


def cloud_event(event_type, avro_bytes, source="test-producer"):
    return json.dumps({
        "specversion": "1.0",
        "id": str(uuid.uuid4()),
        "type": event_type,
        "source": source,
        "datacontenttype": "application/avro",
        "data_base64": base64.b64encode(avro_bytes).decode("ascii"),
    }).encode("utf-8")


def main():
    producer = KafkaProducer(bootstrap_servers=KAFKA_BOOTSTRAP)

    pair = "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827"
    token0 = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"  # WMATIC
    token1 = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"  # USDC

    # --- Produce 10 SwapEvents (within same 5-min window) ---
    base_ts = 1700000000  # Nov 14 2023 ~22:13 UTC (seconds)
    print("Producing 10 SwapEvents to dex-trading-events...")
    for i in range(10):
        ts = base_ts + i * 20  # 20s apart
        swap = {
            "eventId": f"swap-{i}",
            "blockNumber": 50000000 + i,
            "blockTimestamp": ts,
            "transactionHash": f"0xtx{i:04d}",
            "logIndex": 0,
            "pairAddress": pair,
            "token0": token0,
            "token1": token1,
            "token0Symbol": "WMATIC",
            "token1Symbol": "USDC",
            "sender": f"0xsender{i:04d}",
            "recipient": f"0xrecipient{i:04d}",
            "amount0In": str(1000000000000000000 * (i + 1)),  # 1-10 WMATIC
            "amount1In": "0",
            "amount0Out": "0",
            "amount1Out": str(800000 * (i + 1)),  # 0.8-8 USDC
            "price": 0.8 + i * 0.01,
            "volumeUSD": 0.8 * (i + 1),
            "gasUsed": 150000,
            "gasPrice": "30000000000",
            "eventTimestamp": ts * 1000,  # ms
        }
        payload = cloud_event("com.dex.events.swap", avro_encode(SWAP_SCHEMA, swap))
        producer.send("dex-trading-events", value=payload)
        print(f"  Sent swap-{i} (ts={ts})")

    # --- Produce liquidity events (within same 1-hour window) ---
    print("\nProducing liquidity events to dex-liquidity-events...")

    # Mint 1 (tx: 0xtxmint1)
    mint1 = {
        "eventId": "mint-1",
        "blockNumber": 50000100,
        "blockTimestamp": base_ts + 60,
        "transactionHash": "0xtxmint1",
        "logIndex": 0,
        "pairAddress": pair,
        "token0": token0,
        "token1": token1,
        "token0Symbol": "WMATIC",
        "token1Symbol": "USDC",
        "sender": "0xAlice",
        "amount0": "5000000000000000000",
        "amount1": "4000000",
        "eventTimestamp": (base_ts + 60) * 1000,
    }
    producer.send("dex-liquidity-events",
                  value=cloud_event("com.dex.events.mint", avro_encode(MINT_SCHEMA, mint1)))
    print("  Sent mint-1 (Alice, tx=0xtxmint1)")

    # Transfer for mint1 (LP tokens minted: from=0x0)
    transfer1 = {
        "eventId": "transfer-1",
        "blockNumber": 50000100,
        "blockTimestamp": base_ts + 60,
        "transactionHash": "0xtxmint1",
        "logIndex": 1,
        "pairAddress": pair,
        "token0": token0,
        "token1": token1,
        "token0Symbol": "WMATIC",
        "token1Symbol": "USDC",
        "from": "0x0000000000000000000000000000000000000000",
        "to": "0xAlice",
        "value": "4472135954",
        "eventTimestamp": (base_ts + 60) * 1000,
    }
    producer.send("dex-liquidity-events",
                  value=cloud_event("com.dex.events.transfer", avro_encode(TRANSFER_SCHEMA, transfer1)))
    print("  Sent transfer-1 (LP mint, tx=0xtxmint1, value=4472135954)")

    # Mint 2 (tx: 0xtxmint2)
    mint2 = {
        "eventId": "mint-2",
        "blockNumber": 50000200,
        "blockTimestamp": base_ts + 120,
        "transactionHash": "0xtxmint2",
        "logIndex": 0,
        "pairAddress": pair,
        "token0": token0,
        "token1": token1,
        "token0Symbol": "WMATIC",
        "token1Symbol": "USDC",
        "sender": "0xBob",
        "amount0": "2000000000000000000",
        "amount1": "1600000",
        "eventTimestamp": (base_ts + 120) * 1000,
    }
    producer.send("dex-liquidity-events",
                  value=cloud_event("com.dex.events.mint", avro_encode(MINT_SCHEMA, mint2)))
    print("  Sent mint-2 (Bob, tx=0xtxmint2)")

    # Transfer for mint2 (LP tokens minted: from=0x0)
    transfer2 = {
        "eventId": "transfer-2",
        "blockNumber": 50000200,
        "blockTimestamp": base_ts + 120,
        "transactionHash": "0xtxmint2",
        "logIndex": 1,
        "pairAddress": pair,
        "token0": token0,
        "token1": token1,
        "token0Symbol": "WMATIC",
        "token1Symbol": "USDC",
        "from": "0x0000000000000000000000000000000000000000",
        "to": "0xBob",
        "value": "1788854381",
        "eventTimestamp": (base_ts + 120) * 1000,
    }
    producer.send("dex-liquidity-events",
                  value=cloud_event("com.dex.events.transfer", avro_encode(TRANSFER_SCHEMA, transfer2)))
    print("  Sent transfer-2 (LP mint, tx=0xtxmint2, value=1788854381)")

    # Burn 1 (tx: 0xtxburn1) - Alice removes some liquidity
    burn1 = {
        "eventId": "burn-1",
        "blockNumber": 50000300,
        "blockTimestamp": base_ts + 180,
        "transactionHash": "0xtxburn1",
        "logIndex": 0,
        "pairAddress": pair,
        "token0": token0,
        "token1": token1,
        "token0Symbol": "WMATIC",
        "token1Symbol": "USDC",
        "sender": "0xAlice",
        "to": "0xAlice",
        "amount0": "1000000000000000000",
        "amount1": "800000",
        "eventTimestamp": (base_ts + 180) * 1000,
    }
    producer.send("dex-liquidity-events",
                  value=cloud_event("com.dex.events.burn", avro_encode(BURN_SCHEMA, burn1)))
    print("  Sent burn-1 (Alice, tx=0xtxburn1)")

    # Transfer for burn1 (LP tokens burned: to=0x0)
    transfer3 = {
        "eventId": "transfer-3",
        "blockNumber": 50000300,
        "blockTimestamp": base_ts + 180,
        "transactionHash": "0xtxburn1",
        "logIndex": 1,
        "pairAddress": pair,
        "token0": token0,
        "token1": token1,
        "token0Symbol": "WMATIC",
        "token1Symbol": "USDC",
        "from": "0xAlice",
        "to": "0x0000000000000000000000000000000000000000",
        "value": "894427190",
        "eventTimestamp": (base_ts + 180) * 1000,
    }
    producer.send("dex-liquidity-events",
                  value=cloud_event("com.dex.events.transfer", avro_encode(TRANSFER_SCHEMA, transfer3)))
    print("  Sent transfer-3 (LP burn, tx=0xtxburn1, value=894427190)")

    producer.flush()
    print(f"\nAll events produced successfully!")
    print(f"  Trading: 10 SwapEvents (5-min window starting at {base_ts})")
    print(f"  Liquidity: 2 Mints, 1 Burn, 3 Transfers (1-hour window)")
    print(f"  Expected LP tokens minted: 4472135954 + 1788854381 = 6260990335")
    print(f"  Expected LP tokens burned: 894427190")
    print(f"  Expected net LP change: 5366563145")


if __name__ == "__main__":
    main()
