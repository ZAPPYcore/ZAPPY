# Cross-Module Observability Rollout (Tier-10)

## Goals

1. **Uniform telemetry surface** – every top-level module (actions, autonomy,
   creativity, knowledge, learning, memory, metacognition, etc.) emits logs and
   events with shared semantics so downstream monitors can correlate pipelines.
2. **Pluggable sinks** – JSON log files for local durability + event bus
   publishers (memory, file, Redis/Kafka in later phases) should be swappable
   without changing business logic.
3. **Online learning feedback** – telemetry events from production modules feed
   into the learning/metacognition stack to trigger self-improvement loops.

## Phased Plan

### Phase 1 – Foundation (complete)

- `shared_logging`: JSON line logger with severity + metadata envelopes.
- `shared_event_bus`: `EventRecord`, async publisher/subscriber, memory bus,
  and `FileEventPublisher`.
- `LearningTelemetry`: builder that wires the learning runtime to the shared
  logging/event bus stack; classical/deep pipelines emit `learning.*` events.
- `trn` CLI: emits `training.job_*` events and logs.

### Phase 2 – Cross-module instrumentation (in flight)

1. **Actions** – (done) `ActionCommander` consumes `ActionTelemetry`:
   - logs security verdicts, plan synthesis, agent lifecycle;
   - emits `actions.*` events with domain/intent/priority metadata for metacog +
     audit subscribers.
2. **Autonomy** – instrument decision director + reviewers:
   - log signal loads, reviewer verdicts, final decisions;
   - emit `autonomy.decision.*` events for governance monitors.
3. **Creativity** – (done) `CreativityRuntime` + kernel emit telemetry:
   - log ideation, weaving, review completion, and cycle summaries;
   - emit `creativity.*` events with brief titles and idea counts for dashboards.
4. **Knowledge** – (done) knowledge runtime instruments ingest/search/edit/web search:
   - logs artifact sources, record sizes, search queries/results;
   - emits `knowledge.record.*` events for downstream monitors.
5. **Memory** – (done) `memory_data::MemoryRuntime` now emits telemetry:
   - logs capture/search/flush operations with importance + tag metadata;
   - emits `memory.flush.completed` events for durable archival tracking.
6. **Metacognition** – (done) telemetry wraps reflection planning/execution so
   learning can subscribe to `metacognition.*` events.
7. **Planning** – (done) long/short-term planners emit `planning.*` telemetry:
   - logs objective intake, plan synthesis, schedule generation;
   - raises events when re-plan triggers fire for governance subscribers.
8. **NLP** – (done) comprehension/answer/training pipelines emit `nlp.*`
   telemetry:
   - logs console/CLI questions, confidence, and training launches;
   - exposes events for metacognition-driven self-learning triggers.
9. **Reasoning** – (done) inference + multi-domain review emit `reasoning.*`
   telemetry:
   - logs directive intake, hypothesis reviews, verdict selection;
   - publishes events for autonomy/metacognition to trigger follow-up action.
10. **World** – (done) world modeling runtime emits `world.*` telemetry:
    - logs info seeker batches, assimilation jobs, anomaly alerts;
    - surfaces training + governance events for downstream operators.
11. **Simulation Engine** – (done) simulation env generator + predictors emit
    `simulation.*` telemetry:
    - logs scenario batches, comparison MAE, reviewer outcomes;
    - streams advanced report/training events for governance tooling.
12. **Self Upgrade** – (done) diagnostics/planner/reporting emit
    `self_upgrade.*` telemetry:
    - logs checker findings, plan creation, execution lifecycle;
    - records upgrade reports for downstream governance archives.

### Phase 3 – Enterprise rollout

- Provide Redis/Kafka publishers implementing `EventPublisher`.
- Ship Fluentd/Vector configs that tail JSON logs & push to Elastic/OpenSearch.
- Expand docs with dashboards (Prometheus/Grafana) and runbooks for operators.
- Tie metacognition online-learning triggers to event bus topics (e.g.,
  `training.job_failed` -> auto-run diagnostic workflows).

## Immediate next steps

- Implement module-specific telemetry wrappers (starting with actions).
- Add configuration knobs (ENV/CLI) to enable/disable event sinks per module.
- Document log/event schemas for each module and add integration tests that
  validate emitted payloads against JSON schema.

