package com.web3analytics.types;

import java.util.NoSuchElementException;
import java.util.Objects;
import java.util.Optional;
import java.util.function.Consumer;
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
        Objects.requireNonNull(value, "Use ofNullable() for nullable values");
        return new Defined<>(value);
    }

    static <T> TriState<T> ofNullable(T value) {
        return value == null ? new Null<>() : new Defined<>(value);
    }

    static <T> TriState<T> undefined() {
        return new Undefined<>();
    }

    static <T> TriState<T> ofNull() {
        return new Null<>();
    }

    default boolean isDefined() { return this instanceof Defined; }
    default boolean isUndefined() { return this instanceof Undefined; }
    default boolean isNull() { return this instanceof Null; }

    default T get() {
        return switch (this) {
            case Defined<T> d -> d.value;
            case Undefined<T> u -> throw new NoSuchElementException("Value is undefined");
            case Null<T> n -> throw new NoSuchElementException("Value is explicitly null");
        };
    }

    default T orElse(T defaultValue) {
        return switch (this) {
            case Defined<T> d -> d.value;
            case Undefined<T> u -> defaultValue;
            case Null<T> n -> defaultValue;
        };
    }

    default T orElseGet(Supplier<? extends T> supplier) {
        return switch (this) {
            case Defined<T> d -> d.value;
            case Undefined<T> u -> supplier.get();
            case Null<T> n -> supplier.get();
        };
    }

    default <U> TriState<U> map(Function<? super T, ? extends U> mapper) {
        return switch (this) {
            case Defined<T> d -> TriState.of(mapper.apply(d.value));
            case Undefined<T> u -> TriState.undefined();
            case Null<T> n -> TriState.ofNull();
        };
    }

    default <U> TriState<U> flatMap(Function<? super T, TriState<U>> mapper) {
        return switch (this) {
            case Defined<T> d -> mapper.apply(d.value);
            case Undefined<T> u -> TriState.undefined();
            case Null<T> n -> TriState.ofNull();
        };
    }

    default void ifDefined(Consumer<? super T> consumer) {
        if (this instanceof Defined<T> d) {
            consumer.accept(d.value);
        }
    }

    default void match(
            Consumer<? super T> onDefined,
            Runnable onUndefined,
            Runnable onNull
    ) {
        switch (this) {
            case Defined<T> d -> onDefined.accept(d.value);
            case Undefined<T> u -> onUndefined.run();
            case Null<T> n -> onNull.run();
        }
    }

    default <R> R fold(
            Function<? super T, ? extends R> onDefined,
            Supplier<? extends R> onUndefined,
            Supplier<? extends R> onNull
    ) {
        return switch (this) {
            case Defined<T> d -> onDefined.apply(d.value);
            case Undefined<T> u -> onUndefined.get();
            case Null<T> n -> onNull.get();
        };
    }

    /** Collapses to Optional, losing the Undefined/Null distinction. */
    default Optional<T> toOptional() {
        return switch (this) {
            case Defined<T> d -> Optional.of(d.value);
            case Undefined<T> u -> Optional.empty();
            case Null<T> n -> Optional.empty();
        };
    }

    record Defined<T>(T value) implements TriState<T> {
        public Defined {
            Objects.requireNonNull(value, "Defined value cannot be null");
        }

        @Override
        public String toString() { return "Defined[" + value + "]"; }
    }

    record Undefined<T>() implements TriState<T> {
        @Override
        public String toString() { return "Undefined"; }
    }

    record Null<T>() implements TriState<T> {
        @Override
        public String toString() { return "Null"; }
    }
}
