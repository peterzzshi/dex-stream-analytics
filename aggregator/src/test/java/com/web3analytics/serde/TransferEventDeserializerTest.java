package com.web3analytics.serde;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.errors.DeserializationException;
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

class TransferEventDeserializerTest {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    @Test
    void shouldRejectRawAvroPayload() throws Exception {
        byte[] avroPayload = buildTransferEventPayload();
        TransferEventDeserializer deserializer = new TransferEventDeserializer();

        assertThatThrownBy(() -> deserializer.deserialize(avroPayload))
                .isInstanceOf(DeserializationException.class)
                .hasMessageContaining("Expected CloudEvent JSON envelope");
    }

    @Test
    void shouldDeserializeCloudEventWithDataBase64() throws Exception {
        byte[] avroPayload = buildTransferEventPayload();
        TransferEventDeserializer deserializer = new TransferEventDeserializer();

        ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dex.events.transfer");
        envelope.put("source", "ingester");
        envelope.put("id", "evt-1");
        envelope.put("datacontenttype", "application/avro-binary");
        envelope.put("data_base64", Base64.getEncoder().encodeToString(avroPayload));

        TransferEvent event = deserializer.deserialize(
                OBJECT_MAPPER.writeValueAsString(envelope).getBytes(StandardCharsets.UTF_8)
        );

        assertThat(event).isNotNull();
        assertThat(event.eventId()).isEqualTo("tx-1:0");
        assertThat(event.pairAddress()).isEqualTo("0xpair");
        assertThat(event.from()).isEqualTo("0xfrom");
        assertThat(event.to()).isEqualTo("0xto");
        assertThat(event.value()).isEqualTo("777");
    }

    private static byte[] buildTransferEventPayload() throws IOException {
        Schema schema = loadSchema("/avro/TransferEvent.avsc");
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
        record.put("from", "0xfrom");
        record.put("to", "0xto");
        record.put("value", "777");
        record.put("eventTimestamp", 1700000001L);

        ByteArrayOutputStream out = new ByteArrayOutputStream();
        BinaryEncoder encoder = EncoderFactory.get().binaryEncoder(out, null);
        new GenericDatumWriter<GenericData.Record>(schema).write(record, encoder);
        encoder.flush();
        return out.toByteArray();
    }

    private static Schema loadSchema(String path) throws IOException {
        try (InputStream input = TransferEventDeserializerTest.class.getResourceAsStream(path)) {
            if (input == null) {
                throw new IOException("Schema not found: " + path);
            }
            return new Schema.Parser().parse(input);
        }
    }
}
