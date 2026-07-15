FROM gradle:8.10.2-jdk21 AS builder

WORKDIR /app

COPY settings.gradle.kts build.gradle.kts ./
COPY src ./src

RUN gradle --no-daemon clean installDist

FROM eclipse-temurin:21-jre

WORKDIR /app

COPY --from=builder /app/build/install/analytics-service ./analytics-service

EXPOSE 8080

CMD ["./analytics-service/bin/analytics-service"]