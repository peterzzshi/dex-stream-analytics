package com.web3analytics.serialization;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;

import java.io.IOException;

/**
 * Deserializes heterogeneous liquidity stream (MintEvent + BurnEvent).
 */
public class LiquidityEventDeserializer implements DeserializationSchema<DexEvent> {

    private final MintEventDeserializer mintDeserializer;
    private final BurnEventDeserializer burnDeserializer;

    public LiquidityEventDeserializer() throws IOException {
        this.mintDeserializer = new MintEventDeserializer();
        this.burnDeserializer = new BurnEventDeserializer();
    }

    @Override
    public DexEvent deserialize(byte[] message) throws IOException {
        if (message == null || message.length == 0) {
            return null;
        }

        IOException mintError = null;
        try {
            MintEvent mint = mintDeserializer.deserialize(message);
            if (mint != null && isLikelyMint(mint)) {
                return mint;
            }
            if (mint != null) {
                mintError = new IOException("Mint payload shape is invalid");
            }
        } catch (IOException error) {
            mintError = error;
        }

        IOException burnError = null;
        try {
            BurnEvent burn = burnDeserializer.deserialize(message);
            if (burn != null && isLikelyBurn(burn)) {
                return burn;
            }
            if (burn != null) {
                burnError = new IOException("Burn payload shape is invalid");
            }
        } catch (IOException error) {
            burnError = error;
        }

        if (mintError != null && burnError != null) {
            burnError.addSuppressed(mintError);
            throw burnError;
        }

        if (mintError != null) {
            throw mintError;
        }

        if (burnError != null) {
            throw burnError;
        }

        return null;
    }

    @Override
    public boolean isEndOfStream(DexEvent nextElement) {
        return false;
    }

    @Override
    public TypeInformation<DexEvent> getProducedType() {
        return TypeInformation.of(DexEvent.class);
    }

    private static boolean isLikelyMint(MintEvent event) {
        return isSignedInteger(event.amount0()) && isSignedInteger(event.amount1());
    }

    private static boolean isLikelyBurn(BurnEvent event) {
        return isSignedInteger(event.amount0())
                && isSignedInteger(event.amount1())
                && event.recipient() != null
                && !event.recipient().isBlank();
    }

    private static boolean isSignedInteger(String value) {
        if (value == null) {
            return false;
        }

        String trimmed = value.trim();
        if (trimmed.isEmpty()) {
            return false;
        }

        int start = trimmed.charAt(0) == '-' ? 1 : 0;
        if (start == trimmed.length()) {
            return false;
        }

        for (int i = start; i < trimmed.length(); i++) {
            if (!Character.isDigit(trimmed.charAt(i))) {
                return false;
            }
        }
        return true;
    }
}
