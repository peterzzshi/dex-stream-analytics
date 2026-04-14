package com.web3analytics.serde;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.web3analytics.errors.DeserializationErrorCategory;
import com.web3analytics.errors.DeserializationException;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.avro.specific.SpecificDatumReader;
import org.apache.avro.specific.SpecificRecord;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;

import java.io.IOException;

/**
 * Template for CloudEvent envelope unwrap + Avro binary decode.
 * Subclasses provide only the Avro-to-model mapping.
 */
public abstract class AbstractCloudEventAvroDeserializer<T, S extends SpecificRecord> implements DeserializationSchema<T> {
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    private final Class<S> recordClass;
    private final TypeInformation<T> producedType;
    private transient SpecificDatumReader<S> reader;
    private transient BinaryDecoder decoder;

    protected AbstractCloudEventAvroDeserializer(Class<S> recordClass, Class<T> producedClass) {
        this.recordClass = recordClass;
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
            if (reader == null) {
                reader = new SpecificDatumReader<>(recordClass);
            }
            decoder = DecoderFactory.get().binaryDecoder(payload, decoder);
            S record = reader.read(null, decoder);
            return map(record);
        } catch (Exception e) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE, "Avro decoding failed", e);
        }
    }

    protected abstract T map(S record);

    @Override
    public final boolean isEndOfStream(T nextElement) { return false; }

    @Override
    public final TypeInformation<T> getProducedType() { return producedType; }
}
