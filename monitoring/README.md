# Observability Stack: Grafana + Loki + Promtail + Prometheus

Complete monitoring and logging setup for learning how application logs and metrics are collected, stored, and visualized.

## 📚 What You'll Learn

### 1. **Loki** - Log Aggregation
- How Loki differs from Elasticsearch (indexes labels, not content)
- Why label-based indexing is more cost-efficient
- How to design good label strategies

### 2. **Promtail** - Log Shipping
- How log collection agents work
- Push vs Pull models for logs
- How to parse logs and extract labels
- Pipeline stages and transformations

### 3. **Prometheus** - Metrics Collection
- How time-series metrics work
- Pull model for metrics (vs Loki's push model for logs)
- Counter, Gauge, Histogram, Summary metric types
- How to expose metrics from Rust applications

### 4. **Grafana** - Visualization
- LogQL query language (for logs)
- PromQL query language (for metrics)
- Building dashboards combining logs and metrics
- Setting up alerts

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Your Rust Application                       │
│  ┌────────────────────┐      ┌─────────────────────┐       │
│  │  Logs to stdout    │      │  /metrics endpoint  │       │
│  │  or file           │      │  (Prometheus format)│       │
│  └────────┬───────────┘      └─────────┬───────────┘       │
└───────────┼──────────────────────────────┼──────────────────┘
            │                              │
            │                              │
   ┌────────▼─────────┐          ┌────────▼─────────┐
   │    Promtail      │          │   Prometheus     │
   │  (Log Shipper)   │          │ (Metrics Scraper)│
   │                  │          │                  │
   │  • Tails logs    │          │  • Pulls metrics │
   │  • Adds labels   │          │  • Stores data   │
   │  • Pushes to     │          │  • Evaluates     │
   │    Loki          │          │    rules         │
   └────────┬─────────┘          └────────┬─────────┘
            │                              │
            │ HTTP POST                    │ Query
            │                              │
   ┌────────▼─────────┐          ┌────────▼─────────┐
   │      Loki        │          │                  │
   │ (Log Storage)    │          │                  │
   │                  │          │                  │
   │  • Stores logs   │          │                  │
   │  • Indexes       │◄─────────┤     Grafana      │
   │    labels only   │  Query   │  (Visualization) │
   │  • Compresses    │          │                  │
   │    content       │          │  • Dashboards    │
   └──────────────────┘          │  • Queries       │
                                 │  • Alerts        │
                                 └──────────────────┘
```

---

## 🚀 Quick Start

### 1. Start the Monitoring Stack

```bash
cd monitoring
docker-compose up -d
```

### 2. Verify Services are Running

```bash
docker-compose ps
```

You should see all services as "Up":
- grafana (port 3001)
- loki (port 3100)
- promtail (no exposed port)
- prometheus (port 9090)

### 3. Access the UIs

| Service    | URL                      | Credentials       |
|------------|--------------------------|-------------------|
| Grafana    | http://localhost:3001    | admin / admin     |
| Prometheus | http://localhost:9090    | No auth           |
| Loki       | http://localhost:3100    | API only          |

### 4. Check Logs

```bash
# View all logs
docker-compose logs -f

# View specific service
docker-compose logs -f loki
docker-compose logs -f promtail
```

---

## 📖 Learning Path

### Step 1: Understanding Loki (15 minutes)

**Concept**: Loki stores logs with **label-based indexing**.

**Try this**:
1. Open Grafana: http://localhost:3001
2. Login with admin/admin
3. Go to "Explore" (compass icon on left)
4. Select "Loki" datasource
5. Try these queries:

```logql
# Query 1: All logs from a service
{service="order-book"}

# Query 2: Filter by log level
{service="order-book", level="error"}

# Query 3: Search for text
{service="order-book"} |= "FIX connection"

# Query 4: Exclude text
{service="order-book"} != "heartbeat"

# Query 5: Count log lines per second
rate({service="order-book"}[1m])
```

**Key Learning**:
- `{...}` = Label selector (FAST - indexed)
- `|=` = Text search (SLOW - scans content)
- Labels are like database indexes - use them wisely!

### Step 2: Understanding Promtail (20 minutes)

**Concept**: Promtail is the **bridge** between your logs and Loki.

**Try this**:
1. Check Promtail config: `cat promtail/promtail-config.yml`
2. Notice the `scrape_configs` section
3. Check what Promtail sees:

```bash
# View Promtail logs
docker-compose logs promtail

# Check Promtail targets
curl http://localhost:9080/targets
```

**Experiment**: Add a Docker container with the label
```bash
# In your main docker-compose.yml, add to your app:
labels:
  logging: promtail
```

**Key Learning**:
- Promtail **PUSHES** logs to Loki (unlike Prometheus which PULLS)
- Pipeline stages can parse logs and extract labels
- Labels added by Promtail become queryable in Loki

### Step 3: Understanding Prometheus (20 minutes)

**Concept**: Prometheus **scrapes** metrics from HTTP endpoints.

**Try this**:
1. Open Prometheus: http://localhost:9090
2. Go to "Status" → "Targets"
3. See what endpoints Prometheus is scraping
4. Try these queries:

```promql
# Query 1: All Prometheus metrics
up

# Query 2: Prometheus scrape duration
prometheus_target_interval_length_seconds

# Query 3: Rate of scrapes
rate(prometheus_http_requests_total[1m])

# Query 4: Memory usage
process_resident_memory_bytes / 1024 / 1024
```

**Key Learning**:
- Prometheus **PULLS** metrics (HTTP GET /metrics)
- Metrics have types: Counter, Gauge, Histogram, Summary
- PromQL is different from LogQL

### Step 4: Integrating Your Rust App (30 minutes)

**Add Prometheus metrics to your Rust app**:

```bash
# Add to Cargo.toml
cargo add prometheus
```

**Add metrics endpoint** (example):

```rust
use prometheus::{Encoder, TextEncoder, Counter, register_counter};
use axum::{routing::get, Router};

// Define a counter metric
lazy_static::lazy_static! {
    static ref ORDERS_TOTAL: Counter =
        register_counter!("order_book_orders_total", "Total orders").unwrap();
}

// Increment the counter when an order is placed
ORDERS_TOTAL.inc();

// Expose /metrics endpoint
async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

// Add to your router
let app = Router::new()
    .route("/metrics", get(metrics_handler));
```

**Configure Promtail to collect your app logs**:

Edit `promtail/promtail-config.yml`:
```yaml
- job_name: order-book-app
  static_configs:
    - targets:
        - localhost
      labels:
        job: order-book
        service: order-book-api
        __path__: /app/logs/*.log
```

**Key Learning**:
- Metrics endpoint must return Prometheus text format
- Use counters for cumulative values (orders, requests)
- Use gauges for current values (queue size, connections)

### Step 5: Building a Dashboard (30 minutes)

**Create a comprehensive dashboard**:

1. In Grafana, click "+" → "Dashboard"
2. Add panels for:
   - **Logs Panel**: Show recent errors
   - **Graph Panel**: Order rate over time
   - **Stat Panel**: Total orders
   - **Table Panel**: Top symbols by volume

**Example Panel Queries**:

**Logs Panel (Loki)**:
```logql
{service="order-book", level="error"}
```

**Order Rate Graph (Prometheus)**:
```promql
rate(order_book_orders_total[1m])
```

**Total Orders Stat (Prometheus)**:
```promql
sum(order_book_orders_total)
```

**Key Learning**:
- Dashboards can combine multiple datasources
- Variables make dashboards reusable
- Time ranges affect query results

---

## 🔍 Common Queries Cheat Sheet

### LogQL (Loki)

```logql
# Basic filtering
{service="order-book"}
{service="order-book", level="error"}

# Text search
{service="order-book"} |= "error"
{service="order-book"} != "heartbeat"

# Regular expressions
{service="order-book"} |~ "error|warning"

# Parse JSON logs
{service="order-book"} | json

# Extract fields
{service="order-book"} | json | latency > 100

# Count rate
rate({service="order-book"}[1m])

# Top log producers
sum by (container) (count_over_time({job="docker"}[1m]))
```

### PromQL (Prometheus)

```promql
# Instant vector
order_book_orders_total

# Rate (per second)
rate(order_book_orders_total[1m])

# Sum by label
sum by (symbol) (order_book_orders_total)

# Histogram quantile (95th percentile)
histogram_quantile(0.95, rate(order_book_latency_bucket[5m]))

# Increase over time
increase(order_book_orders_total[1h])

# Prediction (linear regression)
predict_linear(order_book_orders_total[1h], 3600)
```

---

## 🎯 Best Practices

### Label Design

**Good Labels** (low cardinality):
```
{service="order-book", level="error", environment="production"}
```

**Bad Labels** (high cardinality):
```
{service="order-book", order_id="uuid-123-456"}  # DON'T DO THIS
```

**Why?** Each unique label combination creates a new stream. High cardinality = poor performance.

### Log Format

**Prefer JSON logs** for easy parsing:
```json
{"timestamp":"2024-01-01T12:00:00Z","level":"INFO","message":"Order placed","symbol":"XAUUSD","order_id":"123"}
```

Configure Rust tracing:
```rust
tracing_subscriber::fmt()
    .json()
    .init();
```

### Retention

Edit retention in `loki/loki-config.yml`:
```yaml
limits_config:
  retention_period: 168h  # 7 days
```

---

## 🐛 Troubleshooting

### Promtail not sending logs

```bash
# Check Promtail logs
docker-compose logs promtail

# Check Promtail targets
curl http://localhost:9080/targets

# Check if Loki is reachable
docker-compose exec promtail wget -O- http://loki:3100/ready
```

### Prometheus not scraping

```bash
# Check Prometheus targets
# Go to http://localhost:9090/targets

# Check if endpoint is reachable
curl http://your-app:3000/metrics
```

### No logs appearing in Grafana

1. Check datasource: Configuration → Data Sources → Loki → "Test"
2. Check Promtail is running: `docker-compose ps promtail`
3. Check Loki logs: `docker-compose logs loki`
4. Try broader query: `{job=~".+"}`

---

## 🧪 Experiments to Try

### 1. Label Cardinality Experiment
Add a high-cardinality label and watch Loki performance degrade.

### 2. Log Volume Test
Generate 10,000 log lines per second and see how Loki handles it.

### 3. Query Performance
Compare text search `|=` vs label filtering `{level="error"}`.

### 4. Alerting
Set up an alert when error rate exceeds threshold.

### 5. Multi-tenancy
Enable `auth_enabled: true` in Loki and use tenant IDs.

---

## 📚 Further Reading

- [Loki Documentation](https://grafana.com/docs/loki/latest/)
- [LogQL Guide](https://grafana.com/docs/loki/latest/logql/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [PromQL Guide](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Grafana Tutorials](https://grafana.com/tutorials/)

---

## 🛠️ Maintenance

### View logs
```bash
docker-compose logs -f [service]
```

### Restart services
```bash
docker-compose restart [service]
```

### Stop everything
```bash
docker-compose down
```

### Clean up (delete data)
```bash
docker-compose down -v
```

### Update images
```bash
docker-compose pull
docker-compose up -d
```

---

## 🎓 Learning Checklist

- [ ] Understand the difference between logs and metrics
- [ ] Know when to use labels vs text search
- [ ] Can write basic LogQL queries
- [ ] Can write basic PromQL queries
- [ ] Understand push (Loki) vs pull (Prometheus) models
- [ ] Can configure Promtail to collect logs
- [ ] Can expose Prometheus metrics from Rust
- [ ] Can build a Grafana dashboard
- [ ] Understand label cardinality and its impact
- [ ] Can set up alerts in Grafana

---

## 💡 Next Steps

1. **Add more metrics** to your Rust app (latency, queue depth, etc.)
2. **Create alerts** for critical conditions
3. **Add Tempo** for distributed tracing (traces + logs + metrics)
4. **Add Alertmanager** for advanced alerting
5. **Explore Grafana plugins** for specialized visualizations

Happy Learning! 🚀
