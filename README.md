 # ZAPPY System Overview (English)

© 2025 Yujin Choi. All Rights Reserved

ZAPPY is an orchestration platform that couples a Rust-based multi-runtime control plane with a Python-based language-model server. The top-level orchestrator (`entire_system_orchestrator.rs`) continually invokes planning, autonomy, action, learning, metacognition, world-model, simulation, self-upgrade, and natural-language runtimes while enforcing policies. This README consolidates the repository structure, runtime loops, module responsibilities, and operational guidance.

NOTE: The main codebase is NOT present. Contact the E-mail on my profile for cooperations.
---

## 1. Repository Map

| Path | Description |
| --- | --- |
| `Cargo.toml` | Declares the workspace, unifying every `ZAPPY--M/*` crate into a single build graph. |
| `ZAPPY--M/entire_system_orchestrator.rs` | CLI entrypoint that boots every runtime and maintains the command loop. |
| `ZAPPY--M/actions` … `ZAPPY--M/world` | Core Rust crates. Each directory has its own `Cargo.toml`/`src` and exposes a bounded module. |
| `ZAPPY--M/langmodel/` | FastAPI-based LLM server plus local Mistral checkpoints; handles natural-language → IR conversion. |
| `ZAPPY--M/trn_cli/` | Training CLI (`trn`) that connects Rust learning helpers with the PyTorch runner. |
| `docs/` | Pipeline protocol, observability rollouts, learning/LLM JSON schemas (`docs/agi_json_schema`). |
| `logs/` | JSONL logs per runtime; orchestrator logs include planning/reasoning/experience/upgrades. |
| `scripts/setup_env.py` | GPU/PyTorch environment checker for new nodes. |
| `langmodel/*.safetensors` | Local LLM weights. `server.py` attempts 4-bit loading before falling back. |
| `Dockerfile`, `docker-compose.gpu.yml` | CUDA + Rust + Python dev containers with GPU passthrough. |
| `POLICIES.md` | Operational/security policy ledger loaded at orchestrator boot. |
| `temp_prompt.py`, `temp/` | Experimental scripts; no production impact. |

> **Build artifacts:** Everything under `target/` (`*.rlib/*.exe/*.pdb`) is a Rust build cache. Ignore unless diagnosing builds.

---

## 2. Runtime Architecture & Loops

### 2.1 Orchestrator Command Loop
1. `tokio::main` creates `logs/orchestrator/*`, loads `PolicyLedger`, and prepares every runtime instance.
2. Reads commands from stdin (`plan`, `simulate`, `knowledge`, `autonomy`, `execute`, `natural`, `reflect`, `experience`, `policy`, `status`, `help`, `exit`, etc.).
3. Each command calls the relevant runtime and writes the outcome to `ExperienceHub` (`learning/pipeline.rs`) as JSON.
4. `natural` routes through `NaturalLanguageClient` to the FastAPI LangModel server → IR(JSON) → pipelines for planning/knowledge/actions.

### 2.2 Autonomy Loop
1. `AutonomyRuntime::bootstrap()` registers planners, executors, sensors, and custom modules in the `ModuleRegistry`.
2. `autonomy cycle` triggers the `AutonomyLinker`, which coordinates `MasterController`/`Director` to issue directives.
3. Each cycle ingests an `AutonomySignal` (scope/metrics/tags) and returns `ControlDirective`s, confidence, and metrics, recording to `ExperienceHub` and `logs/autonomy/runtime.log.jsonl`.

### 2.3 Planning → Reasoning → Action
1. **PlanningRuntime** combines `LongTermPlanner`, `ShortTermPlanner`, and `AdvancedPortfolioPlanner` to build strategic plans and tactical schedules.
2. **ReasoningRuntime** aggregates plan output, natural-language IR, and world signals to produce `Verdict`s (hypothesis, directives, confidence).
3. **ActionCommander** accepts `ActionRequest`s (domain, intent, priority, safety, metadata, attachments), runs policy/safety checks, selects agents, and executes commands (programming, internet, simulation, infrastructure, etc.).

### 2.4 Experience → Knowledge → Learning
1. Every module publishes JSON events via `ExperienceHub::publish`. `ExperienceRecorder` persists them to `logs/orchestrator/experience.log.jsonl`.
2. `knowledge sync` replays recent events through `ExperienceReplayService` to grow `KnowledgeRuntime`.
3. The `learning` crate converts experiences into datasets, orchestrates PyTorch training via `DeviceManager` + `pytorch_runner`, and stores checkpoints/logs under `learning/models` and `learning/logs`.

### 2.5 World Modeling & Simulation
1. `world refresh` loads `ZAPPY--M/world/data/feeds.toml`, syncs external feeds, and rebuilds state via `AssimilationEngine`.
2. `simulate` calls `SimulationEngine::run_advanced(SimulationMethod::HighFidelity, n)` to generate scenarios and insights, which feed back into reasoning and self-upgrade loops.

### 2.6 Metacognition & Self-Upgrade
1. `reflect` invokes `MetacognitionRuntime::reflect(SelfObservation, ReflectionMethod)` across rapid/structured/audit modes, producing resilience-scored summaries.
2. `upgrade` runs `SelfUpgradeRuntime` to diagnose issues, craft improvement plans, execute actions, and log reports under `logs/orchestrator/upgrades`.
3. Outputs feed into `NaturalLanguageClient::generate_report` and policy audit workflows.

### 2.7 Observability & Policy
- All runtimes instrument `shared_logging::JsonLogger` and `shared_event_bus::MemoryEventBus` / `FileEventPublisher`.
- `POLICIES.md` loads at boot; violations emit `POLICY-*` events into `ExperienceHub` and can trigger self-upgrade.
- See `docs/logging_event_bus.md`, `docs/observability_rollout.md`, `docs/pipeline_protocol.md` for telemetry rollout plans.

---

## 3. Module Overview (Highlights)

### Actions (`ZAPPY--M/actions`)
- Executes every `ActionRequest` via domain-specific agents (programming, internet, offline, simulation).
- Key files: `actions.rs`, `actioncommander.rs`, `commandgeneration.rs`, `security_link.rs`, `telemetry.rs`.
- Flow: request → `SecurityLink` policy check → agent dispatch → execution → telemetry/logs.
- Example: `actions/main.rs` `orchestrate_sample()` demonstrates an end-to-end run.

### Autonomy (`ZAPPY--M/autonomy`)
- Wraps planners/executors/sensors with a brokered decision loop.
- Components: `decision/`, `linker.rs`, `master/`, `module/`, `telemetry.rs`.
- CLI: `autonomy cycle|directive|metrics`.

### Creativity (`ZAPPY--M/creativity`)
- Generates/evaluates ideas, narratives, and creative portfolios.

### Knowledge (`ZAPPY--M/knowledge`)
- Receives, stores, searches, edits, and secures artifacts; provides `knowledge sync|search|ingest`.

### Learning (`ZAPPY--M/learning`)
- Manages datasets/models/logs, Rust-side data loaders, PyTorch runner integration.
- Key files: `dataloader.rs`, `device_manager.rs`, `pipeline.rs` (ExperienceHub), `pytorch_runner/`, `telemetry.rs`.
- CLI integration: `trn_cli`.

### Memory Data, Metacognition, NLP, Planning, Reasoning, Self-Upgrade, Simulation Engine, World, Shared Libraries, LangModel, Web UI, TRN CLI
- Mirror the Korean README: each section explains role, key files, and integration points (see original README for structure). All modules expose telemetry and integrate with `ExperienceHub`.

*(For brevity, refer back to the Korean README’s module section; this English file summarizes the same responsibilities.)*

---

## 4. Data · Logs · Policies
- **Experience data**: `logs/orchestrator/experience.log.jsonl` + in-memory `ExperienceHub`; `ExperienceArchive` provides `tail`/`since`.
- **Knowledge data**: `ZAPPY--M/knowledge/saver.rs` keeps storage; external ingests route through `security`.
- **Datasets**: `learning/datasets/<dataset>/schema.json + shards/`; JSON schema in `docs/agi_json_schema`.
- **Model checkpoints**: `learning/models/<model>/checkpoints/checkpoint-*.pt` plus `.meta.json`.
- **Observability**: Every module’s `telemetry.rs` uses `shared_logging` + `shared_event_bus`. See documentation for schemas.
- **Policies**: `POLICIES.md` defines access/data/incident/change/audit requirements; violations raise events and can trigger `self_upgrade`.

---

## 5. Running & Usage

### 5.1 Prerequisites
- Rust 1.75+, Cargo.
- Python 3.10+ with `pip`/`virtualenv`.
- NVIDIA GPU + CUDA 12.1-compatible PyTorch (or CPU mode).
- ≥30 GB disk (LLM + checkpoints).

### 5.2 Environment Prep
1. Optional Docker: `docker compose -f docker-compose.gpu.yml up -d` → enter container.
2. Local diag: `python scripts/setup_env.py --output logs/env/report.json`.
3. Rust deps: `cargo fetch` / `cargo build --workspace`.
4. Python deps: `pip install fastapi uvicorn transformers accelerate bitsandbytes`.

### 5.3 Orchestrator
```bash
cargo run -p zappy-orchestrator
```
Command list mirrors the Korean README (plan, schedule, world, simulate, upgrade, reflect, execute, autonomy, knowledge, natural, experience, policy, status, help, exit).

### 5.4 Module Samples
- Actions: `cargo run -p zappy-actions --example orchestrate_sample`
- Autonomy demo: `cargo test -p zappy-autonomy demo_run`
- Learning CLI: `cargo run -p trn -- run --config ZAPPY--M/learning/configs/sample_train.json`
- Intent training, PyTorch runner, knowledge ingest, memory tests, etc., follow the Korean README instructions.

### 5.5 LangModel Server
```bash
cd ZAPPY--M/langmodel
pip install fastapi uvicorn transformers accelerate bitsandbytes
uvicorn langmodel.server:app --host 0.0.0.0 --port 9000
```
- Default endpoint: `http://127.0.0.1:9000/generate`.
- Use `LANGMODEL_CUDA_MEMORY` to cap GPU memory; falls back to CPU bf16 if 4-bit fails.

### 5.6 Docker Flow
1. `docker compose -f docker-compose.gpu.yml up --build -d`
2. `docker exec -it zappy-agi-dev bash`
3. Inside container: `cargo run -p zappy-orchestrator`
4. Host needs NVIDIA Container Toolkit for GPU passthrough.

### 5.7 Tests & Validation
- Full: `cargo test --workspace`
- Module-level: `cargo test -p <crate>`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Python runner: `pytest ZAPPY--M/learning/pytorch_runner/tests`

---

## 6. Troubleshooting & Ops Tips
- **LLM not returning JSON**: `NaturalLanguageClient` parses the last `{...}` block; check LangModel logs (`[langmodel] raw output`) and enforce prompt rules.
- **Action safety**: `--safety=red` may be blocked by policy; keep `POLICIES.md` updated and tune `security_link.rs` when needed.
- **Empty experience log**: ensure `ExperienceRecorder` path permissions are valid.
- **GPU missing**: if `scripts/setup_env.py` reports `gpu_not_detected`, inspect Docker/NVIDIA setup.
- **LangModel connectivity**: verify `LANGMODEL_ENDPOINT` and test with `curl`.
- **Dataset schemas**: align new datasets/models with `docs/agi_json_schema/*.schema.json`.

---

## 7. References
- `docs/pipeline_protocol.md` – pipeline phases for planning/autonomy/actions.
- `docs/observability_rollout.md` – telemetry rollout plan.
- `docs/logging_event_bus.md` – shared logging/event bus details.
- `docs/pytorch_runner/README.md` – training CLI architecture.
- Module-specific READMEs and source comments for deeper dives.

This English README mirrors the Korean version’s structure so contributors can onboard by reading (1) this overview, (2) the detailed docs under `docs/`, and (3) module source code.
