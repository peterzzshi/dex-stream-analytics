package com.web3analytics.functions;

/**
 * Shared utilities for window-based aggregation functions.
 */
final class WindowUtils {

  private WindowUtils() {
  }

  /**
   * Builds a unique window identifier from pair address and time boundaries.
   */
  static String buildWindowId(String pairAddress, long windowStart, long windowEnd) {
    return pairAddress + ":" + windowStart + ":" + windowEnd;
  }

  /**
   * Builds a window identifier with a semantic prefix (e.g., "trend").
   */
  static String buildWindowId(String pairAddress, long windowStart, long windowEnd, String prefix) {
    return pairAddress + ":" + prefix + ":" + windowStart + ":" + windowEnd;
  }

  /**
   * Returns the lexicographically first non-blank symbol for deterministic aggregation. This
   * ensures identical events produce identical results regardless of processing order.
   */
  static String preferSymbol(String current, String candidate) {
    boolean currentValid = current != null && !current.isBlank();
    boolean candidateValid = candidate != null && !candidate.isBlank();

    if (!currentValid) {
      return candidateValid ? candidate : null;
    }
    if (!candidateValid) {
      return current;
    }

    // Deterministic: pick lexicographically first
    return current.compareTo(candidate) <= 0 ? current : candidate;
  }
}
