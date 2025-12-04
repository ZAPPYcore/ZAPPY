# Learning Directory Layout

This document defines the canonical layout for datasets, models, checkpoints,
and logs managed by the Tier‑10 AGI learning stack. All paths are relative to
`ZAPPY--M/learning/`.

```
learning/
├── datasets/
│   └── <dataset_name>/
│       ├── schema.json          # conforms to docs/agi_json_schema/dataset.schema.json
│       ├── index.json           # shard map + offsets
│       └── shards/
│           ├── shard-00000.data # binary payload (npy, parquet, custom)
│           └── shard-00001.data
├── models/
│   └── <model_name>/
│       ├── model_schema.json    # conforms to docs/agi_json_schema/model.schema.json
│       └── checkpoints/
│           ├── checkpoint-000123.pt
│           ├── checkpoint-000123.meta.json
│           └── ...
├── logs/
│   └── <YYYY>/<MM>/<DD>/trainer.log.jsonl.gz
├── device_manager.rs            # Device discovery + allocation (Rust)
├── dataloader.rs                # JSON-index driven shard loader (Rust)
├── pytorch_runner/              # PyTorch execution entrypoints (Python)
└── telemetry.rs                 # Logging + event bus helpers
```

## Conventions

1. **Binary + JSON pairing** – heavy tensors live in binary files (safetensors,
   `.pt`, `.npy`, etc.) while metadata points to them via JSON.
2. **Shard size** – keep between 256 MB and 2 GB for optimal streaming.
3. **Logging** – all trainers emit JSON lines (`.jsonl`) with an hourly/daily
   rotation policy; files are compressed when closed.
4. **Checkpoints** – along with the binary state, maintain metadata describing
   training step, devices, and hashes for integrity verification.

## Rust helper modules

- `device_manager.rs` exposes `DeviceManager`, `DeviceInfo`, and
  `DevicePreference` for allocating workloads across CPU/GPU resources.
- `dataloader.rs` defines `DatasetIndex` and `ShardLoader`, capable of reading
  JSON indices and streaming shard binaries with a small prefetch buffer.
- `telemetry.rs` exposes `LearningTelemetry` builders to connect the runtime to
  `shared-logging` and `shared-event-bus`, enabling structured observability.

