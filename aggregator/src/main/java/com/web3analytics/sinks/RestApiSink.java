package com.web3analytics.sinks;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.Serial;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import org.apache.flink.api.connector.sink2.Sink;
import org.apache.flink.api.connector.sink2.SinkWriter;
import org.apache.flink.api.connector.sink2.WriterInitContext;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class RestApiSink<T> implements Sink<T> {

  @Serial
  private static final long serialVersionUID = 1975484154156176173L;
  private final String endpoint;

  public RestApiSink(final String endpoint) {
    this.endpoint = endpoint;
  }

  @Override
  public SinkWriter<T> createWriter(final WriterInitContext context) {
    return new RestApiSinkWriter<>(this.endpoint);
  }

  private static class RestApiSinkWriter<T> implements SinkWriter<T> {

    private static final Logger LOG = LoggerFactory.getLogger(RestApiSinkWriter.class);

    private final String endpoint;
    private final HttpClient httpClient;
    private final ObjectMapper objectMapper;

    RestApiSinkWriter(final String endpoint) {
      this.endpoint = endpoint;
      this.httpClient = HttpClient.newBuilder()
          .connectTimeout(Duration.ofSeconds(10))
          .build();
      this.objectMapper = new ObjectMapper();
    }

    @Override
    public void write(final T element, final Context context) throws IOException {
      final String json = this.objectMapper.writeValueAsString(element);

      final HttpRequest request = HttpRequest.newBuilder()
          .uri(URI.create(this.endpoint))
          .header("Content-Type", "application/json")
          .POST(HttpRequest.BodyPublishers.ofString(json))
          .timeout(Duration.ofSeconds(30))
          .build();

      try {
        final HttpResponse<String> response = this.httpClient.send(request,
            HttpResponse.BodyHandlers.ofString());

        if (response.statusCode() < 200 || response.statusCode() >= 300) {
          final String body = response.body();
          LOG.error("Failed to POST to {}: status={}, body={}", this.endpoint,
              response.statusCode(),
              body);
          throw new IOException(
              String.format("HTTP %d from %s: %s", response.statusCode(), this.endpoint, body));
        } else {
          LOG.debug("Successfully posted to {}: status={}", this.endpoint, response.statusCode());
        }
      } catch (final InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new IOException("HTTP request interrupted", e);
      }
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

