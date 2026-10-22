package com.web3analytics.serde;

import com.web3analytics.errors.DecodingError;
import java.io.IOException;
import java.io.Serializable;
import org.apache.flink.streaming.api.functions.ProcessFunction;
import org.apache.flink.util.Collector;
import org.apache.flink.util.OutputTag;

/**
 * Decodes records and routes failures to side output instead of failing the job.
 */
public class SafeDecodeProcessFunction<T> extends ProcessFunction<byte[], T> {

  @Override
  public void processElement(byte[] value, Context context, Collector<T> out) {
    try {
      T decoded = decoder.decode(value);
      if (decoded != null) {
        out.collect(decoded);
      }
    } catch (IOException error) {
      context.output(errorTag, DecodingError.from(stream, value, error));
    }
    }

    private final String stream;
    private final Decoder<T> decoder;
    private final OutputTag<DecodingError> errorTag;

    public SafeDecodeProcessFunction(
            String stream,
            Decoder<T> decoder,
            OutputTag<DecodingError> errorTag
    ) {
        this.stream = stream;
        this.decoder = decoder;
        this.errorTag = errorTag;
    }

  @FunctionalInterface
  public interface Decoder<T> extends Serializable {

    T decode(byte[] message) throws IOException;
    }
}
