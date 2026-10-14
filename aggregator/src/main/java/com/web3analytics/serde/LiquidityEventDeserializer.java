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
 * Routes the heterogeneous liquidity topic by CloudEvent type before Avro decode.
 */
public class LiquidityEventDeserializer implements DeserializationSchema<DexEvent> {
    private static final String EVENT_TYPE_MINT = "com.dex.events.mint";
    private static final String EVENT_TYPE_BURN = "com.dex.events.burn";
    private static final String EVENT_TYPE_TRANSFER = "com.dex.events.transfer";
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    private transient SpecificDatumReader<com.web3analytics.events.MintEvent> mintReader;
    private transient SpecificDatumReader<com.web3analytics.events.BurnEvent> burnReader;
    private transient SpecificDatumReader<com.web3analytics.events.TransferEvent> transferReader;

    private transient BinaryDecoder mintDecoder;
    private transient BinaryDecoder burnDecoder;
    private transient BinaryDecoder transferDecoder;

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

        return switch (eventType) {
            case EVENT_TYPE_MINT -> decodeMint(payload);
            case EVENT_TYPE_BURN -> decodeBurn(payload);
            case EVENT_TYPE_TRANSFER -> decodeTransfer(payload);
            default -> throw new DeserializationException(DeserializationErrorCategory.UNSUPPORTED_EVENT_TYPE,
                    "Unsupported liquidity CloudEvent type: " + eventType);
        };
    }

    @Override
    public boolean isEndOfStream(DexEvent nextElement) { return false; }

    @Override
    public TypeInformation<DexEvent> getProducedType() { return TypeInformation.of(DexEvent.class); }

    private MintEvent decodeMint(byte[] payload) throws IOException {
        try {
            if (mintReader == null) {
                mintReader = new SpecificDatumReader<>(com.web3analytics.events.MintEvent.class);
            }
            mintDecoder = DecoderFactory.get().binaryDecoder(payload, mintDecoder);
            com.web3analytics.events.MintEvent r = mintReader.read(null, mintDecoder);
            return new MintEvent(
                    r.getEventId(), r.getBlockNumber(), r.getBlockTimestamp(),
                    r.getTransactionHash(), r.getLogIndex(), r.getPairAddress(),
                    r.getToken0(), r.getToken1(), r.getToken0Symbol(), r.getToken1Symbol(),
                    r.getSender(), r.getAmount0(), r.getAmount1(), r.getEventTimestamp());
        } catch (Exception err) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE, "Failed to decode MintEvent", err);
        }
    }

    private BurnEvent decodeBurn(byte[] payload) throws IOException {
        try {
            if (burnReader == null) {
                burnReader = new SpecificDatumReader<>(com.web3analytics.events.BurnEvent.class);
            }
            burnDecoder = DecoderFactory.get().binaryDecoder(payload, burnDecoder);
            com.web3analytics.events.BurnEvent r = burnReader.read(null, burnDecoder);
            return new BurnEvent(
                    r.getEventId(), r.getBlockNumber(), r.getBlockTimestamp(),
                    r.getTransactionHash(), r.getLogIndex(), r.getPairAddress(),
                    r.getToken0(), r.getToken1(), r.getToken0Symbol(), r.getToken1Symbol(),
                    r.getSender(), r.getRecipient(), r.getAmount0(), r.getAmount1(), r.getEventTimestamp());
        } catch (Exception err) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE, "Failed to decode BurnEvent", err);
        }
    }

    private TransferEvent decodeTransfer(byte[] payload) throws IOException {
        try {
            if (transferReader == null) {
                transferReader = new SpecificDatumReader<>(com.web3analytics.events.TransferEvent.class);
            }
            transferDecoder = DecoderFactory.get().binaryDecoder(payload, transferDecoder);
            com.web3analytics.events.TransferEvent r = transferReader.read(null, transferDecoder);
            return new TransferEvent(
                    r.getEventId(), r.getBlockNumber(), r.getBlockTimestamp(),
                    r.getTransactionHash(), r.getLogIndex(), r.getPairAddress(),
                    r.getToken0(), r.getToken1(), r.getToken0Symbol(), r.getToken1Symbol(),
                    r.getFrom(), r.getTo(), r.getValue(), r.getEventTimestamp());
        } catch (Exception err) {
            throw new DeserializationException(DeserializationErrorCategory.AVRO_DECODE, "Failed to decode TransferEvent", err);
        }
    }
}
