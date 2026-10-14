package com.web3analytics.serde;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.errors.DeserializationException;
import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.TransferEvent;
import org.apache.avro.Schema;
import org.apache.avro.generic.GenericData;
import org.apache.avro.generic.GenericDatumWriter;
import org.apache.avro.io.BinaryEncoder;
import org.apache.avro.io.EncoderFactory;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Base64;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class LiquidityEventDeserializerTest {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    @Test
    void shouldDeserializeMintEvent() throws Exception {
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
        byte[] cloudEvent = wrapInCloudEvent(buildMintPayload(), "com.dex.events.mint");

        DexEvent event = deserializer.deserialize(cloudEvent);

        assertThat(event).isInstanceOf(MintEvent.class);
        MintEvent mint = (MintEvent) event;
        assertThat(mint.sender()).isEqualTo("0xsender");
        assertThat(mint.amount0()).isEqualTo("1000");
    }

    @Test
    void shouldDeserializeBurnEvent() throws Exception {
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
        byte[] cloudEvent = wrapInCloudEvent(buildBurnPayload(), "com.dex.events.burn");

        DexEvent event = deserializer.deserialize(cloudEvent);

        assertThat(event).isInstanceOf(BurnEvent.class);
        BurnEvent burn = (BurnEvent) event;
        assertThat(burn.sender()).isEqualTo("0xsender");
        assertThat(burn.recipient()).isEqualTo("0xrecipient");
        assertThat(burn.amount1()).isEqualTo("1230");
    }

    @Test
    void shouldRejectRawAvroPayload() throws Exception {
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();

        assertThatThrownBy(() -> deserializer.deserialize(buildMintPayload()))
                .isInstanceOf(DeserializationException.class)
                .hasMessageContaining("Expected CloudEvent JSON envelope");
    }

    @Test
    void shouldDeserializeTransferEvent() throws Exception {
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
        byte[] cloudEvent = wrapInCloudEvent(buildTransferPayload(), "com.dex.events.transfer");

        DexEvent event = deserializer.deserialize(cloudEvent);

        assertThat(event).isInstanceOf(TransferEvent.class);
        TransferEvent transfer = (TransferEvent) event;
        assertThat(transfer.from()).isEqualTo("0xfrom");
        assertThat(transfer.to()).isEqualTo("0xto");
        assertThat(transfer.value()).isEqualTo("777");
    }

    @Test
    void shouldRejectUnsupportedEventType() throws Exception {
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();
        byte[] cloudEvent = wrapInCloudEvent(buildMintPayload(), "com.dex.events.unknown");

        assertThatThrownBy(() -> deserializer.deserialize(cloudEvent))
                .isInstanceOf(DeserializationException.class)
                .hasMessageContaining("Unsupported liquidity CloudEvent type");
    }

    private static byte[] wrapInCloudEvent(byte[] payload, String eventType) throws IOException {
        ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", eventType);
        envelope.put("source", "ingester");
        envelope.put("id", "evt-1");
        envelope.put("datacontenttype", "application/avro-binary");
        envelope.put("data_base64", Base64.getEncoder().encodeToString(payload));
        return OBJECT_MAPPER.writeValueAsString(envelope).getBytes(StandardCharsets.UTF_8);
    }

    private static byte[] buildMintPayload() throws IOException {
        Schema schema = loadSchema("/avro/MintEvent.avsc");
        GenericData.Record record = new GenericData.Record(schema);

        record.put("eventId", "tx-1:0");
        record.put("blockNumber", 123L);
        record.put("blockTimestamp", 1700000000L);
        record.put("transactionHash", "0xtx");
        record.put("logIndex", 0);
        record.put("pairAddress", "0xpair");
        record.put("token0", "0xtoken0");
        record.put("token1", "0xtoken1");
        record.put("token0Symbol", "WMATIC");
        record.put("token1Symbol", "USDC");
        record.put("sender", "0xsender");
        record.put("amount0", "1000");
        record.put("amount1", "1230");
        record.put("eventTimestamp", 1700000001L);

        return encode(schema, record);
    }

    private static byte[] buildBurnPayload() throws IOException {
        Schema schema = loadSchema("/avro/BurnEvent.avsc");
        GenericData.Record record = new GenericData.Record(schema);

        record.put("eventId", "tx-1:0");
        record.put("blockNumber", 123L);
        record.put("blockTimestamp", 1700000000L);
        record.put("transactionHash", "0xtx");
        record.put("logIndex", 0);
        record.put("pairAddress", "0xpair");
        record.put("token0", "0xtoken0");
        record.put("token1", "0xtoken1");
        record.put("token0Symbol", "WMATIC");
        record.put("token1Symbol", "USDC");
        record.put("sender", "0xsender");
        record.put("recipient", "0xrecipient");
        record.put("amount0", "1000");
        record.put("amount1", "1230");
        record.put("eventTimestamp", 1700000001L);

        return encode(schema, record);
    }

    private static byte[] buildTransferPayload() throws IOException {
        Schema schema = loadSchema("/avro/TransferEvent.avsc");
        GenericData.Record record = new GenericData.Record(schema);

        record.put("eventId", "tx-1:1");
        record.put("blockNumber", 123L);
        record.put("blockTimestamp", 1700000000L);
        record.put("transactionHash", "0xtx");
        record.put("logIndex", 1);
        record.put("pairAddress", "0xpair");
        record.put("token0", "0xtoken0");
        record.put("token1", "0xtoken1");
        record.put("token0Symbol", "WMATIC");
        record.put("token1Symbol", "USDC");
        record.put("from", "0xfrom");
        record.put("to", "0xto");
        record.put("value", "777");
        record.put("eventTimestamp", 1700000001L);

        return encode(schema, record);
    }

    private static byte[] encode(Schema schema, GenericData.Record record) throws IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        BinaryEncoder encoder = EncoderFactory.get().binaryEncoder(out, null);
        new GenericDatumWriter<GenericData.Record>(schema).write(record, encoder);
        encoder.flush();
        return out.toByteArray();
    }

    private static Schema loadSchema(String path) throws IOException {
        try (InputStream input = LiquidityEventDeserializerTest.class.getResourceAsStream(path)) {
            if (input == null) {
                throw new IOException("Schema not found: " + path);
            }
            return new Schema.Parser().parse(input);
        }
    }
}
