#!/bin/bash
# Stop script for monitoring stack

echo "🛑 Stopping Observability Stack..."
docker-compose down

echo ""
echo "✅ All services stopped"
echo ""
echo "💡 To remove all data: docker-compose down -v"
