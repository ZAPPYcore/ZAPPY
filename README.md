# ZAPPY 전체 시스템 오버뷰

Rust 기반 멀티 런타임과 Python 기반 모델 서버가 결합된 거대한 오케스트레이션 프로젝트다. 최상위 오케스트레이터(`entire_system_orchestrator.rs`)가 계획, 자율성, 행동, 학습, 메타인지, 세계 모델, 시뮬레이션, 자기개선, 자연어 모듈을 지속적으로 호출하며 정책 준수 여부까지 감시한다. 본 문서는 코드베이스 구조·루프 구조·모듈별 역할·운영 방법을 하나의 문서로 통합한다.

---

## 1. 리포지토리 지도

| 경로 | 설명 |
| --- | --- |
| `Cargo.toml` | 워크스페이스 선언. `ZAPPY--M/*` 하위 모든 모듈을 단일 빌드 그래프로 묶는다. |
| `ZAPPY--M/entire_system_orchestrator.rs` | 전체 시스템 CLI. 모든 런타임을 부팅하고 명령 루프를 유지한다. |
| `ZAPPY--M/actions` … `ZAPPY--M/world` | 핵심 러스트 크레이트. 각 디렉터리는 독립 `Cargo.toml`과 `src/lib.rs`를 갖고 모듈 단위 기능을 제공한다. |
| `ZAPPY--M/langmodel/` | FastAPI 기반 LLM 서버와 로컬 Mistral 모델 파일. 자연어 → IR 변환을 담당한다. |
| `ZAPPY--M/trn_cli/` | 학습 잡 실행 CLI(`trn`). 러스트 학습 헬퍼와 PyTorch 러너를 연결한다. |
| `docs/` | 파이프라인 프로토콜, 관측성, 학습/LLM 스키마 정의. JSON 스키마(`docs/agi_json_schema`) 포함. |
| `logs/` | 런타임별 JSONL 로그. `logs/orchestrator` 하위에 계획/추론/경험/업그레이드 로그가 생성된다. |
| `scripts/setup_env.py` | GPU·PyTorch 환경 검증 스크립트. 신규 노드 준비 시 실행한다. |
| `langmodel/*.safetensors` | 로컬 LLM 가중치. `server.py`가 4bit 로딩을 시도한다. |
| `Dockerfile`, `docker-compose.gpu.yml` | CUDA + Rust + Python 개발 컨테이너. GPU 패스스루 설정 포함. |
| `POLICIES.md` | 오케스트레이터가 부팅 시 적재하는 운영·보안 정책 원장. |
| `temp_prompt.py`, `temp/` | 실험 스크립트/임시 파일. 프로덕션 빌드에는 영향 없음. |

> **빌드 아티팩트:** `target/` 이하 모든 `.rlib/.exe/.pdb`는 Rust 빌드 캐시다. 수정 대상이 아니면 무시한다.

---

## 2. 런타임 아키텍처 & 주기 구조

### 2.1 오케스트레이터 명령 루프
1. `tokio::main`으로 부팅 후 `logs/orchestrator/*` 생성, `PolicyLedger` 로딩, 모든 런타임 인스턴스를 준비한다.
2. 표준 입력에서 명령을 읽고 `plan`, `simulate`, `knowledge`, `autonomy`, `execute`, `natural`, `reflect`, `experience`, `policy`, `status`, `help`, `exit` 등을 분기 처리한다.
3. 각 명령은 대응 모듈의 런타임 메서드를 호출하고, 결과를 `ExperienceHub` (`learning/pipeline.rs`)에 JSON 경험으로 기록한다.
4. `natural` 명령은 `NaturalLanguageClient`를 통해 `langmodel` FastAPI 서버에 요청 → IR(JSON) → 계획/지식/행동 파이프라인으로 재전송한다.

### 2.2 자율성 루프 (Autonomy Runtime)
1. `AutonomyRuntime::bootstrap()`은 `ModuleRegistry`에 `planner`, `executor`, `sensor` 및 필요 시 custom 모듈을 등록한다.
2. `autonomy cycle` 명령이 들어오면 `AutonomyLinker`가 `MasterController` 및 `Director`를 통해 모듈 지시를 결정한다.
3. 각 사이클은 `AutonomySignal`(scope, 지표, 태그)을 받고 `ControlDirective` 리스트, 신뢰도, 메트릭을 반환한다.
4. 사이클 결과는 `ExperienceHub`와 텔레메트리(`logs/autonomy/runtime.log.jsonl`)에 기록되어 후속 학습/지식 동기화에 사용된다.

### 2.3 계획 → 추론 → 행동 루프
1. **계획(PlanningRuntime)**: `LongTermPlanner` + `ShortTermPlanner` + `AdvancedPortfolioPlanner`로 전략/전술 플랜을 생성한다.
2. **추론(ReasoningRuntime)**: 계획 결과, Natural IR, 세계 시그널을 `SignalPacket`으로 수집해 `Verdict`(가설, 지시, confidence)을 만든다.
3. **행동(ActionCommander)**: `ActionRequest`(도메인, 의도, priority, safety, tags, attachments)를 받아 적절한 `ActionAgent`에게 위임 -> 프로그램/시뮬레이션/인프라 명령 실행.
4. **안전**: `ActionConstraints` + `ActionSafetyClass`로 정책을 검증하고, 실패 시 경험 버스에 거부 사유를 남긴다.

### 2.4 경험 → 지식 → 학습 순환
1. 모든 모듈이 `ExperienceHub.publish(module, signal, payload)` 호출로 최신 경험을 큐에 저장하고, `ExperienceRecorder`가 JSONL로 영속화한다.
2. `knowledge sync` 명령은 `ExperienceReplayService`에서 최근 경험을 읽어 `KnowledgeRuntime::ingest_experience()`로 지식 레코드를 확장한다.
3. `KnowledgeRuntime`은 외부 검색(`websearcher`), 로컬 편집(`editor`), 보안 심사(`security`)를 담당하며 `KnowledgeQuery`로 검색을 지원한다.
4. `learning` 모듈은 지식/경험을 데이터셋으로 변환, `DeviceManager` + `PyTorch Runner`를 통해 모델을 재학습하고, 결과 로그와 체크포인트를 `learning/logs`/`learning/models`에 저장한다.

### 2.5 세계 모델 & 시뮬레이션 루프
1. `world refresh` 명령은 `WorldRuntime`이 `feeds.toml`을 읽어 외부 피드를 동기화, `AssimilationEngine`으로 지역/이상치 상태를 재구성한다.
2. `simulate` 명령은 `SimulationEngine::run_advanced(SimulationMethod::HighFidelity, n)`을 호출해 시나리오/인사이트를 산출한다.
3. 시뮬레이션 결과는 세계 모델과 Reasoning, Self-Upgrade 루프의 입력이 되며, 정책 준수 여부를 재평가한다.

### 2.6 메타인지 & 자기개선 루프
1. `reflect` 명령은 `MetacognitionRuntime::reflect(SelfObservation, ReflectionMethod)`를 실행하며, `rapid/structured/audit` 모드를 지원한다.
2. 자기개선(`upgrade`) 명령은 `SelfUpgradeRuntime`이 `UpgradeDirective`를 받아 진단 → 실행 계획 → 보고서를 생성하고 `logs/orchestrator/upgrades`에 결과를 남긴다.
3. 메타인지 요약과 자기개선 결과는 보고서 생성(`NaturalLanguageClient::generate_report`) 및 정책 감사 루틴의 핵심 입력이다.

### 2.7 관측성·정책 적용
- 모든 런타임은 `shared_logging::JsonLogger`와 `shared_event_bus::MemoryEventBus`/`FileEventPublisher`를 사용한다.
- `POLICIES.md`는 부팅 시 로드되어 `policy` 명령으로 확인 가능하며, 위반 시 `ExperienceHub`에 `POLICY-*` 이벤트를 남기게 되어 있다.
- 추가 문서는 `docs/logging_event_bus.md`, `docs/observability_rollout.md`, `docs/pipeline_protocol.md`를 참고한다.

---

## 3. 모듈별 설명 (요약)

### Actions (`ZAPPY--M/actions`)
- **핵심 역할**: 모든 행위 요청(ActionRequest)을 수락·검증·실행하고, 도메인별 Agent를 통해 실제 명령(프로그래밍, 인터넷, 오프라인, 시뮬레이션 등)을 수행한다.
- **중요 타입/파일**: `actions.rs`(도메인/의도/우선순위), `actioncommander.rs`, `commandgeneration.rs`, `security_link.rs`, `telemetry.rs`.
- **흐름**: 요청 → `SecurityLink` 정책 검사 → `ActionAgent` 선택 → 실행/후속 조치 → `ActionTelemetry` 기록.
- **샘플**: `actions/main.rs`의 `orchestrate_sample()`이 end-to-end 예시를 제공.

### Autonomy (`ZAPPY--M/autonomy`)
- **역할**: 다양한 모듈을 브로커(`ModuleBroker`)로 묶어 주기적인 결정 사이클을 돌린다.
- **구성 요소**: `decision/`, `linker.rs`, `master/`, `module/`, `telemetry.rs`.
- **루프**: `AutonomyLinker::execute_cycle` → `MasterController` → 모듈 지시 → `CycleReport`.
- **CLI 통합**: `autonomy cycle|directive|metrics` 하위 명령으로 접근.

### Creativity (`ZAPPY--M/creativity`)
- **역할**: 아이디어 생성/평가, 내러티브 직조, 창의 포트폴리오 관리.
- **핵심 파일**: `create.rs`, `mainfunc/advanced.rs`, `telemetry.rs`.
- **사용**: 다른 모듈에서 창의 브리프(`CreativeBrief`)를 생성하거나 검증할 때 링크.

### Knowledge (`ZAPPY--M/knowledge`)
- **역할**: 아티팩트 수신(`receiver`), 저장(`saver`), 검색(`seeker`), 웹 확장(`websearcher`), 편집(`editor`), 보안(`security`).
- **주요 API**: `KnowledgeRuntime::ingest`, `search`, `ingest_experience`.
- **CLI**: `knowledge sync|search|ingest`.

### Learning (`ZAPPY--M/learning`)
- **역할**: 데이터셋/모델/로그 구조 관리, Rust 기반 장치+데이터 로더, PyTorch 러너.
- **중요 파일**: `dataloader.rs`, `device_manager.rs`, `pipeline.rs`(ExperienceHub), `pytorch_runner/`, `telemetry.rs`, `README.md`.
- **CLI 연동**: `trn` 바이너리(`trn_cli`)가 러너를 호출.

### Memory Data (`ZAPPY--M/memory_data`)
- **역할**: 단기·장기 메모리 계층을 통합. 중요도 기반 보존 및 텔레메트리 제공.
- **구성**: `short_term/`, `long_term/`, `telemetry.rs`, `main.rs`.

### Metacognition (`ZAPPY--M/metacognition`)
- **역할**: 자기 관찰, 반성, 스크립트 생성. `ReflectionMethod`를 통해 다양한 심도 반성을 지원.
- **파일**: `cognition.rs`, `methods.rs`, `executor.rs`, `reporter.rs`, `telemetry.rs`.

### NLP (`ZAPPY--M/nlp`)
- **역할**: 자연어 이해/응답, 콘솔 명령 수집, 데이터셋/훈련, PyTorch 트레이너 연동.
- **중요 파일**: `answer.rs`, `comprehension/main.rs`, `langtrain.rs`, `consolecmdreciever.rs`, `main.rs`.
- **오케스트레이션**: `NaturalLanguageClient`가 IR을 생성하고 필요한 모듈 목록을 반환.

### Planning (`ZAPPY--M/planning`)
- **역할**: 장기·단기 계획 엔진, 고급 포트폴리오, 스코어링, 검토.
- **구성**: `long_term/`, `short_term/`, `module.rs`, `telemetry.rs`.

### Reasoning (`ZAPPY--M/reasoning`)
- **역할**: 멀티 도메인 추론 엔진, 시그널 그래프, 가설 및 verdict 생성.
- **구성**: `engine.rs`, `module.rs`, `multidomain/`, `telemetry.rs`.

### Self-Upgrade (`ZAPPY--M/self_upgrade`)
- **역할**: 시스템 진단, 개선 계획, 실행 및 보고.
- **구성**: `checker.rs`, `selfupgarde.rs`, `reporter.rs`, `reviewer.rs`, `telemetry`.

### Simulation Engine (`ZAPPY--M/simulationengine`)
- **역할**: 시나리오 생성, 예측, 비교, 리뷰, 고급 보고.
- **구성**: `simul_env_generator.rs`, `predictor.rs`, `compare.rs`, `advanced/`, `runtime`.

### World (`ZAPPY--M/world`)
- **역할**: 외부 피드 ingest, 세계 상태 모델링, 정보 탐색, 특징 저장.
- **구성**: `model.rs`, `learning.rs`, `infoseeker.rs`, `feature_store.rs`, `advanced/`.

### Shared Libraries
- `shared_event_bus`: In-memory/durable 이벤트 버스, `EventRecord`, `MemoryEventBus`, `FileEventPublisher`.
- `shared_logging`: JSON 라인 로거, `LogRecord`, `JsonLogger`.

### LangModel (`ZAPPY--M/langmodel`)
- **역할**: FastAPI + Transformers로 로컬 LLM을 서비스. 4bit `BitsAndBytes` 로딩 우선, 실패 시 CPU bf16.
- **엔드포인트**: `POST /generate` (`GenerateRequest`/`GenerateResponse`).
- **환경 변수**: `LANGMODEL_HOST`, `LANGMODEL_PORT`, `LANGMODEL_ENDPOINT`, `LANGMODEL_API_KEY`, `LANGMODEL_JWT`, `LANGMODEL_CUDA_MEMORY`.
- **예외 필터**: `except.txt`에 포함된 금칙어가 응답에 나타나면 “처리 불가능” 반환.
- **프롬프트 템플릿**: JSON 모드가 아닌 일반 질문/요청에는 내부 지시문이 자동으로 붙어 결과·맥락·다음 단계를 담은 부드러운 자연어 응답을 생성한다.
- **출력 길이**: `max_tokens`는 기본 512, 최대 3,072까지 허용해 긴 보고서/설명을 생성할 수 있다.

### Conversational Web UI (`webui/`)
- **구성**: `webui/server.py`(FastAPI 기반 역프록시) + `webui/index.php`(UI·프록시 일체형). PHP 레이어는 UI와 API 프록시를 단일 파일로 통합해 배포가 간단하다.
- **env 설정**: `webui/env.php`에서 `server_url`(예: `http://127.0.0.1:8080/api/chat`), `server_password`(FastAPI가 기대하는 값), `vpn_block` 옵션을 조정한다. 필요 시 `WEBUI_SERVER_URL` 등 환경 변수로 덮어쓸 수 있다.
- **웹 서버 실행**:
  ```bash
  # FastAPI 역프록시 (Rust CLI + LangModel과 통신)
  uvicorn webui.server:app --host 0.0.0.0 --port 8080

  # PHP UI (단일 index.php)
  cd webui
  php -S 0.0.0.0:8081 index.php
  ```
- **동작 방식**: PHP UI가 `env.php`에 정의된 URL로 POST `/api/chat`을 보내며, Chat Password는 서버 측에서 자동 포함된다. 사용자 입력 시 비밀번호가 필요 없다.
- **VPN 차단**: `vpn_block.enabled=true`이면 `ipapi.co`(기본)로 IP 메타데이터를 조회해 VPN/Proxy 접속을 451 상태로 차단한다. 캐시는 `sys_get_temp_dir()`에 저장되며 TTL은 `vpn_block.cache_ttl`.
- **설정 사이드바**: GPU/CPU 선호, 요약 길이, 응답 언어, 톤, 안전 모드, 중점 모듈(최소 5개 옵션)을 실시간으로 선택하면 요청 페이로드에 반영되어 Orchestrator/LangModel 동작에 즉시 적용된다.

### TRN CLI (`ZAPPY--M/trn_cli`)
- **역할**: `trn run/status/list/resume` 명령 제공, Rust 학습 런타임과 Python 러너 연결.
- **의존성**: `zappy-learning`, `shared-*`.
- **계획 문서**: `docs/pytorch_runner/README.md`.

---

## 4. 데이터·로그·정책
- **경험 데이터**: `logs/orchestrator/experience.log.jsonl` + `ExperienceHub` 메모리 원형 버퍼. `ExperienceArchive`가 tail/since API 제공.
- **지식 데이터**: `ZAPPY--M/knowledge/saver.rs`가 로컬 저장소를 유지. 외부 소스 ingest 시 `security` 모듈이 위험 점수 부여.
- **학습 데이터셋**: `learning/datasets/<dataset>/schema.json + shards/`. JSON 스키마는 `docs/agi_json_schema/dataset.schema.json`.
- **모델 체크포인트**: `learning/models/<model>/checkpoints/checkpoint-*.pt` + `.meta.json`.
- **관측성**: 각 모듈의 `telemetry.rs`가 `shared_logging`+`shared_event_bus` 빌더를 노출. `docs/logging_event_bus.md` 참조.
- **정책**: `POLICIES.md`를 `PolicyLedger`가 로드하여 `policy` 명령으로 내용 출력. 위반 시 `self_upgrade` 루프 트리거 정책 존재.

---

## 5. 실행 및 사용법

### 5.1 사전 요구사항
- Rust 1.75+ (`rustup` 추천), Cargo.
- Python 3.10+, `pip`, `virtualenv`.
- GPU 학습/LLM 시 NVIDIA 드라이버 및 CUDA 12.1 대응 PyTorch, 혹은 CPU 모드.
- 30GB 이상 저장 공간 (LLM + 체크포인트).

### 5.2 환경 준비
1. (선택) Docker: `docker compose -f docker-compose.gpu.yml up -d` → 컨테이너 진입.
2. 로컬: `python scripts/setup_env.py --output logs/env/report.json`으로 시스템 리포트 확인.
3. Rust 의존성: `cargo fetch` 또는 `cargo build --workspace`.
4. Python LLM/학습 패키지: `pip install -r requirements`(문서 없음) 대신 README 지침대로 `fastapi`, `uvicorn`, `transformers`, `accelerate`, `bitsandbytes` 설치.

### 5.3 전체 오케스트레이터 실행
```bash
cargo run -p zappy-orchestrator
```
- 프롬프트 `orchestrator>`가 뜨면 다음 명령을 사용할 수 있다.

| 명령 | 설명 |
| --- | --- |
| `plan <objective>` | 전략 계획 생성 (`PlanningRuntime`). |
| `schedule` | 마지막 전략 계획으로 전술 스케줄 생성. |
| `world` | 세계 모델 최신화. |
| `simulate [count]` | 고해상도 시뮬레이션 실행. |
| `upgrade [target]` | Self-Upgrade 지시. |
| `reflect <text> [--method=rapid|structured|audit]` | 메타인지 실행. |
| `execute <domain> <intent> ... | details` | ActionCommander 경로. Priority/Safety/Tag 플래그 지원. |
| `autonomy cycle|directive|metrics` | AutonomyRuntime 하위 명령. |
| `knowledge sync|search|ingest` | 지식 동기화·검색·수동 추가. |
| `natural <text>` | 자연어 → IR. Confidence<0.7이면 “ambiguous”. |
| `experience [n]` | ExperienceHub 최신 N개 이벤트 출력. |
| `policy` | `POLICIES.md` 내용 표시. |
| `status` | 마지막 계획/IR 상태 출력. |
| `help`/`exit` | 도움말/종료. |

### 5.4 모듈 독립 실행 예
- **Actions 샘플**: `cargo run -p zappy-actions --example orchestrate_sample` (또는 `actions/main.rs` 기반 바이너리 구성).
- **Autonomy 데모 루프**: `cargo test -p zappy-autonomy demo_run` 혹은 `autonomy::demo_run(iterations)` 호출.
- **Learning CLI**: `cargo run -p trn -- run --config ZAPPY--M/learning/configs/sample_train.json`.
- **Intent 전용 학습**: `cargo run -p trn -- run --config ZAPPY--M/learning/configs/intent_understanding.json --log-dir build/logs --event-log build/events/intent.jsonl`.
- **PyTorch Runner (개별)**: `python ZAPPY--M/learning/pytorch_runner/main.py --config ...`.
- **Knowledge 수동 ingest**: `knowledge ingest source | title | body`.
- **Memory 테스트**: `cargo test -p zappy-memory_data`.

### 5.5 LangModel 서버
```bash
cd ZAPPY--M/langmodel
pip install fastapi uvicorn transformers accelerate bitsandbytes
uvicorn langmodel.server:app --host 0.0.0.0 --port 9000
```
- `LANGMODEL_ENDPOINT`를 지정하지 않으면 오케스트레이터는 `http://127.0.0.1:9000/generate`로 요청한다.
- GPU 메모리 한계 시 `LANGMODEL_CUDA_MEMORY`로 제한, 실패하면 자동으로 CPU bf16으로 폴백.

### 5.6 Docker 흐름
1. `docker compose -f docker-compose.gpu.yml up --build -d`.
2. `docker exec -it zappy-agi-dev bash`.
3. 컨테이너 내에서 `cargo run -p zappy-orchestrator`.
4. GPU 접근을 위해 호스트에서 NVIDIA Container Toolkit 필요.

### 5.7 테스트 & 검증
- 전 범위 테스트: `cargo test --workspace`.
- 모듈 단위: `cargo test -p zappy-planning`, `cargo test -p shared-event-bus`, 등.
- Lint/클리피: `cargo clippy --workspace --all-targets -- -D warnings`.
- Python 러너: `pytest ZAPPY--M/learning/pytorch_runner/tests` (테스트 추가 시).

---

## 6. 문제 해결 & 운영 팁
- **LLM 응답이 JSON이 아닐 때**: `NaturalLanguageClient`가 마지막 `{ ... }` 블록만 파싱하므로, `langmodel` 로그(`print("[langmodel] raw output", text)`)를 확인하고 프롬프트 규칙을 준수한다.
- **명령 보안**: `execute` 시 `--safety=red`는 정책이 막을 수 있으므로, `POLICIES.md`를 최신 상태로 유지하고 필요 시 `security_link.rs` 설정을 조정한다.
- **경험 버스 누락**: `logs/orchestrator/experience.log.jsonl`가 비어 있으면 `ExperienceRecorder` 경로 권한을 확인한다.
- **GPU 미검출**: `scripts/setup_env.py` 결과에서 `recommended_action`이 `gpu_not_detected`인 경우 Docker/NVIDIA 설정을 재검토.
- **LangModel 연결 실패**: `LANGMODEL_ENDPOINT` 환경 변수를 확인하고 `ensure_langmodel_server`가 접근 가능한지 curl로 사전 확인.
- **데이터 스키마**: 새 데이터셋/모델을 추가할 때는 `docs/agi_json_schema/*.schema.json`을 먼저 검토하여 형식을 맞춘다.

---

## 7. 참고 문서
- `docs/pipeline_protocol.md` – 계획/자율/행동 파이프라인 단계별 프로토콜.
- `docs/observability_rollout.md` – 로그/이벤트 버스 배포 전략.
- `docs/logging_event_bus.md` – 공유 로깅과 이벤트 버스 세부사항.
- `docs/pytorch_runner/README.md` – 트레이닝 CLI 아키텍처.
- 각 모듈 디렉터리 내부 `README` 또는 주석 – 모듈 고유 세부 설명.

이 README는 전체 시스템의 구조적 맥락과 실사용 방법을 한 번에 파악할 수 있도록 설계되었다. 신규 기여자는 본 문서 → `docs/` 세부 문서 → 모듈별 소스 순으로 읽으며 온보딩하는 것이 가장 빠르다.


