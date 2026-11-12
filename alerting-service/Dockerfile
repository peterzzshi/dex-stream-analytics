FROM gradle:9.9.0-jdk21 AS builder

WORKDIR /app

# Cache dependencies layer (only invalidated when build config changes)
COPY settings.gradle.kts build.gradle.kts ./
RUN gradle --no-daemon dependencies || true

# Copy schemas (needed for Avro deserialization at runtime)
COPY ../schemas ./schemas

# Build application (only re-runs when source changes)
COPY src ./src
RUN gradle --no-daemon clean installDist

FROM eclipse-temurin:21-jre

WORKDIR /app

COPY --from=builder /app/build/install/alerting-service ./alerting-service
COPY --from=builder /app/schemas ./schemas

EXPOSE 8080

CMD ["./alerting-service/bin/alerting-service"]
