#!/bin/bash
# Verify all services are running and accessible

set -e

echo "🔍 Checking DEX Stream Analytics services..."
echo

# Check core services
echo "✓ Docker containers:"
docker compose ps --format "  {{.Service}}: {{.Status}}" 2>/dev/null | grep -E "kafka|redis|flink|analytics-service" || {
  echo "  ❌ Services not running. Run: docker compose up -d --build"
  exit 1
}
echo

# Check health endpoints
echo "✓ Health checks:"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health)
if [ "$HTTP_CODE" = "200" ]; then
  echo "  analytics-service: healthy (http://localhost:8080)"
else
  echo "  ❌ analytics-service unreachable"
  exit 1
fi

FLINK_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8081)
if [ "$FLINK_CODE" = "200" ]; then
  echo "  flink-jobmanager: healthy (http://localhost:8081)"
else
  echo "  ❌ flink-jobmanager unreachable"
fi
echo

# Check Kafka topics
echo "✓ Kafka topics:"
TOPICS=$(docker exec kafka kafka-topics --bootstrap-server kafka:9092 --list 2>/dev/null | grep -E "^dex-" | wc -l)
if [ "$TOPICS" -ge 6 ]; then
  echo "  $TOPICS dex-* topics created"
else
  echo "  ❌ Expected 6 topics, found $TOPICS"
  exit 1
fi
echo

# Check ingester status
if docker compose ps | grep -q "ingester.*Up"; then
  echo "✓ Ingester: running (live chain data)"
else
  echo "ℹ Ingester: not running (use test data)"
  echo "  To start: docker compose --profile live up -d"
fi
echo

echo "✅ All core services ready!"
echo
echo "📊 Access points:"
echo "  Dashboard:  http://localhost:8080/dashboard"
echo "  Flink UI:   http://localhost:8081"
echo "  Health:     http://localhost:8080/health"
echo
echo "💡 Next: python3 scripts/produce_test_events.py"
