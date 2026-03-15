package com.web3analytics.serde;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.web3analytics.errors.DeserializationErrorCategory;
import com.web3analytics.errors.DeserializationException;
import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.TransferEvent;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.avro.specific.SpecificDatumReader;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;

import java.io.IOException;

/**
 * Deserializes heterogeneous liquidity stream (MintEvent + BurnEvent).
 */
public class LiquidityEventDeserializer implements DeserializationSchema<DexEvent> {
    private static final String EVENT_TYPE_MINT = "com.dex.events.mint";
    private static final String EVENT_TYPE_BURN = "com.dex.events.burn";
    private static final String EVENT_TYPE_TRANSFER = "com.dex.events.transfer";
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    private final SpecificDatumReader<com.web3analytics.events.MintEvent> mintReader;
    private final SpecificDatumReader<com.web3analytics.events.BurnEvent> burnReader;
    private final SpecificDatumReader<com.web3analytics.events.TransferEvent> transferReader;

    private transient BinaryDecoder mintDecoder;
    private transient BinaryDecoder burnDecoder;
    private transient BinaryDecoder transferDecoder;

    public LiquidityEventDeserializer() {
        this.mintReader = new SpecificDatumReader<>(com.web3analytics.events.MintEvent.class);
        this.burnReader = new SpecificDatumReader<>(com.web3analytics.events.BurnEvent.class);
        this.transferReader = new SpecificDatumReader<>(com.web3analytics.events.TransferEvent.class);
    }

    @Override
    public DexEvent deserialize(byte[] message) throws IOException {
        if (message == null || message.length == 0) {
            return null;
        }

        CloudEventPayloadExtractor.Envelope envelope = CloudEventPayloadExtractor.extractEnvelope(message, OBJECT_MAPPER);
        String eventType = envelope.type();
        byte[] payload = envelope.payload();

        if (eventType == null || eventType.isBlank()) {
            throw new DeserializationException(DeserializationErrorCategory.INVALID_ENVELOPE,
                    "Invalid CloudEvent: missing type field");
        }

        // Pattern match on CloudEvent type BEFORE Avro deserialization.
        return switch (eventType) {
            case EVENT_TYPE_MINT -> decodeMint(payload);
            case EVENT_TYPE_BURN -> decodeBurn(payload);
            case EVENT_TYPE_TRANSFER -> decodeTransfer(payload);
            default -> throw new DeserializationException(DeserializationErrorCategory.UNSUPPORTED_EVENT_TYPE,
                    "Unsupported liquidity CloudEvent type: " + eventType);
        };
    }

    @Override
    public boolean isEndOfStream(DexEvent nextElement) {
        return false;
    }

    @Override
    public TypeInformation<DexEvent> getProducedType() {
        return TypeInformation.of(DexEvent.class);
    }

    private MintEvent decodeMint(byte[] payload) throws IOException {
        try {
            mintDecoder = DecoderFactory.get().binaryDecoder(payload, mintDecoder);
            com.web3analytics.events.MintEvent record = mintReader.read(null, mintDecoder);
            return new MintEvent(
                    record.getEventId(),
                    record.getBlockNumber(),
                    record.getBlockTimestamp(),
                    record.getTransactionHash(),
                    record.getLogIndex(),
                    record.getPairAddress(),
                    record.getToken0(),
                    record.getToken1(),
                    record.getToken0Symbol(),
                    record.getToken1Symbol(),
                    record.getSender(),
                    record.getAmount0(),
                    record.getAmount1(),
                    record.getEventTimestamp()
            );
        } catch (Exception err) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE,
                    "Failed to decode MintEvent", err);
        }
    }

    private BurnEvent decodeBurn(byte[] payload) throws IOException {
        try {
            burnDecoder = DecoderFactory.get().binaryDecoder(payload, burnDecoder);
            com.web3analytics.events.BurnEvent record = burnReader.read(null, burnDecoder);
            return new BurnEvent(
                    record.getEventId(),
                    record.getBlockNumber(),
                    record.getBlockTimestamp(),
                    record.getTransactionHash(),
                    record.getLogIndex(),
                    record.getPairAddress(),
                    record.getToken0(),
                    record.getToken1(),
                    record.getToken0Symbol(),
                    record.getToken1Symbol(),
                    record.getSender(),
                    record.getRecipient(),
                    record.getAmount0(),
                    record.getAmount1(),
                    record.getEventTimestamp()
            );
        } catch (Exception err) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE,
                    "Failed to decode BurnEvent", err);
        }
    }

    private TransferEvent decodeTransfer(byte[] payload) throws IOException {
        try {
            transferDecoder = DecoderFactory.get().binaryDecoder(payload, transferDecoder);
            com.web3analytics.events.TransferEvent record = transferReader.read(null, transferDecoder);
            return new TransferEvent(
                    record.getEventId(),
                    record.getBlockNumber(),
                    record.getBlockTimestamp(),
                    record.getTransactionHash(),
                    record.getLogIndex(),
                    record.getPairAddress(),
                    record.getToken0(),
                    record.getToken1(),
                    record.getToken0Symbol(),
                    record.getToken1Symbol(),
                    record.getFrom(),
                    record.getTo(),
                    record.getValue(),
                    record.getEventTimestamp()
            );
        } catch (Exception err) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE,
                    "Failed to decode TransferEvent", err);
        }
    }
}
