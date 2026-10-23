package com.web3analytics.serde;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.errors.DeserializationException;
import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.TransferEvent;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.apache.avro.Schema;
import org.apache.avro.generic.GenericData;
import org.apache.avro.generic.GenericDatumWriter;
import org.apache.avro.io.BinaryEncoder;
import org.apache.avro.io.EncoderFactory;
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
    return buildMintPayload("1000", "1230");
  }

  private static byte[] buildMintPayload(final String amount0, final String amount1) {
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
      avroRecord.put("amount0", amount0);
      avroRecord.put("amount1", amount1);
      avroRecord.put("eventTimestamp", 1700000001L);

      return encode(schema, avroRecord);
    } catch (final IOException e) {
      throw new RuntimeException("Failed to build test payload", e);
    }
  }

  private static byte[] buildBurnPayload() {
    return buildBurnPayload("1000", "1230");
  }

  private static byte[] buildBurnPayload(final String amount0, final String amount1) {
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
      avroRecord.put("amount0", amount0);
      avroRecord.put("amount1", amount1);
      avroRecord.put("eventTimestamp", 1700000001L);

      return encode(schema, avroRecord);
    } catch (final IOException e) {
      throw new RuntimeException("Failed to build test payload", e);
    }
  }

  private static byte[] buildTransferPayload() {
    return buildTransferPayload("777");
  }

  private static byte[] buildTransferPayload(final String value) {
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
      avroRecord.put("value", value);
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
      final ListCollector<DexEvent> collector = new ListCollector<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildMintPayload(), "com.dex.events.mint");

      deserializer.deserialize(cloudEvent, collector);

      assertThat(collector.items()).hasSize(1);
      assertThat(collector.items().getFirst()).isInstanceOf(MintEvent.class);
      final MintEvent mint = (MintEvent) collector.items().getFirst();
        assertThat(mint.sender()).isEqualTo("0xsender");
        assertThat(mint.amount0()).isEqualTo("1000");
    }

    @Test
    void shouldDeserializeBurnEvent() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final ListCollector<DexEvent> collector = new ListCollector<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildBurnPayload(), "com.dex.events.burn");

      deserializer.deserialize(cloudEvent, collector);

      assertThat(collector.items()).hasSize(1);
      assertThat(collector.items().getFirst()).isInstanceOf(BurnEvent.class);
      final BurnEvent burn = (BurnEvent) collector.items().getFirst();
        assertThat(burn.sender()).isEqualTo("0xsender");
        assertThat(burn.recipient()).isEqualTo("0xrecipient");
        assertThat(burn.amount1()).isEqualTo("1230");
    }

    @Test
    void shouldRejectRawAvroPayload() {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final ListCollector<DexEvent> collector = new ListCollector<>();
      final byte[] payload = buildMintPayload();

      assertThatThrownBy(() -> deserializer.deserialize(payload, collector))
          .isInstanceOf(IOException.class);

      assertThat(collector.items()).isEmpty();
    }

    @Test
    void shouldDeserializeTransferEvent() throws Exception {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final ListCollector<DexEvent> collector = new ListCollector<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildTransferPayload(), "com.dex.events.transfer");

      deserializer.deserialize(cloudEvent, collector);

      assertThat(collector.items()).hasSize(1);
      assertThat(collector.items().getFirst()).isInstanceOf(TransferEvent.class);
      final TransferEvent transfer = (TransferEvent) collector.items().getFirst();
        assertThat(transfer.from()).isEqualTo("0xfrom");
        assertThat(transfer.to()).isEqualTo("0xto");
        assertThat(transfer.value()).isEqualTo("777");
    }

    @Test
    void shouldRejectUnsupportedEventType() {
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
      final ListCollector<DexEvent> collector = new ListCollector<>();
      final byte[] cloudEvent = wrapInCloudEvent(buildMintPayload(), "com.dex.events.unknown");

      assertThatThrownBy(() -> deserializer.deserialize(cloudEvent, collector))
          .isInstanceOf(IOException.class);

      assertThat(collector.items()).isEmpty();
    }

  // -- deserializeAvro(): field mapping and amount validation, no CloudEvent needed --

  @Test
  void mintDeserializeAvroRejectsNegativeAmount() {
    final MintEventDeserializer deserializer = new MintEventDeserializer();
    final byte[] payload = buildMintPayload("-1", "1230");

    assertThatThrownBy(() -> deserializer.deserializeAvro(payload))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("amount0")
        .hasMessageContaining("negative");
  }

  @Test
  void mintDeserializeAvroRejectsAllZeroAmounts() {
    final MintEventDeserializer deserializer = new MintEventDeserializer();
    final byte[] payload = buildMintPayload("0", "0");

    assertThatThrownBy(() -> deserializer.deserializeAvro(payload))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("MintEvent")
        .hasMessageContaining("no non-zero amount field");
  }

  @Test
  void burnDeserializeAvroRejectsNegativeAmount() {
    final BurnEventDeserializer deserializer = new BurnEventDeserializer();
    final byte[] payload = buildBurnPayload("1000", "-1");

    assertThatThrownBy(() -> deserializer.deserializeAvro(payload))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("amount1")
        .hasMessageContaining("negative");
  }

  @Test
  void burnDeserializeAvroRejectsAllZeroAmounts() {
    final BurnEventDeserializer deserializer = new BurnEventDeserializer();
    final byte[] payload = buildBurnPayload("0", "0");

    assertThatThrownBy(() -> deserializer.deserializeAvro(payload))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("BurnEvent")
        .hasMessageContaining("no non-zero amount field");
  }

  @Test
  void transferDeserializeAvroRejectsNegativeAmount() {
    final TransferEventDeserializer deserializer = new TransferEventDeserializer();
    final byte[] payload = buildTransferPayload("-777");

    assertThatThrownBy(() -> deserializer.deserializeAvro(payload))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("value")
        .hasMessageContaining("negative");
  }

  @Test
  void transferDeserializeAvroRejectsZeroAmount() {
    final TransferEventDeserializer deserializer = new TransferEventDeserializer();
    final byte[] payload = buildTransferPayload("0");

    assertThatThrownBy(() -> deserializer.deserializeAvro(payload))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("TransferEvent")
        .hasMessageContaining("no non-zero amount field");
  }
}
