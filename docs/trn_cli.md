# `trn` CLI Specification

The `trn` (train) CLI is the operator-facing entrypoint for launching,
monitoring, and resuming Tier‑10 AGI learning jobs. This document specifies the
initial command set, configuration inputs, and output artifacts.

## Commands

### `trn run`

```
trn run --config configs/train.json --weights models/agi_core/checkpoints/latest.meta.json \
        --profile prod --devices cuda:0,cuda:1 --log-dir learning/logs \
        --event-log learning/logs/events.jsonl
```

Required flags:

- `--config <path>` – training configuration (JSON/YAML). Includes dataset name,
  model name, batch sizes, optimizer settings.
- `--weights <path>` – optional checkpoint metadata to resume from.

Optional flags:

- `--profile <profile>` – e.g., `dev`, `prod`, `fast_finetune`.
- `--devices <list>` – comma-separated device IDs. If omitted, CLI uses
  `DeviceManager::allocate(DevicePreference::GpuFirst, requested_gpus)`.
- `--log-dir <path>` – overrides default `learning/logs/`.
- `--event-log <path>` – optional JSON lines file receiving emitted events
  (`training.job_submitted`, `training.job_completed`, ...).

Outputs:

1. Prints job metadata JSON to stdout (for event bus ingestion).
2. Writes job manifest entry under `learning/logs/jobs/index.jsonl`.
3. Spawns the PyTorch runner and streams structured logs.

Sample invocation (using the bundled config):

```bash
cargo run -p trn -- run \
  --config ZAPPY--M/learning/configs/sample_train.json \
  --log-dir build/logs \
  --event-log build/events/training.jsonl
```

### `trn status <job_id>`

Reads the manifest and prints current/last known status, start/end timestamps,
and log locations. Future versions will integrate with the event bus for live
status.

### `trn list`

Shows the last *N* jobs (default 10) with id, profile, config path, status,
devices. Useful for dashboards and quick audits.

### `trn resume --checkpoint <meta.json>`

Validates the checkpoint metadata, generates a new job id, and internally calls
`trn run` with the checkpoint injected as `--weights`.

## Manifest Format

`learning/logs/jobs/index.jsonl` (append-only JSON lines):

```json
{
  "job_id": "job-20251120-abcdef",
  "submitted_at": "2025-11-20T14:32:00Z",
  "config": "configs/train.json",
  "weights": "models/.../checkpoint-000123.meta.json",
  "profile": "prod",
  "devices": ["cuda:0", "cuda:1"],
  "status": "running",
  "pid": 12345,
  "log_path": "learning/logs/2025/11/20/job-20251120-abcdef.log.jsonl"
}
```

Log rotation is handled per job; each log file is JSON lines with entries such
as `{"event":"step","step":100,"loss":0.123,"timestamp":...}`.

## Failure Handling

- If device allocation fails, CLI emits `job_rejected` event with a reason.
- If runner exits with non-zero status, CLI marks job as `failed` and retains
  stdout/stderr for debugging.
- Interrupted CLI sessions rely on manifest + log file to resume/resync.

## Integration Hooks

- Metacognition/runtime modules can shell out to `trn run` or (future) send a
  gRPC/HTTP request to a resident `trn` daemon.
- Event bus consumers can tail manifest/log files or subscribe to redis/kafka
  (future release) for immediate notifications.

