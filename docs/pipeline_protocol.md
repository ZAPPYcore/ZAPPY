# Learning Pipeline Protocol

The ExperienceHub provides a shared channel for AGI modules (actions, autonomy, planning, reasoning, world, simulation, self-upgrade, etc.) to publish learning-ready events. Each event is encoded as a `PipelineEnvelope` serialized to JSON:

```jsonc
{
  "id": "uuid-v4",
  "module": "planning",
  "signal": "plan.generated",
  "payload": { "objective": "...", "phases": 3 },
  "timestamp": "RFC3339"
}
```

## Guarantees
1. **Ordering** – Records are stored in arrival order. Consumers should use `since(timestamp)` to resume from checkpoints.
2. **Retention** – The hub keeps the latest N envelopes (configurable). Downstream learners must retrieve and archive for longer-term storage.
3. **Security** – Only modules complying with `POLICIES.md` may publish/consume; sensitive payloads must be scrubbed or encrypted prior to publication.

## Usage
- Producers call `ExperienceHub::publish(module, signal, payload)` using structured JSON.
- Learners call `snapshot(limit)` to fetch the latest window or `since(ts)` to resume incremental training.
- Downstream ML jobs convert envelopes into training batches, enabling experience sharing across autonomous agents.

