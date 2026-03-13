package com.web3analytics.types;

import com.web3analytics.models.SwapEvent;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Examples demonstrating TriState usage in DEX analytics.
 * 
 * Shows how tri-state optionals solve real business problems:
 * 1. Symbol resolution (resolved | pending | failed)
 * 2. USD volume calculation (calculated | pending | N/A for non-USD pairs)
 * 3. LP token correlation (correlated | pending | failed)
 */
class TriStateExamples {
    
    /**
     * Example 1: Token symbol resolution
     * 
     * Scenarios:
     * - Defined("WMATIC"): Symbol successfully resolved from contract
     * - Undefined: Symbol resolution not attempted yet (default state)
     * - Null: Symbol resolution attempted but failed (contract has no symbol)
     */
    @Test
    void tokenSymbolResolution() {
        // Successfully resolved
        TriState<String> resolved = TriState.of("WMATIC");
        assertTrue(resolved.isDefined());
        assertEquals("WMATIC", resolved.get());
        
        // Not yet resolved (default state for new events)
        TriState<String> pending = TriState.undefined();
        assertTrue(pending.isUndefined());
        assertEquals("UNKNOWN", pending.orElse("UNKNOWN"));
        
        // Resolution failed (contract doesn't implement symbol())
        TriState<String> failed = TriState.ofNull();
        assertTrue(failed.isNull());
        
        // Pattern matching for logging
        resolved.match(
            symbol -> System.out.println("Resolved: " + symbol),
            () -> System.out.println("Pending resolution"),
            () -> System.out.println("Failed to resolve")
        );
    }
    
    /**
     * Example 2: USD volume calculation
     * 
     * Scenarios:
     * - Defined(12345.67): Volume calculated using price oracle
     * - Undefined: Calculation not performed yet (no oracle available)
     * - Null: Pair doesn't involve USD stablecoin (N/A)
     */
    @Test
    void volumeUSDCalculation() {
        // Calculated volume for USDC pair
        TriState<Double> calculated = TriState.of(12345.67);
        
        // Not yet calculated (oracle lookup pending)
        TriState<Double> pending = TriState.undefined();
        
        // Not applicable (WMATIC/WETH pair - no USD)
        TriState<Double> notApplicable = TriState.ofNull();
        
        // Map to display string
        String display = calculated.map(v -> String.format("$%.2f", v))
            .orElse("N/A");
        assertEquals("$12345.67", display);
        
        // Default to 0.0 for undefined, but null for explicit N/A
        Double value = pending.orElse(0.0); // Pending -> use 0
        assertEquals(0.0, value);
    }
    
    /**
     * Example 3: Chaining operations with flatMap (monadic composition)
     */
    @Test
    void monadicComposition() {
        // Chain USD conversion: amount -> price lookup -> USD calculation
        TriState<Double> amount = TriState.of(100.0);
        
        TriState<Double> usdValue = amount
            .flatMap(this::lookupPrice)  // TriState<Double> -> TriState<Double>
            .map(price -> price * 100.0); // Apply exchange rate
        
        assertTrue(usdValue.isDefined());
        assertEquals(180000.0, usdValue.get()); // 100 * 1800 * 100
    }
    
    private TriState<Double> lookupPrice(Double amount) {
        // Simulate price lookup (1800 for WETH)
        return TriState.of(1800.0);
    }
    
    /**
     * Example 5: Serialization control
     * 
     * Show how TriState affects Avro serialization:
     * - Defined: Include field with value
     * - Undefined: Omit field from message (saves bandwidth)
     * - Null: Include field as explicit null
     */
    @Test
    void serializationControl() {
        record EventPayload(
            String eventId,
            TriState<Double> volumeUSD,
            TriState<String> token0Symbol
        ) {}
        
        // Case 1: All fields defined
        var complete = new EventPayload(
            "evt-123",
            TriState.of(1000.0),
            TriState.of("WMATIC")
        );
        
        // Case 2: Symbol undefined (not resolved yet) - omit from message
        var pending = new EventPayload(
            "evt-456",
            TriState.of(2000.0),
            TriState.undefined()
        );
        
        // Case 3: Volume null (not USD pair) - serialize as null
        var nonUSD = new EventPayload(
            "evt-789",
            TriState.ofNull(),
            TriState.of("WETH")
        );
        
        // In serialization logic:
        // if (volumeUSD.isDefined()) record.put("volumeUSD", volumeUSD.get());
        // else if (volumeUSD.isNull()) record.put("volumeUSD", null);
        // // if undefined, don't include field at all
    }
    
    /**
     * Example 6: Integration with existing Optional-based code
     */
    @Test
    void optionalInterop() {
        TriState<String> triState = TriState.of("value");
        
        // Convert to Optional when needed
        var optional = triState.toOptional();
        assertTrue(optional.isPresent());
        assertEquals("value", optional.get());
        
        // Undefined and Null both become empty Optional
        assertTrue(TriState.<String>undefined().toOptional().isEmpty());
        assertTrue(TriState.<String>ofNull().toOptional().isEmpty());
    }
}
