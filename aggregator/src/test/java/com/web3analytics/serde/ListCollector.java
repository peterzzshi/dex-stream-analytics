package com.web3analytics.serde;

import java.util.ArrayList;
import java.util.List;
import org.apache.flink.util.Collector;

/**
 * Test double for Flink's {@link Collector}: accumulates collected records into a list.
 */
final class ListCollector<T> implements Collector<T> {

  private final List<T> items = new ArrayList<>();

  @Override
  public void collect(final T messageRecord) {
    this.items.add(messageRecord);
  }

  @Override
  public void close() {
  }

  List<T> items() {
    return this.items;
  }
}
