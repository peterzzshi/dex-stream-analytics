package com.web3analytics.serde;

import com.web3analytics.errors.DeserializationException;
import java.math.BigDecimal;

/**
 * Cross-field sanity checks for decoded on-chain amount fields, shared across the
 * swap/mint/burn/transfer deserializers.
 */
final class AmountValidation {

  private AmountValidation() {
  }

  static BigDecimal requireNonNegative(final String fieldName, final String rawValue) {
    final BigDecimal value;
    try {
      value = new BigDecimal(rawValue);
    } catch (final NumberFormatException | NullPointerException e) {
      throw new DeserializationException(fieldName + " is not a valid number: " + rawValue);
    }
    if (value.signum() < 0) {
      throw new DeserializationException(fieldName + " must not be negative: " + rawValue);
    }
    return value;
  }

  static void requireAtLeastOnePositive(final String eventName, final BigDecimal... amounts) {
    for (final BigDecimal amount : amounts) {
      if (amount.signum() > 0) {
        return;
      }
    }
    throw new DeserializationException(eventName + " has no non-zero amount field");
  }
}
