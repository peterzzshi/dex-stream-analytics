package com.web3analytics.types;

import static org.assertj.core.api.Assertions.assertThat;

import org.junit.jupiter.api.Test;

/**
 * Behavioral tests demonstrating TriState usage patterns in DEX analytics. Focus: How TriState
 * solves real business problems with three-valued logic.
 */
class TriStateTest {

  @Test
  void distinguishesBetweenPendingAndFailedSymbolResolution() {
    TriState<String> resolved = TriState.of("WMATIC");
    TriState<String> pending = TriState.undefined();
    TriState<String> failed = TriState.ofNull();

    assertThat(resolved.isDefined()).isTrue();
    assertThat(resolved.get()).isEqualTo("WMATIC");

    assertThat(pending.isUndefined()).isTrue();
    assertThat(pending.orElse("UNKNOWN")).isEqualTo("UNKNOWN");

    assertThat(failed.isNull()).isTrue();
  }

  @Test
  void distinguishesBetweenNotCalculatedAndNotApplicable() {
    TriState<Double> calculated = TriState.of(12345.67);
    TriState<Double> notYetCalculated = TriState.undefined();
    TriState<Double> notApplicable = TriState.ofNull();

    assertThat(calculated.isDefined()).isTrue();
    assertThat(calculated.get()).isEqualTo(12345.67);

    assertThat(notYetCalculated.isUndefined()).isTrue();
    assertThat(notYetCalculated.orElse(0.0)).isEqualTo(0.0);

    assertThat(notApplicable.isNull()).isTrue();
  }

  @Test
  void mapsDefinedValuesToNewValues() {
    TriState<Double> amount = TriState.of(100.0);

    TriState<String> formatted = amount.map(v -> String.format("$%.2f", v));

    assertThat(formatted.isDefined()).isTrue();
    assertThat(formatted.get()).isEqualTo("$100.00");
  }

  @Test
  void mapPreservesUndefinedAndNullStates() {
    TriState<Double> undefined = TriState.undefined();
    TriState<Double> nullState = TriState.ofNull();

    TriState<String> mappedUndefined = undefined.map(v -> String.format("$%.2f", v));
    TriState<String> mappedNull = nullState.map(v -> String.format("$%.2f", v));

    assertThat(mappedUndefined.isUndefined()).isTrue();
    assertThat(mappedNull.isNull()).isTrue();
  }

  @Test
  void flatMapChainsTriStateOperations() {
    TriState<Double> amount = TriState.of(100.0);

    TriState<Double> usdValue = amount
        .flatMap(this::lookupPrice)
        .map(price -> price * 100.0);

    assertThat(usdValue.isDefined()).isTrue();
    assertThat(usdValue.get()).isEqualTo(180000.0);
  }

  @Test
  void flatMapPropagatesUndefinedAndNull() {
    TriState<Double> undefined = TriState.undefined();
    TriState<Double> nullState = TriState.ofNull();

    TriState<Double> fromUndefined = undefined.flatMap(this::lookupPrice);
    TriState<Double> fromNull = nullState.flatMap(this::lookupPrice);

    assertThat(fromUndefined.isUndefined()).isTrue();
    assertThat(fromNull.isNull()).isTrue();
  }

  @Test
  void foldReturnsValueBasedOnState() {
    TriState<String> defined = TriState.of("WMATIC");
    TriState<String> undefined = TriState.undefined();
    TriState<String> nullState = TriState.ofNull();

    String definedResult = defined.fold(
        val -> "Defined: " + val,
        () -> "Undefined",
        () -> "Null"
    );

    String undefinedResult = undefined.fold(
        val -> "Defined: " + val,
        () -> "Undefined",
        () -> "Null"
    );

    String nullResult = nullState.fold(
        val -> "Defined: " + val,
        () -> "Undefined",
        () -> "Null"
    );

    assertThat(definedResult).isEqualTo("Defined: WMATIC");
    assertThat(undefinedResult).isEqualTo("Undefined");
    assertThat(nullResult).isEqualTo("Null");
  }

  @Test
  void convertsToOptionalCollapsingUndefinedAndNull() {
    TriState<String> defined = TriState.of("value");
    TriState<String> undefined = TriState.undefined();
    TriState<String> nullState = TriState.ofNull();

    assertThat(defined.toOptional()).isPresent();
    assertThat(defined.toOptional()).contains("value");

    assertThat(undefined.toOptional()).isEmpty();
    assertThat(nullState.toOptional()).isEmpty();
  }

  @Test
  void orElseReturnsValueForDefinedAndDefault() {
    TriState<String> defined = TriState.of("WMATIC");
    TriState<String> undefined = TriState.undefined();
    TriState<String> nullState = TriState.ofNull();

    assertThat(defined.orElse("DEFAULT")).isEqualTo("WMATIC");
    assertThat(undefined.orElse("DEFAULT")).isEqualTo("DEFAULT");
    assertThat(nullState.orElse("DEFAULT")).isEqualTo("DEFAULT");
  }

  private TriState<Double> lookupPrice(Double amount) {
    return TriState.of(1800.0);
  }
}
