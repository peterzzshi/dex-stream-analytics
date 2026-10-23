package com.web3analytics.serde;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatCode;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.web3analytics.errors.DeserializationException;
import java.math.BigDecimal;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.junit.jupiter.params.provider.ValueSource;

class AmountValidationTest {

  private static Stream<Arguments> amountsWithAtLeastOnePositive() {
    return Stream.of(
        Arguments.of((Object) new BigDecimal[]{BigDecimal.TEN, BigDecimal.ZERO, BigDecimal.ZERO}),
        Arguments.of((Object) new BigDecimal[]{BigDecimal.ZERO, BigDecimal.TEN, BigDecimal.ZERO}),
        Arguments.of((Object) new BigDecimal[]{BigDecimal.ZERO, BigDecimal.ZERO, BigDecimal.TEN}),
        Arguments.of((Object) new BigDecimal[]{BigDecimal.TEN}));
  }

  @ParameterizedTest
  @ValueSource(strings = {"0", "100", "123.456"})
  void requireNonNegativeAcceptsZeroOrPositive(final String rawValue) {
    assertThat(AmountValidation.requireNonNegative("amount", rawValue))
        .isEqualTo(new BigDecimal(rawValue));
  }

  @ParameterizedTest
  @ValueSource(strings = {"-1", "-0.0001"})
  void requireNonNegativeRejectsNegative(final String rawValue) {
    assertThatThrownBy(() -> AmountValidation.requireNonNegative("amount", rawValue))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("amount")
        .hasMessageContaining("negative");
  }

  @ParameterizedTest
  @ValueSource(strings = {"not-a-number", ""})
  void requireNonNegativeRejectsUnparseableValue(final String rawValue) {
    assertThatThrownBy(() -> AmountValidation.requireNonNegative("amount", rawValue))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("not a valid number");
  }

  @Test
  void requireNonNegativeRejectsNullValue() {
    assertThatThrownBy(() -> AmountValidation.requireNonNegative("amount", null))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("not a valid number");
  }

  @ParameterizedTest
  @MethodSource("amountsWithAtLeastOnePositive")
  void requireAtLeastOnePositivePassesWhenAnyAmountIsPositive(final BigDecimal[] amounts) {
    assertThatCode(() -> AmountValidation.requireAtLeastOnePositive("SwapEvent", amounts))
        .doesNotThrowAnyException();
  }

  @Test
  void requireAtLeastOnePositiveRejectsAllZero() {
    assertThatThrownBy(() -> AmountValidation.requireAtLeastOnePositive(
        "SwapEvent", BigDecimal.ZERO, BigDecimal.ZERO))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("SwapEvent")
        .hasMessageContaining("no non-zero amount field");
  }
}
