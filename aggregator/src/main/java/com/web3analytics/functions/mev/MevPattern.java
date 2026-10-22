package com.web3analytics.functions.mev;

import com.web3analytics.models.DexEvent;

import java.util.List;

/**
 * Base interface for MEV pattern detection.
 */
public interface MevPattern {

  List<MevCandidate> detect(List<DexEvent> events, SessionContext context);
}
