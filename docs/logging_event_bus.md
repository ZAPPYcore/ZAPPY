# Logging & Event Bus Architecture

## Logging

- Implemented in `shared-logging` crate.
- Provides `JsonLogger` + `LogRecord` structures with ISO timestamps, severity
  levels, module names, and arbitrary metadata.
- Each log call writes a single JSON line (UTF-8) to the configured file with
  fsync-level flushing to guarantee durability.
- Directories are auto-created, making it safe to point at
  `learning/logs/YYYY/MM/DD/*.log.jsonl`.
- Meant to be embedded in all crates (actions/autonomy/learning/...) to unify
  logging format. Log shippers (Fluentd/Vector) can tail these files and forward
  to Elasticsearch / OpenSearch.

## Event Bus

- Implemented in `shared-event-bus` crate.
- Provides `EventRecord`, `EventPublisher`, and `EventSubscriber` traits.
- Includes `MemoryEventBus` for local development: uses `tokio::broadcast` +
  in-memory backlog for simple fan-out.
- Includes `FileEventPublisher` which appends JSON line events to disk,
  guaranteeing durability and enabling downstream ingestion pipelines.
- Future production adapters (Redis Streams, Kafka, NATS) will implement the
  same traits, allowing modules to remain agnostic to transport.
- Events are JSON-friendly and contain `id`, `source`, `event_type`, timestamp,
  and `payload`.

## Integration Plan

1. **Short term** – integrate `JsonLogger` into `trn`, learning runtimes, and
   other crates to produce structured logs.
2. **Mid term** – bake `MemoryEventBus` into integration tests and provide a
   Redis-based publisher for onsite deployments. `FileEventPublisher` now
   provides durable local logs that can be tailed or shipped.
3. **Long term** – connect metacognition/autonomy modules to the bus for
   real-time adaptation (online learning triggers, alerts, etc.).

`ZAPPY--M/learning/telemetry.rs` provides a concrete example: the learning
runtime uses `LearningTelemetry` to log pipeline milestones and emit
`learning.*` events whenever classical/deep/subsidiary stages run. Builders
accept a `JsonLogger` target and an `EventPublisher` (e.g., `FileEventPublisher`
or `MemoryEventBus`), making it easy for other crates to reuse the same pattern.

