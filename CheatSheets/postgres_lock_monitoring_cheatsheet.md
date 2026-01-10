# PostgreSQL Lock Monitoring Cheatsheet

**Author:** Brixton  
**Purpose:** Monitor lock contention in PostgreSQL/TimescaleDB for HFT systems  
**Target Latency:** Sub-millisecond to 15ms  
**Last Updated:** 2026-01-06

---

## Table of Contents

1. [Quick Reference](#quick-reference)
2. [Critical Monitoring Queries](#critical-monitoring-queries)
3. [Advanced Diagnostics](#advanced-diagnostics)
4. [Historical Analysis](#historical-analysis)
5. [C# Integration Examples](#c-integration-examples)
6. [Emergency Procedures](#emergency-procedures)
7. [Monitoring Thresholds](#monitoring-thresholds)
8. [Lock Types Reference](#lock-types-reference)

---

## Quick Reference

### One-Liner Health Checks

```sql
-- Are locks blocking anything RIGHT NOW?
SELECT COUNT(*) FROM pg_stat_activity WHERE wait_event_type = 'Lock';

-- Who's the top blocker?
SELECT blocking.pid, blocking.query 
FROM pg_stat_activity blocked
JOIN pg_stat_activity blocking ON blocking.pid = ANY(pg_blocking_pids(blocked.pid))
GROUP BY blocking.pid, blocking.query 
ORDER BY COUNT(*) DESC LIMIT 1;

-- Current deadlock count
SELECT deadlocks FROM pg_stat_database WHERE datname = current_database();
```

---

## Critical Monitoring Queries

### 1. Current Blocking Queries (MOST CRITICAL)

**Use Case:** Real-time detection when latency spikes occur  
**Run Frequency:** Every 1-5 seconds during market hours  
**Alert Threshold:** ANY result = investigate immediately

```sql
-- Who's blocking whom RIGHT NOW?
SELECT 
    blocked.pid AS blocked_pid,
    blocked.usename AS blocked_user,
    blocked.application_name AS blocked_app,
    blocked.client_addr AS blocked_client,
    now() - blocked.query_start AS blocked_duration,
    blocked.state AS blocked_state,
    blocked.query AS blocked_query,
    
    blocking.pid AS blocking_pid,
    blocking.usename AS blocking_user,
    blocking.application_name AS blocking_app,
    now() - blocking.query_start AS blocking_duration,
    blocking.state AS blocking_state,
    blocking.query AS blocking_query,
    
    blocking.wait_event_type AS blocking_wait_event_type,
    blocking.wait_event AS blocking_wait_event
FROM pg_stat_activity AS blocked
JOIN pg_stat_activity AS blocking 
    ON blocking.pid = ANY(pg_blocking_pids(blocked.pid))
WHERE blocked.pid != pg_backend_pid()
ORDER BY blocked_duration DESC;
```

**What to look for:**
- `blocked_duration` > 100ms → Critical for HFT
- `blocking_state` = 'idle in transaction' → Long-running transaction holding locks
- Same `blocking_pid` appearing multiple times → One transaction blocking many

---

### 2. Lock Wait Summary

**Use Case:** Quick health check dashboard  
**Run Frequency:** Every 5-10 seconds  
**Alert Threshold:** `waiting_queries` > 0

```sql
-- How many queries are waiting on locks?
SELECT 
    wait_event_type,
    wait_event,
    COUNT(*) AS waiting_queries,
    MIN(now() - query_start) AS min_wait_time,
    MAX(now() - query_start) AS max_wait_time,
    AVG(now() - query_start) AS avg_wait_time
FROM pg_stat_activity 
WHERE wait_event_type = 'Lock'
    AND pid != pg_backend_pid()
GROUP BY wait_event_type, wait_event
ORDER BY waiting_queries DESC;
```

**Interpretation:**
- `wait_event = 'relation'` → Table-level lock wait
- `wait_event = 'tuple'` → Row-level lock wait (common in HFT)
- `wait_event = 'transactionid'` → Waiting for transaction to complete
- `max_wait_time` > '100ms' → Investigate immediately

---

### 3. Current Lock Table Snapshot

**Use Case:** Deep dive when investigating specific lock issues  
**Run Frequency:** On-demand  
**Alert Threshold:** N/A (diagnostic)

```sql
-- What locks exist right now?
SELECT 
    pl.locktype,
    pl.database,
    pl.relation::regclass AS table_name,
    pl.page,
    pl.tuple,
    pl.virtualxid,
    pl.transactionid,
    pl.mode,
    pl.granted,
    pl.fastpath,
    sa.pid,
    sa.usename,
    sa.application_name,
    sa.client_addr,
    sa.backend_start,
    sa.xact_start,
    sa.query_start,
    sa.state,
    sa.backend_xid,
    sa.backend_xmin,
    sa.query
FROM pg_locks pl
LEFT JOIN pg_stat_activity sa ON pl.pid = sa.pid
WHERE pl.pid != pg_backend_pid()
ORDER BY pl.granted, sa.query_start;
```

**What to look for:**
- `granted = false` → Lock request waiting
- `mode = 'AccessExclusiveLock'` during market hours → CRITICAL (blocks everything)
- High count of locks on same `table_name` → Contention hotspot

---

### 4. Locks by Table (Find Hot Tables)

**Use Case:** Identify which tables are contention hotspots  
**Run Frequency:** Every 30-60 seconds  
**Alert Threshold:** `waiting_count` > 0 on critical tables

```sql
-- Which tables have the most lock activity?
SELECT 
    pl.relation::regclass AS table_name,
    pl.mode,
    COUNT(*) AS lock_count,
    COUNT(*) FILTER (WHERE NOT pl.granted) AS waiting_count,
    COUNT(*) FILTER (WHERE pl.granted) AS granted_count
FROM pg_locks pl
WHERE pl.relation IS NOT NULL
    AND pl.pid != pg_backend_pid()
GROUP BY pl.relation, pl.mode
ORDER BY lock_count DESC, waiting_count DESC;
```

**Expected hot tables in your system:**
- `market_data` / `ticks` → High insert volume
- `positions` / `accounts` → Frequent updates
- `orders` / `trades` → Transaction-heavy

**Action items:**
- If `market_data` has high `waiting_count` → Check batch insert patterns
- If `positions` has row locks → Review transaction duration
- If `accounts` has exclusive locks → Investigate long transactions

---

### 5. Long-Running Transactions Holding Locks

**Use Case:** Find root cause of lock contention  
**Run Frequency:** Every 10-30 seconds  
**Alert Threshold:** `transaction_duration` > 5 seconds

```sql
-- Transactions open for too long (configurable threshold)
SELECT 
    pid,
    usename,
    application_name,
    client_addr,
    backend_start,
    xact_start,
    now() - xact_start AS transaction_duration,
    state,
    wait_event_type,
    wait_event,
    query
FROM pg_stat_activity
WHERE xact_start IS NOT NULL
    AND now() - xact_start > interval '5 seconds'  -- Adjust threshold for HFT
    AND pid != pg_backend_pid()
ORDER BY xact_start;
```

**For HFT systems, adjust threshold:**
- Warning: > 1 second
- Critical: > 5 seconds
- Emergency: > 10 seconds (likely causing cascading issues)

**Common causes:**
- Forgotten `BEGIN` without `COMMIT`
- Long-running reports during market hours
- Network issues causing client disconnection
- Application crashes leaving transactions open

---

### 6. Real-Time Lock Wait Detection

**Use Case:** Monitoring loop for alerting  
**Run Frequency:** Every 1-5 seconds in production  
**Alert Threshold:** ANY row returned

```sql
-- Queries currently waiting on locks with details
SELECT 
    sa.pid,
    sa.usename,
    sa.application_name,
    sa.client_addr,
    sa.wait_event_type,
    sa.wait_event,
    now() - sa.query_start AS wait_duration,
    sa.state,
    sa.query,
    pl.locktype,
    pl.mode AS lock_mode,
    pl.granted,
    pl.relation::regclass AS locked_table
FROM pg_stat_activity sa
LEFT JOIN pg_locks pl ON sa.pid = pl.pid
WHERE sa.wait_event_type = 'Lock'
    AND sa.pid != pg_backend_pid()
ORDER BY wait_duration DESC;
```

**Alert message should include:**
- PID of waiting query
- Wait duration
- Locked table name
- Application name (CCP vs RTG)
- Client address

---

## Advanced Diagnostics

### 7. Deadlock History

**Use Case:** Daily review and trending  
**Run Frequency:** Once per hour or daily  
**Alert Threshold:** `deadlocks_per_hour` > 1

```sql
-- Deadlock count since last stats reset
SELECT 
    datname,
    deadlocks,
    stats_reset,
    now() - stats_reset AS time_since_reset,
    ROUND(deadlocks::numeric / EXTRACT(EPOCH FROM (now() - stats_reset)) * 3600, 2) AS deadlocks_per_hour
FROM pg_stat_database 
WHERE datname = current_database();
```

**What deadlocks indicate:**
- Transaction ordering issues in application code
- Need for explicit lock ordering (always lock tables/rows in same order)
- Possible need for `SELECT FOR UPDATE` to acquire locks early

**To reset stats (for testing):**
```sql
SELECT pg_stat_reset_single_table_counters(oid) 
FROM pg_class WHERE relname = 'your_table';
```

---

### 8. Blocking Tree (Full Hierarchy)

**Use Case:** Complex blocking scenarios (cascade blocking)  
**Run Frequency:** On-demand when investigating complex issues  
**Alert Threshold:** `level` > 2 (multi-level blocking)

```sql
-- Complete blocking chain including nested blocks
WITH RECURSIVE blocking_tree AS (
    -- Find all blocked sessions
    SELECT 
        activity.pid,
        activity.usename,
        activity.query,
        activity.query_start,
        now() - activity.query_start AS duration,
        blocking.pid AS blocking_pid,
        1 AS level
    FROM pg_stat_activity activity
    JOIN pg_stat_activity blocking 
        ON blocking.pid = ANY(pg_blocking_pids(activity.pid))
    WHERE activity.pid != pg_backend_pid()
    
    UNION ALL
    
    -- Recursively find what's blocking the blockers
    SELECT 
        activity.pid,
        activity.usename,
        activity.query,
        activity.query_start,
        now() - activity.query_start,
        blocking.pid,
        bt.level + 1
    FROM blocking_tree bt
    JOIN pg_stat_activity activity ON activity.pid = bt.blocking_pid
    JOIN pg_stat_activity blocking 
        ON blocking.pid = ANY(pg_blocking_pids(activity.pid))
)
SELECT 
    level,
    pid,
    usename,
    duration,
    blocking_pid,
    LEFT(query, 80) AS query_snippet
FROM blocking_tree
ORDER BY level, duration DESC;
```

**Example output:**
```
level | pid  | duration | blocking_pid | query_snippet
------|------|----------|--------------|---------------
1     | 1234 | 00:00:05 | 5678         | UPDATE positions SET...
1     | 2345 | 00:00:03 | 5678         | INSERT INTO trades...
2     | 5678 | 00:01:30 | 9012         | BEGIN; UPDATE accounts...
3     | 9012 | 00:05:00 | NULL         | VACUUM FULL...
```

This shows PID 9012 is blocking 5678, which is blocking 1234 and 2345.

---

### 9. Lock Wait Events (Detailed)

**Use Case:** Understanding what type of locks are problematic  
**Run Frequency:** Every minute  
**Alert Threshold:** Trending increase in specific `wait_event`

```sql
-- What specific lock types are causing waits?
SELECT 
    wait_event,
    COUNT(*) AS count,
    AVG(EXTRACT(EPOCH FROM (now() - query_start))) AS avg_wait_seconds,
    MAX(EXTRACT(EPOCH FROM (now() - query_start))) AS max_wait_seconds,
    ARRAY_AGG(DISTINCT application_name) AS affected_apps
FROM pg_stat_activity
WHERE wait_event_type = 'Lock'
    AND pid != pg_backend_pid()
GROUP BY wait_event
ORDER BY count DESC;
```

**Common wait events:**
- `relation` → Waiting for table-level lock (DDL, TRUNCATE, etc.)
- `tuple` → Waiting for row-level lock (UPDATE/DELETE conflicts)
- `transactionid` → Waiting for transaction to commit/rollback
- `extend` → Waiting to extend relation (table growth contention)
- `page` → Waiting for page lock (rare, usually vacuum-related)

---

## Historical Analysis

### 10. Query Performance with Lock Context

**Use Case:** Historical analysis of slow queries  
**Run Frequency:** Daily or weekly review  
**Prerequisites:** Requires `pg_stat_statements` extension

```sql
-- Enable extension (run once)
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Historical view of slow queries
SELECT 
    queryid,
    LEFT(query, 100) AS query_snippet,
    calls,
    ROUND(mean_exec_time::numeric, 2) AS mean_ms,
    ROUND(stddev_exec_time::numeric, 2) AS stddev_ms,
    ROUND(min_exec_time::numeric, 2) AS min_ms,
    ROUND(max_exec_time::numeric, 2) AS max_ms,
    ROUND((total_exec_time / 1000)::numeric, 2) AS total_seconds,
    rows
FROM pg_stat_statements
WHERE mean_exec_time > 10  -- Queries averaging > 10ms
ORDER BY mean_exec_time DESC
LIMIT 20;

-- Check when stats were last reset
SELECT stats_reset FROM pg_stat_statements_info;
```

**Limitations:**
- Doesn't directly show lock wait time (shows total execution time)
- Slow queries with high `stddev_ms` might indicate intermittent lock contention
- Use in conjunction with application-level metrics

**Reset stats:**
```sql
SELECT pg_stat_statements_reset();
```

---

### 11. TimescaleDB-Specific Lock Monitoring

**Use Case:** Monitor locks on hypertables and chunks  
**Run Frequency:** Every 30 seconds  
**Alert Threshold:** Locks on recent chunks during insert-heavy periods

```sql
-- Locks on TimescaleDB hypertable chunks
SELECT 
    h.table_name AS hypertable,
    c.chunk_name,
    c.range_start,
    c.range_end,
    pl.mode,
    pl.granted,
    COUNT(*) AS lock_count
FROM pg_locks pl
JOIN pg_class pc ON pl.relation = pc.oid
JOIN timescaledb_information.chunks c ON pc.relname = c.chunk_name
JOIN timescaledb_information.hypertables h ON c.hypertable_name = h.table_name
WHERE pl.pid != pg_backend_pid()
GROUP BY h.table_name, c.chunk_name, c.range_start, c.range_end, pl.mode, pl.granted
ORDER BY h.table_name, c.range_start DESC;
```

**What to look for:**
- Locks on most recent chunk during market data ingestion
- `AccessExclusiveLock` on chunks (compression, reordering)
- High lock count on specific chunks

---

### 12. Continuous Aggregate Lock Impact

**Use Case:** Monitor lock impact of continuous aggregate refreshes  
**Run Frequency:** During and after refresh operations  
**Alert Threshold:** Locks blocking real-time queries

```sql
-- Find continuous aggregate refresh jobs and their locks
SELECT 
    ca.view_name,
    ca.materialization_hypertable_name,
    j.job_id,
    j.last_run_status,
    j.next_start,
    pl.mode,
    pl.granted,
    COUNT(*) AS lock_count
FROM timescaledb_information.continuous_aggregates ca
JOIN timescaledb_information.jobs j ON j.proc_name = 'policy_refresh_continuous_aggregate'
LEFT JOIN pg_locks pl ON pl.relation = (
    SELECT oid FROM pg_class WHERE relname = ca.materialization_hypertable_name
)
WHERE pl.pid != pg_backend_pid()
GROUP BY ca.view_name, ca.materialization_hypertable_name, j.job_id, 
         j.last_run_status, j.next_start, pl.mode, pl.granted
ORDER BY ca.view_name;
```

**Best practices:**
- Schedule continuous aggregate refreshes during off-market hours
- Use shorter refresh windows to minimize lock duration
- Monitor `last_run_status` for failures

---

## C# Integration Examples

### Basic Lock Health Monitor

```csharp
using Npgsql;
using Dapper;

public class PostgresLockMonitor
{
    private readonly NpgsqlDataSource _dataSource;
    private readonly ILogger<PostgresLockMonitor> _logger;

    public PostgresLockMonitor(NpgsqlDataSource dataSource, ILogger<PostgresLockMonitor> logger)
    {
        _dataSource = dataSource;
        _logger = logger;
    }

    public async Task<LockHealthReport> CheckLockHealthAsync()
    {
        await using var conn = await _dataSource.OpenConnectionAsync();
        
        // Query 2: Quick health check
        var waitingSummary = await conn.QueryAsync<LockWaitSummary>(@"
            SELECT 
                wait_event_type AS WaitEventType,
                wait_event AS WaitEvent,
                COUNT(*) AS WaitingQueries,
                MAX(EXTRACT(EPOCH FROM (now() - query_start))) AS MaxWaitSeconds
            FROM pg_stat_activity 
            WHERE wait_event_type = 'Lock'
                AND pid != pg_backend_pid()
            GROUP BY wait_event_type, wait_event
        ");

        var waitList = waitingSummary.ToList();
        
        if (waitList.Any())
        {
            _logger.LogWarning(
                "Lock contention detected: {Count} queries waiting, max wait {MaxWait:F3}s",
                waitList.Sum(s => s.WaitingQueries),
                waitList.Max(s => s.MaxWaitSeconds)
            );

            // Query 1: Get blocking details for alert
            var blockingDetails = await conn.QueryAsync<BlockingQuery>(@"
                SELECT 
                    blocked.pid AS BlockedPid,
                    EXTRACT(EPOCH FROM (now() - blocked.query_start)) AS BlockedDuration,
                    blocked.query AS BlockedQuery,
                    blocked.application_name AS BlockedApp,
                    blocking.pid AS BlockingPid,
                    blocking.query AS BlockingQuery,
                    blocking.application_name AS BlockingApp
                FROM pg_stat_activity AS blocked
                JOIN pg_stat_activity AS blocking 
                    ON blocking.pid = ANY(pg_blocking_pids(blocked.pid))
                WHERE blocked.pid != pg_backend_pid()
                ORDER BY BlockedDuration DESC
                LIMIT 5
            ");

            return new LockHealthReport
            {
                IsHealthy = false,
                WaitingQueryCount = waitList.Sum(s => s.WaitingQueries),
                MaxWaitSeconds = waitList.Max(s => s.MaxWaitSeconds),
                BlockingDetails = blockingDetails.ToList()
            };
        }

        return new LockHealthReport { IsHealthy = true };
    }

    public async Task<Dictionary<string, int>> GetTableLockCountsAsync()
    {
        await using var conn = await _dataSource.OpenConnectionAsync();
        
        // Query 4: Lock counts by table
        var tableLocks = await conn.QueryAsync<(string TableName, int LockCount)>(@"
            SELECT 
                pl.relation::regclass::text AS TableName,
                COUNT(*) AS LockCount
            FROM pg_locks pl
            WHERE pl.relation IS NOT NULL
                AND pl.pid != pg_backend_pid()
            GROUP BY pl.relation
            ORDER BY LockCount DESC
        ");

        return tableLocks.ToDictionary(t => t.TableName, t => t.LockCount);
    }

    public async Task<int> GetDeadlockCountAsync()
    {
        await using var conn = await _dataSource.OpenConnectionAsync();
        
        var deadlocks = await conn.QuerySingleAsync<int>(@"
            SELECT deadlocks 
            FROM pg_stat_database 
            WHERE datname = current_database()
        ");

        return deadlocks;
    }
}

// Data models
public class LockWaitSummary
{
    public string WaitEventType { get; set; }
    public string WaitEvent { get; set; }
    public int WaitingQueries { get; set; }
    public double MaxWaitSeconds { get; set; }
}

public class BlockingQuery
{
    public int BlockedPid { get; set; }
    public double BlockedDuration { get; set; }
    public string BlockedQuery { get; set; }
    public string BlockedApp { get; set; }
    public int BlockingPid { get; set; }
    public string BlockingQuery { get; set; }
    public string BlockingApp { get; set; }
}

public class LockHealthReport
{
    public bool IsHealthy { get; set; }
    public int WaitingQueryCount { get; set; }
    public double MaxWaitSeconds { get; set; }
    public List<BlockingQuery> BlockingDetails { get; set; } = new();
}
```

---

### Background Monitoring Service

```csharp
using Microsoft.Extensions.Hosting;

public class LockMonitoringService : BackgroundService
{
    private readonly PostgresLockMonitor _monitor;
    private readonly ILarkNotifier _larkNotifier;
    private readonly ILogger<LockMonitoringService> _logger;
    private readonly TimeSpan _checkInterval = TimeSpan.FromSeconds(5);
    private int _lastDeadlockCount = 0;

    public LockMonitoringService(
        PostgresLockMonitor monitor,
        ILarkNotifier larkNotifier,
        ILogger<LockMonitoringService> logger)
    {
        _monitor = monitor;
        _larkNotifier = larkNotifier;
        _logger = logger;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        _logger.LogInformation("Lock monitoring service started");

        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                await CheckLockHealthAsync();
                await CheckDeadlocksAsync();
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error in lock monitoring loop");
            }

            await Task.Delay(_checkInterval, stoppingToken);
        }
    }

    private async Task CheckLockHealthAsync()
    {
        var health = await _monitor.CheckLockHealthAsync();
        
        if (!health.IsHealthy)
        {
            var severity = health.MaxWaitSeconds switch
            {
                > 1.0 => "critical",
                > 0.1 => "warning",
                _ => "info"
            };

            if (health.MaxWaitSeconds > 0.1) // 100ms threshold
            {
                var blockingInfo = health.BlockingDetails.FirstOrDefault();
                var message = $"🔒 PostgreSQL Lock Contention Detected\n\n" +
                             $"Waiting Queries: {health.WaitingQueryCount}\n" +
                             $"Max Wait Time: {health.MaxWaitSeconds:F3}s\n";

                if (blockingInfo != null)
                {
                    message += $"\nTop Blocker:\n" +
                              $"  PID: {blockingInfo.BlockingPid}\n" +
                              $"  App: {blockingInfo.BlockingApp}\n" +
                              $"  Query: {TruncateQuery(blockingInfo.BlockingQuery)}\n\n" +
                              $"Blocked:\n" +
                              $"  PID: {blockingInfo.BlockedPid}\n" +
                              $"  App: {blockingInfo.BlockedApp}\n" +
                              $"  Duration: {blockingInfo.BlockedDuration:F3}s\n" +
                              $"  Query: {TruncateQuery(blockingInfo.BlockedQuery)}";
                }

                await _larkNotifier.SendAlertAsync(new LarkAlert
                {
                    Title = "⚠️ PostgreSQL Lock Contention",
                    Content = message,
                    Severity = severity
                });

                _logger.LogWarning(
                    "Lock contention: {Count} waiting, max {MaxWait:F3}s, blocker PID {BlockerPid}",
                    health.WaitingQueryCount,
                    health.MaxWaitSeconds,
                    blockingInfo?.BlockingPid ?? 0
                );
            }
        }
    }

    private async Task CheckDeadlocksAsync()
    {
        var currentDeadlocks = await _monitor.GetDeadlockCountAsync();
        
        if (currentDeadlocks > _lastDeadlockCount)
        {
            var newDeadlocks = currentDeadlocks - _lastDeadlockCount;
            
            await _larkNotifier.SendAlertAsync(new LarkAlert
            {
                Title = "💀 PostgreSQL Deadlock Detected",
                Content = $"New deadlocks detected: {newDeadlocks}\n" +
                         $"Total deadlocks since restart: {currentDeadlocks}\n\n" +
                         $"Action: Review transaction ordering in application code",
                Severity = "warning"
            });

            _logger.LogWarning("Deadlocks detected: {NewCount} new, {Total} total", 
                newDeadlocks, currentDeadlocks);
        }

        _lastDeadlockCount = currentDeadlocks;
    }

    private static string TruncateQuery(string query, int maxLength = 100)
    {
        if (string.IsNullOrEmpty(query)) return "(empty)";
        return query.Length > maxLength 
            ? query.Substring(0, maxLength) + "..." 
            : query;
    }
}

// Register in Program.cs
builder.Services.AddSingleton<PostgresLockMonitor>();
builder.Services.AddHostedService<LockMonitoringService>();
```

---

### Metrics Export for Prometheus/Grafana

```csharp
using Prometheus;

public class PostgresLockMetrics
{
    private static readonly Gauge WaitingQueries = Metrics
        .CreateGauge("postgres_lock_waiting_queries", 
            "Number of queries waiting on locks");
    
    private static readonly Gauge MaxLockWaitSeconds = Metrics
        .CreateGauge("postgres_lock_max_wait_seconds", 
            "Maximum lock wait time in seconds");
    
    private static readonly Gauge DeadlocksTotal = Metrics
        .CreateGauge("postgres_deadlocks_total", 
            "Total number of deadlocks");
    
    private static readonly Counter DeadlocksCounter = Metrics
        .CreateCounter("postgres_deadlocks_count", 
            "Incremental count of deadlocks");

    private readonly PostgresLockMonitor _monitor;

    public PostgresLockMetrics(PostgresLockMonitor monitor)
    {
        _monitor = monitor;
    }

    public async Task UpdateMetricsAsync()
    {
        var health = await _monitor.CheckLockHealthAsync();
        var deadlocks = await _monitor.GetDeadlockCountAsync();

        WaitingQueries.Set(health.WaitingQueryCount);
        MaxLockWaitSeconds.Set(health.MaxWaitSeconds);
        DeadlocksTotal.Set(deadlocks);
    }
}

// Update metrics in monitoring loop
public class MetricsUpdateService : BackgroundService
{
    private readonly PostgresLockMetrics _metrics;
    private readonly TimeSpan _updateInterval = TimeSpan.FromSeconds(10);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                await _metrics.UpdateMetricsAsync();
            }
            catch (Exception ex)
            {
                // Log but don't crash
            }
            
            await Task.Delay(_updateInterval, stoppingToken);
        }
    }
}
```

---

## Emergency Procedures

### Identify and Kill Blocking Query

```sql
-- Step 1: Identify the blocker
SELECT 
    blocking.pid AS blocking_pid,
    blocking.usename,
    blocking.application_name,
    blocking.client_addr,
    now() - blocking.query_start AS duration,
    blocking.state,
    blocking.query,
    COUNT(blocked.pid) AS blocked_count
FROM pg_stat_activity blocking
JOIN pg_stat_activity blocked 
    ON blocking.pid = ANY(pg_blocking_pids(blocked.pid))
WHERE blocking.pid != pg_backend_pid()
GROUP BY blocking.pid, blocking.usename, blocking.application_name, 
         blocking.client_addr, blocking.query_start, blocking.state, blocking.query
ORDER BY blocked_count DESC, duration DESC
LIMIT 1;

-- Step 2: Try to cancel gracefully (preferred)
SELECT pg_cancel_backend(12345);  -- Replace with actual PID

-- Step 3: If cancel doesn't work, terminate forcefully
SELECT pg_terminate_backend(12345);  -- Replace with actual PID

-- Step 4: Verify it's gone
SELECT pid, state, query 
FROM pg_stat_activity 
WHERE pid = 12345;
```

### Kill All Idle Transactions

**⚠️ DANGER:** Use with extreme caution, only in emergencies

```sql
-- Find all idle transactions older than 5 minutes
SELECT 
    pid,
    usename,
    application_name,
    now() - xact_start AS transaction_age,
    state,
    query
FROM pg_stat_activity
WHERE state = 'idle in transaction'
    AND now() - xact_start > interval '5 minutes'
    AND pid != pg_backend_pid();

-- Kill them (BE CAREFUL!)
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle in transaction'
    AND now() - xact_start > interval '5 minutes'
    AND pid != pg_backend_pid()
    AND application_name != 'critical_app_name';  -- Protect critical apps
```

### Emergency: Kill All Connections to Database

**⚠️ EXTREME DANGER:** Only use during total system failure

```sql
-- Kick everyone out except superusers
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname = 'your_trading_db'
    AND pid != pg_backend_pid()
    AND usename != 'postgres';  -- Don't kill superuser connections
```

---

## Monitoring Thresholds

### Recommended Thresholds for HFT Systems

| Metric | Normal | Warning | Critical | Action |
|--------|--------|---------|----------|--------|
| **Lock Wait Time** | 0ms | > 50ms | > 100ms | Alert to Lark immediately |
| **Waiting Query Count** | 0 | > 0 | > 5 | Investigate blocker |
| **Long Transaction** | < 100ms | > 1s | > 5s | Kill if non-critical |
| **Deadlocks/Hour** | 0 | > 1 | > 10 | Review transaction logic |
| **Blocked Duration** | N/A | > 100ms | > 1s | Kill blocker if safe |
| **Lock Count per Table** | < 100 | > 500 | > 1000 | Check for lock storm |

### Table-Specific Thresholds

For your trading system tables:

```
market_data / ticks:
  - Lock wait: Critical if > 10ms (high insert rate)
  - Long transaction: Critical if > 1s
  
positions / accounts:
  - Lock wait: Critical if > 50ms (affects order execution)
  - Long transaction: Critical if > 500ms
  
orders / trades:
  - Lock wait: Critical if > 15ms (your SLA)
  - Long transaction: Critical if > 100ms
```

### Alert Escalation

```
Level 1 (Log Only):
  - Lock wait 0-50ms
  - Waiting queries < 3
  
Level 2 (Lark Warning):
  - Lock wait 50-100ms
  - Waiting queries 3-10
  - Deadlock detected
  
Level 3 (Lark Critical):
  - Lock wait > 100ms
  - Waiting queries > 10
  - Transaction > 5s during market hours
  - Any AccessExclusiveLock during market hours
  
Level 4 (PagerDuty):
  - Lock wait > 1s
  - System-wide blocking (> 50 waiting queries)
  - Continuous aggregate refresh stuck
```

---

## Lock Types Reference

### Table-Level Locks (Strongest to Weakest)

| Lock Mode | Acquired By | Conflicts With | Use Case |
|-----------|-------------|----------------|----------|
| **ACCESS EXCLUSIVE** | `DROP TABLE`, `TRUNCATE`, `VACUUM FULL`, `ALTER TABLE` | Everything | Schema changes - avoid during market hours |
| **EXCLUSIVE** | `REFRESH MATERIALIZED VIEW` | All except ACCESS SHARE | Rarely used |
| **SHARE ROW EXCLUSIVE** | Certain `ALTER` operations | SHARE and higher | Rare |
| **SHARE** | `CREATE INDEX` | ROW EXCLUSIVE and higher | Index creation |
| **SHARE UPDATE EXCLUSIVE** | `VACUUM`, `ANALYZE`, `CREATE INDEX CONCURRENTLY` | Itself and higher | Maintenance |
| **ROW EXCLUSIVE** | `INSERT`, `UPDATE`, `DELETE` | SHARE and higher | Normal DML |
| **ROW SHARE** | `SELECT FOR UPDATE/SHARE` | EXCLUSIVE and higher | Row locking |
| **ACCESS SHARE** | `SELECT` | ACCESS EXCLUSIVE only | Normal reads |

### Row-Level Locks

| Lock Mode | SQL Command | Conflicts With | Use Case |
|-----------|-------------|----------------|----------|
| **FOR UPDATE** | `SELECT ... FOR UPDATE` | FOR UPDATE, FOR NO KEY UPDATE, FOR SHARE, FOR KEY SHARE | Exclusive row lock |
| **FOR NO KEY UPDATE** | `UPDATE` (default) | FOR UPDATE, FOR NO KEY UPDATE, FOR SHARE | Update non-key columns |
| **FOR SHARE** | `SELECT ... FOR SHARE` | FOR UPDATE, FOR NO KEY UPDATE | Prevent updates |
| **FOR KEY SHARE** | Foreign key checks | FOR UPDATE only | Weakest lock |

### Lock Compatibility Matrix

```
Current Lock          | Can Acquire?
----------------------|------------------
ACCESS SHARE          | Everything except ACCESS EXCLUSIVE
ROW SHARE             | Everything except EXCLUSIVE, ACCESS EXCLUSIVE
ROW EXCLUSIVE         | ACCESS SHARE, ROW SHARE, ROW EXCLUSIVE
SHARE UPDATE EXCLUSIVE| ACCESS SHARE, ROW SHARE
SHARE                 | ACCESS SHARE, ROW SHARE, SHARE
SHARE ROW EXCLUSIVE   | ACCESS SHARE, ROW SHARE
EXCLUSIVE             | ACCESS SHARE
ACCESS EXCLUSIVE      | Nothing
```

### Wait Events for Lock Type

| wait_event | Description | Common Cause |
|------------|-------------|--------------|
| `relation` | Table-level lock | DDL operations, TRUNCATE |
| `tuple` | Row-level lock | Concurrent UPDATEs on same row |
| `transactionid` | Transaction lock | Waiting for transaction to finish |
| `extend` | Relation extension | Growing table (multiple backends) |
| `page` | Page lock | Usually vacuum-related |
| `object` | Object lock | DDL on dependent objects |

---

## TimescaleDB-Specific Considerations

### Hypertable Chunk Locking

```sql
-- Monitor locks on individual chunks (useful for debugging compression issues)
SELECT 
    c.hypertable_name,
    c.chunk_name,
    c.range_start,
    c.range_end,
    c.is_compressed,
    pl.mode,
    pl.granted,
    sa.query
FROM timescaledb_information.chunks c
JOIN pg_class pc ON pc.relname = c.chunk_name
LEFT JOIN pg_locks pl ON pl.relation = pc.oid
LEFT JOIN pg_stat_activity sa ON sa.pid = pl.pid
WHERE pl.pid IS NOT NULL
    AND pl.pid != pg_backend_pid()
ORDER BY c.hypertable_name, c.range_start DESC;
```

### Compression Job Locks

Compression acquires `ACCESS EXCLUSIVE` locks on chunks being compressed:

```sql
-- Check if compression jobs are holding locks
SELECT 
    j.job_id,
    j.proc_name,
    j.scheduled,
    j.last_run_status,
    j.last_run_started_at,
    j.last_run_duration,
    pl.mode,
    pl.granted
FROM timescaledb_information.jobs j
LEFT JOIN pg_locks pl ON pl.pid = (
    SELECT pid FROM pg_stat_activity 
    WHERE application_name LIKE '%job%' || j.job_id || '%'
    LIMIT 1
)
WHERE j.proc_name LIKE '%compress%'
ORDER BY j.last_run_started_at DESC;
```

**Best practices:**
- Schedule compression during off-market hours
- Use short compression windows
- Monitor `last_run_duration` - long durations indicate potential lock issues

### Continuous Aggregate Refresh Locks

```sql
-- Monitor continuous aggregate refresh locks
SELECT 
    ca.view_name,
    j.last_run_started_at,
    j.last_run_duration,
    pl.mode,
    pl.granted,
    sa.state,
    sa.query
FROM timescaledb_information.continuous_aggregates ca
JOIN timescaledb_information.jobs j 
    ON j.proc_name = 'policy_refresh_continuous_aggregate'
LEFT JOIN pg_stat_activity sa 
    ON sa.application_name LIKE '%' || j.job_id || '%'
LEFT JOIN pg_locks pl ON pl.pid = sa.pid
WHERE sa.pid IS NOT NULL
ORDER BY j.last_run_started_at DESC;
```

---

## Grafana Dashboard Queries

### Panel 1: Current Lock Wait Count

```sql
SELECT 
    COUNT(*) as value,
    'waiting_queries' as metric
FROM pg_stat_activity 
WHERE wait_event_type = 'Lock'
    AND pid != pg_backend_pid();
```

### Panel 2: Maximum Lock Wait Time

```sql
SELECT 
    COALESCE(MAX(EXTRACT(EPOCH FROM (now() - query_start))), 0) as value,
    'max_wait_seconds' as metric
FROM pg_stat_activity 
WHERE wait_event_type = 'Lock'
    AND pid != pg_backend_pid();
```

### Panel 3: Lock Count by Table (Table)

```sql
SELECT 
    pl.relation::regclass::text AS table_name,
    COUNT(*) AS lock_count,
    COUNT(*) FILTER (WHERE NOT pl.granted) AS waiting
FROM pg_locks pl
WHERE pl.relation IS NOT NULL
    AND pl.pid != pg_backend_pid()
GROUP BY pl.relation
ORDER BY lock_count DESC
LIMIT 10;
```

### Panel 4: Deadlock Rate (Graph over time)

```sql
SELECT 
    deadlocks,
    stats_reset
FROM pg_stat_database 
WHERE datname = current_database();
```

Configure Grafana to calculate rate of change.

---

## Troubleshooting Scenarios

### Scenario 1: Sudden Latency Spike

**Symptoms:**
- Order execution latency jumps from 5ms to 500ms
- No obvious application changes

**Investigation:**
1. Run Query #1 (Current Blocking Queries)
2. Check if blocker is from your app or external (e.g., admin query)
3. Run Query #6 (Real-Time Lock Wait Detection)

**Common causes:**
- Long-running report query during market hours
- Forgotten BEGIN without COMMIT
- Background VACUUM FULL or maintenance

**Resolution:**
- Kill the blocking query if non-critical
- Add `statement_timeout` to prevent long queries
- Schedule maintenance during off-hours

---

### Scenario 2: Intermittent Deadlocks

**Symptoms:**
- Occasional transaction rollbacks with "deadlock detected" error
- Appears random, hard to reproduce

**Investigation:**
1. Enable deadlock logging:
   ```sql
   ALTER SYSTEM SET log_lock_waits = on;
   ALTER SYSTEM SET deadlock_timeout = '1s';
   SELECT pg_reload_conf();
   ```
2. Check PostgreSQL logs for deadlock details
3. Run Query #7 (Deadlock History) to confirm increasing trend

**Common causes:**
- Transactions acquiring locks in different orders
- Example:
  ```csharp
  // Transaction A
  UPDATE accounts WHERE id = 1;  -- Lock account 1
  UPDATE positions WHERE id = 2; -- Lock position 2
  
  // Transaction B (different order!)
  UPDATE positions WHERE id = 2; -- Lock position 2
  UPDATE accounts WHERE id = 1;  -- WAIT on account 1 → DEADLOCK
  ```

**Resolution:**
- Enforce consistent lock ordering in application code
- Use explicit `SELECT FOR UPDATE` to acquire all locks upfront:
  ```csharp
  // Better: acquire all locks first
  BEGIN;
  SELECT * FROM accounts WHERE id = 1 FOR UPDATE;
  SELECT * FROM positions WHERE id = 2 FOR UPDATE;
  // Now do your updates
  UPDATE accounts ...
  UPDATE positions ...
  COMMIT;
  ```

---

### Scenario 3: High Lock Count on Single Table

**Symptoms:**
- Query #4 shows 1000+ locks on `market_data` table
- Inserts slowing down

**Investigation:**
1. Run Query #4 (Locks by Table)
2. Check if locks are granted or waiting
3. Run Query #3 (Lock Table Snapshot) filtered to that table

**Common causes:**
- Many concurrent inserts without batching
- Long transaction holding lock while inserting
- TimescaleDB chunk contention

**Resolution:**
- Batch inserts (e.g., 1000 rows per transaction instead of 1)
- Reduce transaction duration
- Check if TimescaleDB chunk interval is too small
- Consider partitioning strategy

---

### Scenario 4: ACCESS EXCLUSIVE Lock During Market Hours

**Symptoms:**
- Everything blocks suddenly
- Query #1 shows `AccessExclusiveLock` mode

**Investigation:**
1. Run Query #1 to find the culprit
2. Check `query` field - likely DDL or maintenance

**Common causes:**
- Accidental `ALTER TABLE` during market hours
- Manual `VACUUM FULL`
- `TRUNCATE` command
- Index rebuild

**Resolution:**
- Kill immediately: `SELECT pg_terminate_backend(pid);`
- Add guardrails to prevent DDL during market hours
- Use `CREATE INDEX CONCURRENTLY` instead of regular CREATE INDEX

---

## Automation Scripts

### Daily Lock Health Report (Cron/Scheduled Task)

```bash
#!/bin/bash
# daily_lock_report.sh

PGHOST="localhost"
PGDATABASE="trading_db"
PGUSER="monitoring_user"

REPORT_FILE="/var/log/postgres/lock_report_$(date +%Y%m%d).txt"

{
    echo "=== PostgreSQL Lock Health Report ==="
    echo "Date: $(date)"
    echo ""
    
    echo "## Deadlock Count ##"
    psql -h $PGHOST -d $PGDATABASE -U $PGUSER -c "
        SELECT deadlocks, stats_reset 
        FROM pg_stat_database 
        WHERE datname = current_database();
    "
    
    echo ""
    echo "## Tables with Most Locks (24h average) ##"
    psql -h $PGHOST -d $PGDATABASE -U $PGUSER -c "
        SELECT 
            pl.relation::regclass AS table_name,
            COUNT(*) AS lock_count
        FROM pg_locks pl
        WHERE pl.relation IS NOT NULL
        GROUP BY pl.relation
        ORDER BY lock_count DESC
        LIMIT 10;
    "
    
    echo ""
    echo "## Long-Running Transactions ##"
    psql -h $PGHOST -d $PGDATABASE -U $PGUSER -c "
        SELECT 
            pid,
            usename,
            application_name,
            now() - xact_start AS duration,
            state
        FROM pg_stat_activity
        WHERE xact_start IS NOT NULL
            AND now() - xact_start > interval '10 seconds'
        ORDER BY duration DESC;
    "
} > $REPORT_FILE

# Send report to Lark/Slack/Email
# ... your notification logic here ...
```

### Automated Lock Killer (Use with Caution)

```csharp
// Auto-kill queries waiting too long
public class LockKillerService : BackgroundService
{
    private readonly NpgsqlDataSource _dataSource;
    private readonly ILogger<LockKillerService> _logger;
    private readonly TimeSpan _maxWaitTime = TimeSpan.FromSeconds(30);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                await CheckAndKillBlockersAsync();
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error in lock killer");
            }

            await Task.Delay(TimeSpan.FromSeconds(10), stoppingToken);
        }
    }

    private async Task CheckAndKillBlockersAsync()
    {
        await using var conn = await _dataSource.OpenConnectionAsync();

        // Find blockers causing > 30s wait
        var longBlockers = await conn.QueryAsync<int>(@"
            SELECT DISTINCT blocking.pid
            FROM pg_stat_activity blocked
            JOIN pg_stat_activity blocking 
                ON blocking.pid = ANY(pg_blocking_pids(blocked.pid))
            WHERE EXTRACT(EPOCH FROM (now() - blocked.query_start)) > @MaxWaitSeconds
                AND blocking.application_name != 'CriticalApp'  -- Protect critical apps
                AND blocked.pid != pg_backend_pid()
            ",
            new { MaxWaitSeconds = _maxWaitTime.TotalSeconds }
        );

        foreach (var pid in longBlockers)
        {
            _logger.LogWarning("Auto-killing blocker PID {Pid} (exceeded {MaxWait}s threshold)", 
                pid, _maxWaitTime.TotalSeconds);

            try
            {
                // Try cancel first
                await conn.ExecuteAsync("SELECT pg_cancel_backend(@Pid)", new { Pid = pid });
                await Task.Delay(TimeSpan.FromSeconds(2));

                // If still there, terminate
                var stillExists = await conn.QuerySingleAsync<bool>(
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid = @Pid)",
                    new { Pid = pid }
                );

                if (stillExists)
                {
                    await conn.ExecuteAsync("SELECT pg_terminate_backend(@Pid)", new { Pid = pid });
                    _logger.LogWarning("Terminated blocker PID {Pid}", pid);
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to kill blocker PID {Pid}", pid);
            }
        }
    }
}
```

---

## Configuration Recommendations

### PostgreSQL Settings for Lock Monitoring

Add to `postgresql.conf`:

```ini
# Enable lock wait logging
log_lock_waits = on                    # Log queries waiting for locks
deadlock_timeout = 1s                   # Time before checking for deadlock

# Statement timeout (failsafe)
statement_timeout = 30000               # 30 seconds max query time

# Logging
log_min_duration_statement = 100       # Log queries > 100ms
log_line_prefix = '%t [%p] %u@%d '     # Include timestamp, PID, user, database

# Lock table size (if you have many concurrent connections)
max_locks_per_transaction = 64          # Default, increase if needed

# Connection limits
max_connections = 100                   # Adjust based on your load

# Query stats
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.track = all
```

### Application-Level Settings

```csharp
// Connection string settings
var dataSourceBuilder = new NpgsqlDataSourceBuilder(connectionString);

// Set default timeout
dataSourceBuilder.ConnectionStringBuilder.CommandTimeout = 10; // 10 seconds

// Connection pooling
dataSourceBuilder.ConnectionStringBuilder.MaxPoolSize = 50;
dataSourceBuilder.ConnectionStringBuilder.MinPoolSize = 5;

// Application name for tracking
dataSourceBuilder.ConnectionStringBuilder.ApplicationName = "RTG";

var dataSource = dataSourceBuilder.Build();
```

---

## Additional Resources

### PostgreSQL Documentation
- Lock Monitoring: https://www.postgresql.org/docs/current/monitoring-locks.html
- Lock Management: https://www.postgresql.org/docs/current/explicit-locking.html
- pg_stat_activity: https://www.postgresql.org/docs/current/monitoring-stats.html#MONITORING-PG-STAT-ACTIVITY-VIEW

### TimescaleDB Documentation
- Hypertables: https://docs.timescale.com/use-timescale/latest/hypertables/
- Compression: https://docs.timescale.com/use-timescale/latest/compression/
- Continuous Aggregates: https://docs.timescale.com/use-timescale/latest/continuous-aggregates/

### Tools
- pgAdmin: https://www.pgadmin.org/
- pg_activity: https://github.com/dalibo/pg_activity (Real-time monitoring)
- pgBadger: https://github.com/darold/pgbadger (Log analyzer)

---

## Appendix: Quick Copy-Paste Commands

```sql
-- Quick health check
SELECT COUNT(*) as blocked_queries 
FROM pg_stat_activity 
WHERE wait_event_type = 'Lock';

-- Find blocker
SELECT blocking.pid, blocking.query 
FROM pg_stat_activity blocked
JOIN pg_stat_activity blocking 
    ON blocking.pid = ANY(pg_blocking_pids(blocked.pid))
LIMIT 1;

-- Kill a query
SELECT pg_cancel_backend(12345);      -- Graceful
SELECT pg_terminate_backend(12345);   -- Forceful

-- Check deadlocks
SELECT deadlocks FROM pg_stat_database WHERE datname = current_database();

-- Long transactions
SELECT pid, now() - xact_start as duration, query
FROM pg_stat_activity
WHERE xact_start IS NOT NULL
ORDER BY duration DESC;
```

---

**End of Cheatsheet**

*Keep this document updated as you discover new patterns and issues in your trading system. Add your own queries and thresholds based on production experience.*
