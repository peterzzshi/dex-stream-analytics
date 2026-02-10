# Build Flink job with Java 17 and Flink 2.0.1
FROM maven:3.9-eclipse-temurin-17 AS build
WORKDIR /build
COPY pom.xml .
RUN mvn -q -B dependency:go-offline
COPY src src
COPY ../schemas/avro ../schemas/avro
RUN mvn -q -B package -DskipTests

FROM flink:2.0.1-scala_2.12-java17
COPY --from=build /build/target/aggregator-1.0-SNAPSHOT.jar /opt/flink/usrlib/aggregator.jar
ENV FLINK_PARALLELISM=2
CMD ["/docker-entrypoint.sh", "taskmanager"]
