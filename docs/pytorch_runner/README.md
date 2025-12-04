# PyTorch Runner & `trn` CLI Plan

This document explains how the Tier‑10 learning stack will bridge the Rust
orchestration layer with PyTorch GPU/CPU execution and expose an operator‑grade
CLI (`trn`). The plan is broken down into layers so each component can be
implemented and tested incrementally.

## Goals

1. **Deterministic environment bootstrap** – reuse `scripts/setup_env.py`
   output to confirm CUDA/ROCm readiness before spawning training jobs.
2. **Device‑aware scheduling** – leverage `DeviceManager` (Rust) to select
   GPU/CPU resources and pass device topology to the runner.
3. **Schema‑driven data access** – rely on `DatasetIndex` / `ShardLoader`
   (Rust) to provide shard manifests; PyTorch DataLoader reads referenced
   files directly.
4. **CLI parity** – `trn` must support `run`, `status`, `list`, `resume`, and
   produce structured JSON events/logs consumable by other modules and the
   event bus.
5. **Online learning hooks** – expose a thin gRPC/HTTP (future work) or IPC
   interface so metacognition / runtime monitors can trigger `trn run`
   programmatically.

## Architecture Overview

```
┌──────────────────────────┐
│           CLI            │  `trn run --config ...`
└────────────┬─────────────┘
             │
    (Rust binary crate)
             │
┌────────────▼─────────────┐   ┌────────────────────────┐
│ Job Planner / Scheduler  │──▶│ DeviceManager          │
│ - validate config JSON   │   │ - autodetect GPUs/CPUs │
│ - allocate job_id        │   │ - produce Allocation   │
└────────────┬─────────────┘   └─────────┬──────────────┘
             │                           │
             │ spawn runner process      │
┌────────────▼─────────────┐             │
│ PyTorch Runner (Python)  │◀────────────┘
│ - loads config, dataset  │
│ - builds model/optimizer │
│ - executes training loop │
└────────────┬─────────────┘
             │ metrics / events (JSON line)
┌────────────▼─────────────┐
│ Logging + Event Bus      │
└──────────────────────────┘
```

### CLI responsibilities

- Parse configs (YAML/JSON) against schemas (future).
- Invoke `DeviceManager::allocate` to decide `CUDA_VISIBLE_DEVICES`.
- Emit job metadata:
  ```json
  {
    "event": "job_submitted",
    "job_id": "job-uuid",
    "config_path": "...",
    "devices": ["cuda:0","cuda:1"]
  }
  ```
- Spawn Python runner via `python -m learning.pytorch_runner --config ...`
  with environment variables describing datasets, logs, etc.

### PyTorch runner responsibilities (upcoming Python package)

- Validate dataset schema (via JSON).
- Build PyTorch `DataLoader` using shard manifest.
- Build model/optimizer/scheduler from config.
- Use `torch.distributed.launch` (DDP) when multiple GPUs requested.
- Persist checkpoints (`.pt` + `.meta.json`).
- Stream metrics/events back via stdout (JSON line) for the CLI/log collector.

### Event bus and logging

- CLI and runner both emit JSON lines that can be tailed by Fluentd/Vector.
- Optional IPC or Redis Streams connector will forward events to autonomy /
  metacognition modules.

## CLI Command Surface (v1)

- `trn run --config CONFIG --weights <path> --profile prod --devices cuda:0,cuda:1`
- `trn status <job_id>` – query job manifest/logs (local JSON DB initially).
- `trn list` – show recent jobs from manifest.
- `trn resume --checkpoint <meta.json>` – restart from a saved checkpoint.

## Data Flow

1. Operator (or autopilot) calls `trn run`.
2. CLI validates config and dataset/index presence.
3. `DeviceManager` selects devices, CLI sets env vars.
4. CLI spawns Python runner (DDP or single process).
5. Runner logs JSON lines to `learning/logs/YYYY/MM/DD/`.
6. Upon completion, CLI writes job manifest entry (`learning/logs/jobs/index.jsonl`).

## Next Steps

1. Implement the `trn` CLI skeleton (Rust crate) calling into `DeviceManager`
   and emitting stub job manifests.
2. Scaffold the Python runner package with config parsing and placeholder loop.
3. Integrate event/log forwarding hooks.
4. Expand CLI commands (`status`, `list`, `resume`).

## Sample configuration

`ZAPPY--M/learning/configs/sample_train.json` demonstrates a minimal training
config referencing the sample dataset under `learning/datasets/sample`. Use it
for smoke tests:

```bash
cargo run -p trn -- run \
  --config ZAPPY--M/learning/configs/sample_train.json \
  --log-dir build/logs \
  --event-log build/events/training.jsonl
```

