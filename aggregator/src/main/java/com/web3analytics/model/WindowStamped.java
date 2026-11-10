package com.web3analytics.model;

/** Implemented by accumulators that report which window produced them. */
public interface WindowStamped {

  void setWindowEnd(long windowEnd);
}
