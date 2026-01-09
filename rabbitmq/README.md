# RabbitMQ Configuration for Order Book Exchange

## Quick Start

### Start RabbitMQ
```bash
docker-compose up -d rabbitmq
```

### Check Status
```bash
docker-compose ps
docker-compose logs -f rabbitmq
```

### Access Management UI
- URL: http://localhost:15672
- Username: `admin`
- Password: `admin`

### Stop RabbitMQ
```bash
docker-compose down
```

### Clean Volumes (removes all data)
```bash
docker-compose down -v
```

## Pre-configured Topology

### Exchanges
- **`market.data`** (topic): Main exchange for market data
- **`market.data.dlx`** (topic): Dead-letter exchange for failed messages

### Queues
- **`market.ticks`**: Stores market tick data
  - Max length: 1,000,000 messages
  - TTL: 24 hours
  - Dead-letter exchange: `market.data.dlx`

- **`market.ticks.dlq`**: Dead-letter queue for processing failures

### Routing Keys
- `tick.{symbol}` - Individual symbol ticks (e.g., `tick.EURUSD`, `tick.XAUUSD`)
- `tick.#` - All ticks (wildcard subscription)

## Message Format

Market ticks are published as JSON:

```json
{
  "symbol_id": "1",
  "symbol_name": "EURUSD",
  "bid_price": "1.08500",
  "ask_price": "1.08520",
  "bid_size": "1000000",
  "ask_size": "1500000",
  "timestamp": "2025-01-09T10:30:45.123456Z"
}
```

## Connection String

From Rust application:
```
amqp://admin:admin@localhost:5672/%2F
```

## Performance Tuning

### Memory Settings
- High watermark: 60% of available RAM
- Disk free limit: 1GB

### Channel Settings
- Max channels: 2047
- Heartbeat interval: 60 seconds

## Monitoring

### Via Management UI
1. Navigate to http://localhost:15672
2. Go to "Queues" tab to see message rates
3. Check "Connections" for active publishers/consumers

### Via CLI
```bash
# List queues
docker exec order-book-rabbitmq rabbitmqctl list_queues

# List exchanges
docker exec order-book-rabbitmq rabbitmqctl list_exchanges

# List bindings
docker exec order-book-rabbitmq rabbitmqctl list_bindings

# Connection info
docker exec order-book-rabbitmq rabbitmqctl list_connections
```

## Production Recommendations

1. **Change default credentials** in `docker-compose.yml`
2. **Enable TLS** for AMQP connections
3. **Configure clustering** for high availability
4. **Set up monitoring** with Prometheus/Grafana
5. **Adjust memory limits** based on message volume
6. **Enable persistent storage** with reliable volumes

## Troubleshooting

### Cannot connect to RabbitMQ
```bash
# Check if container is running
docker ps | grep rabbitmq

# Check logs
docker-compose logs rabbitmq

# Restart
docker-compose restart rabbitmq
```

### Memory issues
- Increase `vm_memory_high_watermark` in `rabbitmq.conf`
- Monitor memory usage in Management UI

### Message build-up
- Check consumer count
- Verify consumer processing speed
- Adjust prefetch count
