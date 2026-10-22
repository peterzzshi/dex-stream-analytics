package com.web3analytics.serde;

import static org.assertj.core.api.Assertions.assertThat;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.TransferEvent;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import org.apache.avro.Schema;
import org.apache.avro.generic.GenericData;
import org.apache.avro.generic.GenericDatumWriter;
import org.apache.avro.io.BinaryEncoder;
import org.apache.avro.io.EncoderFactory;
import org.apache.flink.util.Collector;
import org.junit.jupiter.api.Test;

class LiquidityEventDeserializerTest {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

  private static byte[] wrapInCloudEvent(final byte[] payload, final String eventType) {
    try {
      final ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
      envelope.put("specversion", "1.0");
      envelope.put("type", eventType);
      envelope.put("source", "ingester");
      envelope.put("id", "evt-1");
      envelope.put("datacontenttype", "application/avro-binary");
      envelope.put("data_base64", Base64.getEncoder().encodeToString(payload));
      return OBJECT_MAPPER.writeValueAsString(envelope).getBytes(StandardCharsets.UTF_8);
    } catch (final IOException e) {
      throw new RuntimeException("Failed to wrap in CloudEvent", e);
    }
  }

  private static byte[] buildMintPayload() {
    try {
      final Schema schema = loadSchema("/avro/MintEvent.avsc");
      final GenericData.Record avroRecord = new GenericData.Record(schema);

      avroRecord.put("eventId", "tx-1:0");
      avroRecord.put("blockNumber", 123L);
      avroRecord.put("blockTimestamp", 1700000000L);
      avroRecord.put("transactionHash", "0xtx");
      avroRecord.put("logIndex", 0);
      avroRecord.put("chain", "eip155:1");
      avroRecord.put("chainName", "Ethereum");
      avroRecord.put("pairAddress", "0xpair");
      avroRecord.put("token0", "0xtoken0");
      avroRecord.put("token1", "0xtoken1");
      avroRecord.put("token0Symbol", "WMATIC");
      avroRecord.put("token1Symbol", "USDC");
      avroRecord.put("sender", "0xsender");
      avroRecord.put("amount0", "1000");
      avroRecord.put("amount1", "1230");
      avroRecord.put("eventTimestamp", 1700000001L);

      return encode(schema, avroRecord);
    } catch (final IOException e) {
      throw new RuntimeException("Failed to build test payload", e);
    }
  }

  private static byte[] buildBurnPayload() {
    try {
      final Schema schema = loadSchema("/avro/BurnEvent.avsc");
      final GenericData.Record avroRecord = new GenericData.Record(schema);

      avroRecord.put("eventId", "tx-1:0");
      avroRecord.put("blockNumber", 123L);
      avroRecord.put("blockTimestamp", 1700000000L);
      avroRecord.put("transactionHash", "0xtx");
      avroRecord.put("logIndex", 0);
      avroRecord.put("chain", "eip155:1");
      avroRecord.put("chainName", "Ethereum");
      avroRecord.put("pairAddress", "0xpair");
      avroRecord.put("token0", "0xtoken0");
      avroRecord.put("token1", "0xtoken1");
      avroRecord.put("token0Symbol", "WMATIC");
      avroRecord.put("token1Symbol", "USDC");
      avroRecord.put("sender", "0xsender");
      avroRecord.put("recipient", "0xrecipient");
      avroRecord.put("amount0", "1000");
      avroRecord.put("amount1", "1230");
      avroRecord.put("eventTimestamp", 1700000001L);

      return encode(schema, avroRecord);
    } catch (final IOException e) {
      throw new RuntimeException("Failed to build test payload", e);
    }
  }

  private static byte[] buildTransferPayload() {
    try {
      final Schema schema = loadSchema("/avro/TransferEvent.avsc");
      final GenericData.Record avroRecord = new GenericData.Record(schema);

      avroRecord.put("eventId", "tx-1:1");
      avroRecord.put("blockNumber", 123L);
      avroRecord.put("blockTimestamp", 1700000000L);
      avroRecord.put("transactionHash", "0xtx");
      avroRecord.put("logIndex", 1);
      avroRecord.put("chain", "eip155:1");
      avroRecord.put("chainName", "Ethereum");
      avroRecord.put("pairAddress", "0xpair");
      avroRecord.put("token0", "0xtoken0");
      avroRecord.put("token1", "0xtoken1");
      avroRecord.put("token0Symbol", "WMATIC");
      avroRecord.put("token1Symbol", "USDC");
      avroRecord.put("from", "0xfrom");
      avroRecord.put("to", "0xto");
      avroRecord.put("value", "777");
      avroRecord.put("eventTimestamp", 1700000001L);

      return encode(schema, avroRecord);
    } catch (final IOException e) {
      throw new RuntimeException("Failed to build test payload", e);
    }
  }

  private static byte[] encode(final Schema schema, final GenericData.Record avroRecord)
      throws IOException {
    final ByteArrayOutputStream out = new ByteArrayOutputStream();
    final BinaryEncoder encoder = EncoderFactory.get().binaryEncoder(out, null);
    new GenericDatumWriter<GenericData.Record>(schema).write(avroRecord, encoder);
    encoder.flush();
    return out.toByteArray();
  }

  private static Schema loadSchema(final String path) throws IOException {
    try (final InputStream input = LiquidityEventDeserializerTest.class.getResourceAsStream(path)) {
      if (input == null) {
        throw new IOException("Schema not found: " + path);
      }
      return new Schema.Parser().parse(input);
    }
  }

    @Test
    void shouldDeserializeMintEvent() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final List<DexEvent> results = new ArrayList<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildMintPayload(), "com.dex.events.mint");

      deserializer.deserialize(cloudEvent, new Collector<>() {
        @Override
        public void collect(final DexEvent swapEvent) {
          results.add(swapEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).hasSize(1);
      assertThat(results.getFirst()).isInstanceOf(MintEvent.class);
      final MintEvent mint = (MintEvent) results.getFirst();
        assertThat(mint.sender()).isEqualTo("0xsender");
        assertThat(mint.amount0()).isEqualTo("1000");
    }

    @Test
    void shouldDeserializeBurnEvent() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final List<DexEvent> results = new ArrayList<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildBurnPayload(), "com.dex.events.burn");

      deserializer.deserialize(cloudEvent, new Collector<>() {
        @Override
        public void collect(final DexEvent swapEvent) {
          results.add(swapEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).hasSize(1);
      assertThat(results.getFirst()).isInstanceOf(BurnEvent.class);
      final BurnEvent burn = (BurnEvent) results.getFirst();
        assertThat(burn.sender()).isEqualTo("0xsender");
        assertThat(burn.recipient()).isEqualTo("0xrecipient");
        assertThat(burn.amount1()).isEqualTo("1230");
    }

    @Test
    void shouldRejectRawAvroPayload() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final List<DexEvent> results = new ArrayList<>();
      final byte[] payload = buildMintPayload();

      deserializer.deserialize(payload, new Collector<>() {
        @Override
        public void collect(final DexEvent dexEvent) {
          results.add(dexEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).isEmpty();
    }

    @Test
    void shouldDeserializeTransferEvent() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final List<DexEvent> results = new ArrayList<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildTransferPayload(), "com.dex.events.transfer");

      deserializer.deserialize(cloudEvent, new Collector<>() {
        @Override
        public void collect(final DexEvent dexEvent) {
          results.add(dexEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).hasSize(1);
      assertThat(results.getFirst()).isInstanceOf(TransferEvent.class);
      final TransferEvent transfer = (TransferEvent) results.getFirst();
        assertThat(transfer.from()).isEqualTo("0xfrom");
        assertThat(transfer.to()).isEqualTo("0xto");
        assertThat(transfer.value()).isEqualTo("777");
    }

    @Test
    void shouldRejectUnsupportedEventType() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final List<DexEvent> results = new ArrayList<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildMintPayload(), "com.dex.events.unknown");

      deserializer.deserialize(cloudEvent, new Collector<>() {
        @Override
        public void collect(final DexEvent dexEvent) {
          results.add(dexEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).isEmpty();
    }
}
