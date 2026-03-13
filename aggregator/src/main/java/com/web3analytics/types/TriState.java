package com.web3analytics.types;

import java.util.NoSuchElementException;
import java.util.Objects;
import java.util.Optional;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Supplier;

/**
 * Tri-state optional type representing three distinct states:
 * 
 * 1. Defined(value) - Field has a value, should be set/updated in serialization
 * 2. Undefined - Field is absent, should be omitted from serialization (no change)
 * 3. Null - Field is explicitly null, should be serialized as null (removes field)
 * 
 * Inspired by Braze API, where:
 * - Defined: Update the field with new value
 * - Undefined: Leave field unchanged (don't include in request)
 * - Null: Remove the field (set to null)
 * 
 * Use cases in DEX analytics:
 * - volumeUSD: calculated | not-calculated-yet | explicitly-null-for-non-USD-pairs
 * - token0Symbol: resolved | not-resolved-yet | explicitly-null-for-unknown
 * 
 * Design principles:
 * - Algebraic data type (sum type with 3 variants)
 * - Immutable and type-safe
 * - Monadic operations (map, flatMap, orElse)
 * - Pattern matching support via sealed interface
 * - Demonstrates functional programming expertise
 * 
 * @param <T> The type of value when Defined
 */
public sealed interface TriState<T> {
    
    /**
     * Create a Defined state with a non-null value.
     */
    static <T> TriState<T> of(T value) {
        Objects.requireNonNull(value, "Use ofNullable() or explicit Null state for null values");
        return new Defined<>(value);
    }
    
    /**
     * Create a TriState from a nullable value.
     * - Non-null: Defined(value)
     * - Null: Null (explicit null)
     */
    static <T> TriState<T> ofNullable(T value) {
        return value == null ? new Null<>() : new Defined<>(value);
    }
    
    /**
     * Create an Undefined state (field absent, no change).
     */
    static <T> TriState<T> undefined() {
        return new Undefined<>();
    }
    
    /**
     * Create an explicit Null state (field should be removed/nullified).
     */
    static <T> TriState<T> ofNull() {
        return new Null<>();
    }
    
    /**
     * Check if this is a Defined state.
     */
    default boolean isDefined() {
        return this instanceof Defined;
    }
    
    /**
     * Check if this is an Undefined state.
     */
    default boolean isUndefined() {
        return this instanceof Undefined;
    }
    
    /**
     * Check if this is an explicit Null state.
     */
    default boolean isNull() {
        return this instanceof Null;
    }
    
    /**
     * Get the value if Defined, throw exception otherwise.
     */
    default T get() {
        return switch (this) {
            case Defined<T> d -> d.value;
            case Undefined<T> u -> throw new NoSuchElementException("Value is undefined");
            case Null<T> n -> throw new NoSuchElementException("Value is explicitly null");
        };
    }
    
    /**
     * Get the value if Defined, return default otherwise.
     */
    default T orElse(T defaultValue) {
        return switch (this) {
            case Defined<T> d -> d.value;
            case Undefined<T> u -> defaultValue;
            case Null<T> n -> defaultValue;
        };
    }
    
    /**
     * Get the value if Defined, compute default otherwise.
     */
    default T orElseGet(Supplier<? extends T> supplier) {
        return switch (this) {
            case Defined<T> d -> d.value;
            case Undefined<T> u -> supplier.get();
            case Null<T> n -> supplier.get();
        };
    }
    
    /**
     * Map the value if Defined, preserve state otherwise.
     * Demonstrates functor pattern.
     */
    default <U> TriState<U> map(Function<? super T, ? extends U> mapper) {
        return switch (this) {
            case Defined<T> d -> TriState.of(mapper.apply(d.value));
            case Undefined<T> u -> TriState.undefined();
            case Null<T> n -> TriState.ofNull();
        };
    }
    
    /**
     * FlatMap for monadic composition.
     * Allows chaining TriState-returning operations.
     */
    default <U> TriState<U> flatMap(Function<? super T, TriState<U>> mapper) {
        return switch (this) {
            case Defined<T> d -> mapper.apply(d.value);
            case Undefined<T> u -> TriState.undefined();
            case Null<T> n -> TriState.ofNull();
        };
    }
    
    /**
     * Execute action if Defined, do nothing otherwise.
     */
    default void ifDefined(Consumer<? super T> consumer) {
        if (this instanceof Defined<T> d) {
            consumer.accept(d.value);
        }
    }
    
    /**
     * Execute action based on state (pattern matching with lambdas).
     */
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
    
    /**
     * Pattern matching that returns a value (expression-based).
     */
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
    
    /**
     * Convert to Optional (loses tri-state information).
     * Defined -> Optional.of, Undefined/Null -> Optional.empty
     */
    default Optional<T> toOptional() {
        return switch (this) {
            case Defined<T> d -> Optional.of(d.value);
            case Undefined<T> u -> Optional.empty();
            case Null<T> n -> Optional.empty();
        };
    }
    
    /**
     * Defined state: field has a value.
     */
    record Defined<T>(T value) implements TriState<T> {
        public Defined {
            Objects.requireNonNull(value, "Defined value cannot be null");
        }
        
        @Override
        public String toString() {
            return "Defined[" + value + "]";
        }
    }
    
    /**
     * Undefined state: field is absent (omit from serialization).
     */
    record Undefined<T>() implements TriState<T> {
        @Override
        public String toString() {
            return "Undefined";
        }
    }
    
    /**
     * Null state: field is explicitly null (serialize as null).
     */
    record Null<T>() implements TriState<T> {
        @Override
        public String toString() {
            return "Null";
        }
    }
}
