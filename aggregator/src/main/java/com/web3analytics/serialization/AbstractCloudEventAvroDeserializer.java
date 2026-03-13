package com.web3analytics.serialization;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.web3analytics.errors.DeserializationException;
import org.apache.avro.Schema;
import org.apache.avro.generic.GenericDatumReader;
import org.apache.avro.generic.GenericRecord;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;

import java.io.IOException;
import java.io.InputStream;

/**
 * Shared CloudEvent + Avro deserialization flow.
 * Concrete deserializers provide only schema path and record mapping strategy.
 */
public abstract class AbstractCloudEventAvroDeserializer<T> implements DeserializationSchema<T> {
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    private final GenericDatumReader<GenericRecord> reader;
    private final TypeInformation<T> producedType;
    private transient BinaryDecoder decoder;

    protected AbstractCloudEventAvroDeserializer(String schemaPath, Class<T> producedClass) throws IOException {
        Schema schema = loadSchema(schemaPath);
        this.reader = new GenericDatumReader<>(schema);
        this.producedType = TypeInformation.of(producedClass);
    }

    @Override
    public final T deserialize(byte[] message) throws IOException {
        if (message == null || message.length == 0) {
            return null;
        }

        byte[] payload = CloudEventPayloadExtractor.extract(message, OBJECT_MAPPER);
        if (payload.length == 0) {
            return null;
        }

        try {
            decoder = DecoderFactory.get().binaryDecoder(payload, decoder);
            GenericRecord record = reader.read(null, decoder);
            return map(record);
        } catch (Exception e) {
            throw new DeserializationException("Avro decoding failed", e);
        }
    }

    protected abstract T map(GenericRecord record);

    @Override
    public final boolean isEndOfStream(T nextElement) {
        return false;
    }

    @Override
    public final TypeInformation<T> getProducedType() {
        return producedType;
    }

    private Schema loadSchema(String resourcePath) throws IOException {
        try (InputStream input = getClass().getResourceAsStream(resourcePath)) {
            if (input == null) {
                throw new DeserializationException("Schema file not found: " + resourcePath);
            }
            return new Schema.Parser().parse(input);
        }
    }
}
