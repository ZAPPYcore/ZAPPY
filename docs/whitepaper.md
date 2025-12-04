# ZAPPY AGI Orchestration Platform — Whitepaper

## Abstract

ZAPPY is an AGI orchestration platform that fuses a Rust-based multi-runtime control plane with a Python-based language model server and an end-to-end data/learning substrate. The system’s `entire_system_orchestrator` process continuously coordinates planning, autonomy, reasoning, world modeling, simulation, execution, metacognition, self-upgrade, knowledge management, and learning runtimes while enforcing security policies and capturing every decision as replayable experience. This whitepaper describes the project’s objectives, architecture, operational guarantees, and roadmap so contributors and operators can reason about the platform as a cohesive product rather than a collection of crates.

---

## 1. Vision and Design Tenets

1. **Holistic autonomy loop** – close the feedback cycle between planning → reasoning → action → world/experience → learning → self-improvement without leaving the orchestrator context.
2. **Policy-first operations** – load `POLICIES.md` at boot, propagate guardrails into the action/security layer, and trigger self-upgrade workflows on violations.
3. **Telemetry as a product** – require every module to emit structured JSON logs and events via `shared_logging` + `shared_event_bus`, enabling downstream observability, audits, and online learning.
4. **Replayable intelligence** – treat all module outputs as `ExperienceHub` envelopes so new learners, diagnostics, and future agents can replay or continue from any point in time.
5. **Local-first LLM stack** – bundle FastAPI + Transformers serving (`ZAPPY--M/langmodel`) with on-disk Mistral checkpoints to remove cloud dependencies while retaining JSON IR guarantees.

---

## 2. System Topology

### 2.1 Control Plane

- **Entire System Orchestrator (`ZAPPY--M/entire_system_orchestrator.rs`)** instantiates all runtimes, wires telemetry buses, and exposes a REPL (`plan`, `simulate`, `execute`, `autonomy`, `knowledge`, `experience`, `policy`, etc.). Each command fan-outs into the relevant runtime and records a `PipelineEnvelope` via the shared `ExperienceHub`.  
- **Natural Language Client** bridges CLI `natural` commands to the local LLM server, enforcing JSON-only IR responses and caching the latest intent for downstream modules.  
- **Policy Ledger** loads `POLICIES.md`, attaches the hash to every action/upgrade, and validates policy queries via the `policy` command.

### 2.2 Execution & Cognitive Plane

| Module (crate) | Core responsibilities |
| --- | --- |
| `actions` | Defines `ActionDomain`, `ActionIntent`, safety classes, constraints, agents, and telemetry to route and execute operational work. |
| `autonomy` | Runs brokered decision cycles (`AutonomyRuntime`, `ModuleRegistry`, `MasterController`) and exposes `cycle/directive/metrics` commands. |
| `planning` | Hosts `LongTermPlanner`, `ShortTermPlanner`, `AdvancedPortfolioPlanner`, scoring/reviewer engines, and tactical schedule builders. |
| `reasoning` | Aggregates multi-domain signals, produces `Verdict`s, and feeds directives back into plan/execution loops. |
| `world` | Refreshes world feeds (`feeds.toml`), maintains feature stores, and synchronizes anomalies. |
| `simulationengine` | Provides scenario generation, predictors, advanced reports, and telemetered comparisons. |
| `knowledge` | Ingests, stores, secures, and searches artifacts (receiver/saver/seeker/websearcher/editor/security). |
| `learning` | Owns `ExperienceHub`, datasets, models, PyTorch runner, device manager, and CLI integration via `trn_cli`. |
| `memory_data` | Manages short/long-term tiers with importance-based retention and telemetry. |
| `metacognition` | Implements observation, reflection methods (`rapid/structured/audit`), and reporting pipelines. |
| `self_upgrade` | Diagnoses, plans, executes, and reports system upgrades, persisting outputs under `logs/orchestrator/upgrades`. |
| `creativity`, `nlp`, `simulationengine`, `world`, `shared_*` | Provide specialized cognition, natural language tooling, scenario engines, and shared infrastructure. |

All crates are first-class workspace members in `Cargo.toml`, ensuring a single `cargo build --workspace` produces the entire runtime graph.

### 2.3 Data & Learning Plane

- **Experience Fabric** – `ExperienceHub`, `ExperienceRecorder`, `ExperienceArchive`, and `ExperienceReplayService` (all in `ZAPPY--M/learning`) store every module signal as a JSONL envelope with UUID, module, signal, payload, and timestamp. Producers call `publish`, learners call `snapshot`/`since`, and replay merges archive + in-memory buffers for deterministic learning.
- **Learning Pipelines** – `learning/modules.rs`, `deepLearning/`, `subsidarylearning/`, and `pytorch_runner/` connect Rust orchestration with Python training jobs, while `trn_cli` provides operator tooling.
- **Knowledge & Memory** – Knowledge runtime ingests both external web data and internal experiences; Memory runtime ensures high-signal experiences persist with importance annotations.

### 2.4 Natural Language & Interface Plane

- **LangModel Server (`ZAPPY--M/langmodel/server.py`)** – FastAPI app that loads bundled Mistral weights (prefers 4-bit via `BitsAndBytesConfig`, falls back to CPU bf16) and enforces JSON outputs using bracket extraction, exception filters, and health-probe shortcuts (`system check`).  
- **CLI & Automation** – Operators interact through the orchestrator REPL or scripts (`temp_prompt.py`) that hit `POST /generate` for IR generation.

---

## 3. Core Flows

### 3.1 Planning → Reasoning → Action

1. Operator issues `plan <objective>` – `PlanningRuntime` synthesizes strategic plans, stores them in `last_plan`, and emits `planning.plan.generated` envelopes.  
2. `schedule` builds tactical task lists, bridging strategic intent to actionable sequences.  
3. `ReasoningRuntime` consumes plans, world signals, and natural language IRs to create `Verdict`s with confidence/priority.  
4. `ActionCommander` validates constraints via `SecurityLink`, selects specialized agents (programming, internet, offline, self-train, etc.), enforces safety classes, and logs `actions.*` telemetry.  
5. Every step records to the ExperienceHub, enabling downstream learning and audit.

### 3.2 Autonomy Cycle

`AutonomyRuntime::bootstrap()` registers planner/executor/sensor modules with a `ModuleRegistry`. When `autonomy cycle` is invoked, the runtime pulls `AutonomySignal`s, routes them through the `MasterController`, produces `ControlDirective`s, and logs to `logs/autonomy/runtime.log.jsonl`. Metrics and directives can be queried interactively, ensuring self-driving coordination without manual plan invocations.

### 3.3 Experience → Knowledge → Learning

1. Modules call `ExperienceHub::publish` with structured payloads.  
2. `ExperienceRecorder` appends JSONL lines (fsync) to `logs/orchestrator/experience.log.jsonl`.  
3. `knowledge sync` reads recent experiences using `ExperienceReplayService::since`, generating auditable knowledge artifacts and searchable snippets.  
4. `learning` converts experiences + knowledge into datasets (validated by `docs/agi_json_schema/*`), runs PyTorch jobs via `device_manager.rs` + `pytorch_runner`, and stores checkpoints/logs in `learning/models` and `learning/logs`.  
5. Training telemetry feeds back into metacognition and self-upgrade loops to trigger improvements or diagnostics.

### 3.4 Metacognition & Self-Upgrade

- `reflect <text> --method=...` builds `SelfObservation`s and invokes asynchronous reflection methods (`rapid`, `structured`, `audit`) that return resilience metrics and summaries, later consumed by reporting tasks.  
- `upgrade [target]` constructs `UpgradeDirective`s (~85 confidence default) and executes a self-upgrade plan whose results, status, and action list are persisted to upgrade logs.  
- Both modules share telemetry with the ExperienceHub, ensuring reflections and upgrades influence subsequent planning and policy.

---

## 4. Observability & Audit Guarantees

1. **Shared Logging (`shared_logging`)** – `JsonLogger` writes ISO8601-stamped JSON lines with severity, module, message, and metadata. Every crate adopts the logger via its `telemetry.rs`, guaranteeing consistent structure for ingestion into Fluentd/Vector or custom dashboards.  
2. **Event Bus (`shared_event_bus`)** – `MemoryEventBus` (tokio broadcast) supports local development, while `FileEventPublisher` ensures durable JSONL traces. Future work includes Redis/Kafka adapters per `docs/observability_rollout.md`.  
3. **Experience Replay** – Operators can call `experience [n]` to inspect the latest events, or learners can resume from timestamps. Combined with module-specific log files (`logs/orchestrator/*.log.jsonl`), the system offers end-to-end traceability.  
4. **Documentation** – `docs/logging_event_bus.md` and `docs/observability_rollout.md` outline phased instrumentation and integration plans, reinforcing that telemetry is not optional but a tracked deliverable.

---

## 5. Security, Safety, and Governance

- **Policy Ledger** – Bootstraps from `POLICIES.md`, covering access control, data governance, incident response, change management, and observability mandates. Violations trigger self-upgrade workflows and `POLICY-*` events within five minutes.  
- **Action Safety** – `ActionSafetyClass` (green → red) and `ActionConstraints` encode deadlines, compute credits, risk tolerances, and jurisdictions. `SecurityLink` translates policy grades into safety classes to gate execution.  
- **Knowledge Security** – `knowledge/security` module scores ingest sources, while `knowledge/websearcher.rs` and `knowledge/editor.rs` enforce sanitization before persistence.  
- **LLM Safeguards** – `langmodel/server.py` censors banned terms via `except.txt`, enforces JSON output for orchestration prompts, and returns `"처리 불가능"` when policy-violating text appears.  
- **Audit Trails** – Experience, logs, and policy hashes combine to form immutable audit chains, and `ExperienceArchive` ensures historical data cannot be silently dropped.

---

## 6. Deployment & Operations

1. **Baseline Requirements** – Rust 1.75+, Python 3.10+, CUDA-capable GPU with ~30 GB storage for LLM checkpoints, or CPU fallback when GPUs are absent.  
2. **Environment Verification** – `scripts/setup_env.py --output logs/env/report.json` inspects `nvidia-smi`, `rocm-smi`, and PyTorch install state, recommending `ok`, `install_torch_cuda_build`, or `gpu_not_detected`.  
3. **Containerized Workflow** – `Dockerfile` builds atop `nvidia/cuda:12.1.1-cudnn8-runtime-ubuntu22.04`, installing Rust toolchains, CUDA PyTorch wheels, and Maturin, then runs `cargo fetch`. `docker-compose.gpu.yml` mounts the repo at `/workspace`, exposes all GPUs via `NVIDIA_VISIBLE_DEVICES=all`, and keeps the container ready for interactive sessions.  
4. **Runtime Launch** – `cargo run -p zappy-orchestrator` starts the control plane. Module demos (e.g., `cargo run -p zappy-actions --example orchestrate_sample`, `cargo run -p trn -- run --config ...`) enable focused testing.  
5. **LangModel Server** – `pip install fastapi uvicorn transformers accelerate bitsandbytes` plus `uvicorn langmodel.server:app --host 0.0.0.0 --port 9000`. Environment variables such as `LANGMODEL_ENDPOINT`, `LANGMODEL_CUDA_MEMORY`, and `LANGMODEL_API_KEY` configure production deployments.  
6. **Testing & Quality Gates** – `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `pytest ZAPPY--M/learning/pytorch_runner/tests` (where applicable) enforce regression coverage.

---

## 7. Roadmap & Future Work

1. **Telemetry sinks** – ship Redis/Kafka publishers and Grafana dashboards per `docs/observability_rollout.md` Phase 3.  
2. **Policy automation** – integrate signed policy updates (S3/KV) into `actions`, `autonomy`, and `planning` runtime broadcasts with real-time validation.  
3. **Learner extensibility** – formalize JSON schema validation for new datasets/models via CI, and expose `ExperienceHub` APIs to third-party learners.  
4. **LLM adaptability** – add automatic quantization fallback strategies, streaming responses, and reinforcement signals from ExperienceHub events.  
5. **Autonomy scaling** – attach additional module kinds (creativity, simulation) to the `ModuleRegistry` with adaptive prioritization based on telemetry trends.  
6. **Operational runbooks** – extend `docs/` with incident response playbooks, GPU capacity planning guides, and observability dashboards to support Tier-10 service levels.

---

## 8. Conclusion

ZAPPY treats AGI orchestration as a production discipline: every decision is policy-aware, every signal is replayable, every module is observable, and every deployment is self-diagnosing. The repository’s structure—Rust workspace crates, Python LLM service, shared telemetry libraries, policy ledger, and comprehensive documentation—forms a cohesive platform for sustained autonomy research and enterprise-grade operations. This whitepaper should serve as the canonical reference for stakeholders evaluating the scope, guarantees, and evolution of the project.


