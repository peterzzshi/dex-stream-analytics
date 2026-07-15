package com.web3analytics.serde;

import com.web3analytics.models.DecodingError;
import org.apache.flink.streaming.api.functions.ProcessFunction;
import org.apache.flink.util.Collector;
import org.apache.flink.util.OutputTag;

import java.io.Serializable;

/**
 * Decodes records and routes failures to side output instead of failing the job.
 */
public class SafeDecodeProcessFunction<T> extends ProcessFunction<byte[], T> {

    @FunctionalInterface
    public interface Decoder<T> extends Serializable {
        T decode(byte[] message) throws Exception;
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

    @Override
    public void processElement(byte[] value, Context context, Collector<T> out) {
        try {
            T decoded = decoder.decode(value);
            if (decoded != null) {
                out.collect(decoded);
            }
        } catch (Exception error) {
            context.output(errorTag, DecodingError.from(stream, value, error));
        }
    }
}
