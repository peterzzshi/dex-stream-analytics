package com.web3analytics.functions;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.math.RoundingMode;
import java.util.Locale;

final class Web3Math {

    private Web3Math() {
    }

    static BigInteger parseBigInt(String value) {
        if (!isSignedInteger(value)) {
            return BigInteger.ZERO;
        }
        return new BigInteger(value.trim());
    }

    static String normalizeAddress(String address) {
        if (address == null) {
            return "";
        }
        return address.toLowerCase(Locale.ROOT);
    }

    static double round8(double value) {
        return BigDecimal.valueOf(value)
                .setScale(8, RoundingMode.HALF_UP)
                .doubleValue();
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
