package com.web3analytics.sinks;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.module.SimpleModule;
import com.fasterxml.jackson.databind.ser.std.ToStringSerializer;
import com.web3analytics.errors.NetworkException;
import com.web3analytics.errors.RateLimitException;
import com.web3analytics.errors.RejectedException;
import com.web3analytics.errors.ServerException;
import com.web3analytics.errors.SinkException;
import java.io.IOException;
import java.io.Serial;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import org.apache.flink.api.connector.sink2.Sink;
import org.apache.flink.api.connector.sink2.SinkWriter;
import org.apache.flink.api.connector.sink2.WriterInitContext;
import org.apache.flink.metrics.Counter;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * POSTs each emitted window to a REST endpoint, retrying transient failures inline before giving
 * up.
 *
 * <p>Retry is inline rather than delegated to Flink's restart strategy because a restart replays
 * from the last checkpoint: the operator re-emits every window since then, including the ones that
 * already succeeded, so a single failing window turns into a burst of duplicate POSTs. An inline
 * retry re-sends only the record that failed.
 *
 * <p>The costs are bounded: {@link RestApiSinkWriter#MAX_ATTEMPTS} attempts with a delay capped at
 * {@link RetryPolicy#MAX_BACKOFF_MS} keep the worst case far under the 10-minute checkpoint
 * timeout, so the blocking wait cannot starve a checkpoint.
 */
public class RestApiSink<T> implements Sink<T> {

  @Serial
  private static final long serialVersionUID = 1975484154156176173L;
  private final String endpoint;

  public RestApiSink(final String endpoint) {
    this.endpoint = endpoint;
  }

  @Override
  public SinkWriter<T> createWriter(final WriterInitContext context) {
    return new RestApiSinkWriter<>(this.endpoint,
        context.metricGroup().counter("recordsDroppedFatal"),
        context.metricGroup().counter("sinkRetries"));
  }

  private static class RestApiSinkWriter<T> implements SinkWriter<T> {

    private static final Logger LOG = LoggerFactory.getLogger(RestApiSinkWriter.class);

    /**
     * Total attempts per record, matching the ingesters' {@code max_retries: 3} plus the first.
     */
    private static final int MAX_ATTEMPTS = 4;

    private static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(10);
    private static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(30);

    private final String endpoint;
    private final HttpClient httpClient;
    private final ObjectMapper objectMapper;
    private final Counter droppedFatal;
    private final Counter retries;

    RestApiSinkWriter(final String endpoint, final Counter droppedFatal, final Counter retries) {
      this.endpoint = endpoint;
      this.droppedFatal = droppedFatal;
      this.retries = retries;
      this.httpClient = HttpClient.newBuilder().connectTimeout(CONNECT_TIMEOUT).build();
      this.objectMapper = new ObjectMapper();
      // Token amounts are 78-digit integers: as bare JSON numbers they exceed every
      // downstream long/double, so they carry as strings like gasPrice already does.
      this.objectMapper.registerModule(new SimpleModule()
          .addSerializer(BigInteger.class, ToStringSerializer.instance)
          .addSerializer(BigDecimal.class, ToStringSerializer.instance));
    }

    /**
     * Exhaustive over the sealed hierarchy, so a new failure type has to state whether resending it
     * is worth doing rather than inheriting an answer.
     */
    private static boolean retriable(final SinkException failure) {
      return switch (failure) {
        case final RejectedException ignored -> false;
        case final NetworkException ignored -> true;
        case final ServerException ignored -> true;
        case final RateLimitException ignored -> true;
      };
    }

    private static void sleep(final long millis) {
      try {
        Thread.sleep(millis);
      } catch (final InterruptedException e) {
        Thread.currentThread().interrupt();
      }
    }

    @Override
    public void write(final T element, final Context context) throws IOException {
      // A window that cannot serialize is a defect in this pipeline, not bad input, so the
      // JsonProcessingException propagates and fails the job. Counting it as a drop would make a
      // broken serializer look like a clean run.
      final String json = this.objectMapper.writeValueAsString(element);

      try {
        this.send(this.request(json));
      } catch (final RejectedException e) {
        // The service read the payload and refused it. The same bytes will be refused again, so
        // failing the task would only loop; this is the one failure that ends here.
        this.droppedFatal.inc();
        LOG.error("Dropping record rejected by {}", this.endpoint, e);
      }
      // Every other SinkException is an IOException, so it propagates to Flink's restart strategy.
      // A second delivery of an already-sent record is possible either way; only the receiving
      // service can tell them apart, by making the POST idempotent.
    }

    private HttpRequest request(final String json) {
      return HttpRequest.newBuilder()
          .uri(URI.create(this.endpoint))
          .header("Content-Type", "application/json")
          .POST(HttpRequest.BodyPublishers.ofString(json))
          .timeout(REQUEST_TIMEOUT)
          .build();
    }

    /**
     * Sends the request, retrying transient failures with backoff. A rejected payload is thrown on
     * the first attempt because a resend cannot help; the last transient failure is thrown once the
     * attempts run out.
     */
    private void send(final HttpRequest request) throws SinkException {
      SinkException last = null;
      for (int attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
        try {
          this.post(request);
          return;
        } catch (final SinkException e) {
          if (!retriable(e)) {
            throw e;
          }
          last = e;
          if (attempt < MAX_ATTEMPTS) {
            this.retries.inc();
            RestApiSinkWriter.sleep(RetryPolicy.delayMillis(attempt, e));
          }
        }
      }
      throw last;
    }

    /**
     * One POST. Returns normally on 2xx, otherwise throws the classified failure.
     */
    private void post(final HttpRequest request) throws SinkException {
      final HttpResponse<String> response;
      try {
        response = this.httpClient.send(request, HttpResponse.BodyHandlers.ofString());
      } catch (final InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new NetworkException(this.endpoint, "request interrupted");
      } catch (final IOException e) {
        throw new NetworkException(this.endpoint, e.toString());
      }

      final int status = response.statusCode();
      if (status >= 200 && status < 300) {
        LOG.debug("Successfully posted to {}: status={}", this.endpoint, status);
        return;
      }
      final String body = response.body();
      // 408 sits with 429 rather than with the other 4xx: a request timeout is a statement about
      // this attempt, not about the payload, so the same bytes may well be accepted later.
      if (status == 429 || status == 408) {
        throw new RateLimitException(this.endpoint, status, body);
      }
      if (status >= 500) {
        throw new ServerException(this.endpoint, status, body);
      }
      throw new RejectedException(this.endpoint, status, body);
    }

    @Override
    public void flush(final boolean endOfInput) {
      // No buffering, nothing to flush
    }

    @Override
    public void close() {
      // HttpClient doesn't need explicit cleanup
    }
  }
}
