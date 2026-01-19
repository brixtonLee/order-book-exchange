#!/bin/bash
# Quick start script for monitoring stack

set -e

echo "🚀 Starting Observability Stack..."
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Start the stack
echo "📦 Starting containers..."
docker-compose up -d

echo ""
echo "⏳ Waiting for services to be ready..."
sleep 5

# Check service health
echo ""
echo "🔍 Checking service health..."

# Check Grafana
if curl -s http://localhost:3001/api/health > /dev/null; then
    echo "✅ Grafana is ready"
else
    echo "⚠️  Grafana is starting..."
fi

# Check Loki
if curl -s http://localhost:3100/ready > /dev/null; then
    echo "✅ Loki is ready"
else
    echo "⚠️  Loki is starting..."
fi

# Check Prometheus
if curl -s http://localhost:9090/-/ready > /dev/null; then
    echo "✅ Prometheus is ready"
else
    echo "⚠️  Prometheus is starting..."
fi

echo ""
echo "🎉 Monitoring stack is running!"
echo ""
echo "📊 Access the following UIs:"
echo "   Grafana:    http://localhost:3001 (admin/admin)"
echo "   Prometheus: http://localhost:9090"
echo "   Loki API:   http://localhost:3100"
echo ""
echo "📖 Read the README.md for learning guides and queries!"
echo ""
echo "🛑 To stop: docker-compose down"
echo "🗑️  To remove data: docker-compose down -v"
