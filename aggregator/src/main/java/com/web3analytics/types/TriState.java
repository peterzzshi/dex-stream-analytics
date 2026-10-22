package com.web3analytics.types;

import java.util.NoSuchElementException;
import java.util.Objects;
import java.util.Optional;
import java.util.function.Function;
import java.util.function.Supplier;

/**
 * Algebraic tri-state type distinguishing three cases:
 *
 * <ul>
 *   <li>{@code Defined(value)} – field has a concrete value</li>
 *   <li>{@code Undefined} – field absent / not yet resolved</li>
 *   <li>{@code Null} – field explicitly null (e.g. oracle lookup failed)</li>
 * </ul>
 *
 * This matters for DEX analytics because "volumeUSD not yet calculated"
 * and "volumeUSD is N/A for this pair" are semantically different from
 * "volumeUSD is zero."
 */
public sealed interface TriState<T> {

    static <T> TriState<T> of(T value) {
      Objects.requireNonNull(value, "Value cannot be null");
        return new Defined<>(value);
    }

    static <T> TriState<T> undefined() {
        return new Undefined<>();
    }

    static <T> TriState<T> ofNull() {
        return new Null<>();
    }

  default boolean isDefined() {
    return this instanceof Defined;
  }

  default boolean isUndefined() {
    return this instanceof Undefined;
  }

  default boolean isNull() {
    return this instanceof Null;
  }

    default T get() {
        return switch (this) {
          case Defined<T>(var value) -> value;
          case Undefined<T> ignored -> throw new NoSuchElementException("Value is undefined");
          case Null<T> ignored -> throw new NoSuchElementException("Value is explicitly null");
        };
    }

    default T orElse(T defaultValue) {
        return switch (this) {
          case Defined<T>(var value) -> value;
          case Undefined<T> ignored -> defaultValue;
          case Null<T> ignored2 -> defaultValue;
        };
    }

    default <U> TriState<U> map(Function<? super T, ? extends U> mapper) {
        return switch (this) {
          case Defined<T>(var value) -> TriState.of(mapper.apply(value));
          case Undefined<T> ignored -> TriState.undefined();
          case Null<T> ignored -> TriState.ofNull();
        };
    }

    default <U> TriState<U> flatMap(Function<? super T, TriState<U>> mapper) {
        return switch (this) {
          case Defined<T>(var value) -> mapper.apply(value);
          case Undefined<T> ignored -> TriState.undefined();
          case Null<T> ignored -> TriState.ofNull();
        };
    }

    default <R> R fold(
            Function<? super T, ? extends R> onDefined,
            Supplier<? extends R> onUndefined,
            Supplier<? extends R> onNull
    ) {
        return switch (this) {
          case Defined<T>(var value) -> onDefined.apply(value);
          case Undefined<T> ignored -> onUndefined.get();
          case Null<T> ignored -> onNull.get();
        };
    }

    default Optional<T> toOptional() {
        return switch (this) {
          case Defined<T>(var value) -> Optional.of(value);
          case Undefined<T> ignored -> Optional.empty();
          case Null<T> ignored2 -> Optional.empty();
        };
    }

    record Defined<T>(T value) implements TriState<T> {
        public Defined {
            Objects.requireNonNull(value, "Defined value cannot be null");
        }
    }

    record Undefined<T>() implements TriState<T> {
    }

    record Null<T>() implements TriState<T> {
    }
}
