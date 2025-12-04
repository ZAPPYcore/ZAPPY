use std::{
    collections::HashSet,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use shared_event_bus::MemoryEventBus;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};
use uuid::Uuid;
use zappy_actions::{
    actioncommander::ActionCommander,
    actions::{
        ActionConstraints, ActionDomain, ActionIntent, ActionMetadata, ActionPayload,
        ActionPriority, ActionRequest, ActionSafetyClass, PayloadAttachment,
    },
    telemetry::ActionTelemetry,
};
use zappy_autonomy::{AutonomyRuntime, AutonomySignal, DirectivePriority, ModuleKind, SignalScope};
use zappy_knowledge::{KnowledgeArtifact, KnowledgeQuery, KnowledgeRuntime, KnowledgeSnippet};
use zappy_learning::{
    pipeline::{ExperienceArchive, ExperienceHub, ExperienceRecorder},
    ExperienceReplayService,
};
use zappy_metacognition::{
    cognition::SelfObservation,
    methods::ReflectionMethod,
    orchestration_entry::{MetacognitionRuntime, ReflectionDigest},
    telemetry::MetacognitionTelemetry,
};
use zappy_planning::long_term::{
    advanced::AdvancedPortfolioPlanner, engine::PlanScoringEngine, reviewer::StrategicPlanReviewer,
    LongTermPlanner,
};
use zappy_planning::{PlanningDirective, PlanningRuntime, PlanningTelemetry, ShortTermPlanner};
use zappy_reasoning::{
    module::{DirectivePriority as ReasoningPriority, Verdict},
    ReasoningDirective, ReasoningRuntime, ReasoningTelemetryBuilder, SignalPacket,
};
use zappy_self_upgrade::{SelfUpgradeRuntime, UpgradeDirective};
use zappy_simulationengine::{
    advanced::report::SimulationReport, methods::SimulationMethod, SimulationEngine,
};
use zappy_world::{AssimilationJob, WorldRuntime};

/// High-level orchestrator controlling every major runtime.
pub struct EntireSystemOrchestrator {
    planning: PlanningRuntime,
    world: WorldRuntime,
    simulation: SimulationEngine,
    self_upgrade: SelfUpgradeRuntime,
    metacognition: MetacognitionRuntime,
    knowledge: KnowledgeRuntime,
    autonomy: AutonomyRuntime,
    reasoning: ReasoningRuntime,
    actions: ActionCommander,
    experience_hub: Arc<ExperienceHub>,
    experience_replay: ExperienceReplayService,
    policies: PolicyLedger,
    last_plan: Option<zappy_planning::StrategicPlan>,
    natural_language: NaturalLanguageClient,
    last_natural_ir: Option<NaturalLanguageIr>,
}

const MAX_FILE_SNAPSHOT_BYTES: usize = 200_000;
const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 60_000;
const MAX_COMMAND_TIMEOUT_MS: u64 = 300_000;
const MAX_COMMANDS_PER_REQUEST: usize = 5;

impl EntireSystemOrchestrator {
    /// Bootstraps all runtimes with telemetry and storage paths.
    async fn bootstrap() -> Result<Self> {
        std::fs::create_dir_all("logs/orchestrator")?;
        let policies = PolicyLedger::load("POLICIES.md")?;
        let natural_language = NaturalLanguageClient::new_from_env().await?;
        let planning_bus = Arc::new(MemoryEventBus::new(256));
        let planning_telemetry = PlanningTelemetry::builder("planning-orchestrator")
            .event_publisher(planning_bus)
            .log_path(PathBuf::from("logs/orchestrator/planning.log.jsonl"))
            .build()
            .ok();
        let mut planning_runtime = PlanningRuntime::new(
            LongTermPlanner::default(),
            ShortTermPlanner::default(),
            planning_telemetry.clone(),
        );
        let advanced_planner = AdvancedPortfolioPlanner::new(
            LongTermPlanner::default(),
            PlanScoringEngine::new(),
            StrategicPlanReviewer::default(),
            planning_telemetry.clone(),
        );
        planning_runtime.set_advanced(advanced_planner);

        let world_runtime = WorldRuntime::builder()
            .feeds_config_path("ZAPPY--M/world/data/feeds.toml")
            .context("loading world feeds config")?
            .feature_store_path("logs/orchestrator/world_features.jsonl")
            .context("initializing world feature store")?
            .build()?;
        let simulation = SimulationEngine::builder().build()?;
        let self_upgrade = SelfUpgradeRuntime::builder()
            .report_dir("logs/orchestrator/upgrades")
            .build()?;
        let meta_bus = Arc::new(MemoryEventBus::new(64));
        let meta_telemetry = MetacognitionTelemetry::builder("metacognition-orchestrator")
            .event_publisher(meta_bus)
            .log_path(PathBuf::from("logs/orchestrator/metacognition.log.jsonl"))
            .build()
            .ok();
        let mut metacognition = MetacognitionRuntime::new();
        if let Some(tel) = meta_telemetry {
            metacognition.set_telemetry(tel);
        }
        let experience_log_path = PathBuf::from("logs/orchestrator/experience.log.jsonl");
        let experience_recorder = Arc::new(
            ExperienceRecorder::new(&experience_log_path)
                .context("initializing experience recorder")?,
        );
        let experience_hub =
            Arc::new(ExperienceHub::new(512).with_recorder(experience_recorder.clone()));
        let experience_archive = ExperienceArchive::new(&experience_log_path);
        let experience_replay =
            ExperienceReplayService::new(experience_hub.clone(), Some(experience_archive));
        let action_bus = Arc::new(MemoryEventBus::new(256));
        let action_telemetry = ActionTelemetry::builder("actions-orchestrator")
            .event_publisher(action_bus)
            .log_path(PathBuf::from("logs/orchestrator/actions.log.jsonl"))
            .build()
            .ok();
        let mut action_builder = ActionCommander::builder();
        if let Some(tel) = action_telemetry {
            action_builder = action_builder.telemetry(tel);
        }
        let actions = action_builder.build();
        let reasoning_bus = Arc::new(MemoryEventBus::new(64));
        let reasoning_telemetry = ReasoningTelemetryBuilder::new("reasoning-orchestrator")
            .event_publisher(reasoning_bus)
            .log_path(PathBuf::from("logs/orchestrator/reasoning.log.jsonl"))
            .build()
            .ok();
        let reasoning = ReasoningRuntime::new(reasoning_telemetry);
        let knowledge = KnowledgeRuntime::bootstrap();
        let autonomy = AutonomyRuntime::bootstrap();

        Ok(Self {
            planning: planning_runtime,
            world: world_runtime,
            simulation,
            self_upgrade,
            metacognition,
            knowledge,
            autonomy,
            reasoning,
            actions,
            experience_hub,
            experience_replay,
            natural_language,
            policies,
            last_plan: None,
            last_natural_ir: None,
        })
    }

    async fn run(&mut self) -> Result<()> {
        println!("🚀 Entire System Orchestrator ready. Type 'help' for options.");
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        loop {
            print!("orchestrator> ");
            io::stdout().flush()?;
            let line = match reader.next_line().await? {
                Some(line) => line.trim().to_string(),
                None => break,
            };
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, ' ');
            let command = parts.next().unwrap();
            let args = parts.next().unwrap_or("").trim();
            match command {
                "plan" => self.handle_plan(args).await?,
                "schedule" => self.handle_schedule().await?,
                "world" => self.handle_world().await?,
                "simulate" => self.handle_simulation(args).await?,
                "upgrade" => self.handle_upgrade(args).await?,
                "reflect" => self.handle_reflect(args).await?,
                "execute" => self.handle_execute(args).await?,
                "autonomy" => self.handle_autonomy(args).await?,
                "knowledge" => self.handle_knowledge(args).await?,
                "natural" | "natural_language" => self.handle_natural_language(args).await?,
                "experience" => self.handle_experience(args)?,
                "policy" => self.handle_policy()?,
                "status" => self.print_status().await?,
                "help" => Self::print_help(),
                "exit" | "quit" => break,
                other => println!("Unknown command: {other}. Type 'help' for usage."),
            }
        }
        Ok(())
    }

    async fn handle_plan(&mut self, description: &str) -> Result<()> {
        let desc = if description.is_empty() {
            "stabilize core systems"
        } else {
            description
        };
        let directive = PlanningDirective::critical(desc);
        let plan = self
            .planning
            .propose_strategic_plan(vec![directive])
            .context("planning failed")?;
        match plan {
            Some(plan) => {
                println!(
                    "Plan created for {} with {} phases.",
                    plan.objective.description,
                    plan.phases.len()
                );
                self.record_experience(
                    "planning",
                    "plan.generated",
                    json!({ "objective": plan.objective.description, "phases": plan.phases.len() }),
                );
                self.last_plan = Some(plan);
            }
            None => println!("No plan generated (insufficient directives)."),
        }
        Ok(())
    }

    async fn handle_schedule(&mut self) -> Result<()> {
        if let Some(plan) = &self.last_plan {
            let schedule = self.planning.build_tactical_schedule(plan)?;
            println!(
                "Tactical schedule: {} tasks spanning {} hours.",
                schedule.tasks.len(),
                schedule.horizon_hours
            );
            self.record_experience(
                "planning",
                "schedule.generated",
                json!({ "tasks": schedule.tasks.len(), "hours": schedule.horizon_hours }),
            );
        } else {
            println!("No plan available. Run `plan <objective>` first.");
        }
        Ok(())
    }

    async fn handle_world(&mut self) -> Result<()> {
        let state = self.world.refresh().await?;
        println!(
            "World refreshed: {} regions, {} anomalies.",
            state.regions.len(),
            state.anomalies.len()
        );
        self.record_experience(
            "world",
            "state.refresh",
            json!({ "regions": state.regions.len(), "anomalies": state.anomalies.len() }),
        );
        Ok(())
    }

    async fn handle_simulation(&mut self, args: &str) -> Result<()> {
        let count: usize = args.parse().unwrap_or_else(|_| 2);
        let report = self
            .simulation
            .run_advanced(SimulationMethod::HighFidelity, count)
            .await?;
        println!(
            "Simulation report generated: {} scenarios, {} insights.",
            report.scenario_count,
            report.insights.len()
        );
        self.record_experience(
            "simulation",
            "report.generated",
            json!({ "scenarios": report.scenario_count, "insights": report.insights.len() }),
        );
        Ok(())
    }

    async fn handle_upgrade(&mut self, args: &str) -> Result<()> {
        let target = if args.is_empty() {
            "core-system-vNEXT"
        } else {
            args
        };
        let directive = UpgradeDirective::new(
            format!("Upgrade orchestrator stack to {target}"),
            target,
            85,
        );
        let plan = self.self_upgrade.execute(directive)?;
        println!(
            "Upgrade completed with {} actions. Status: {:?}",
            plan.actions.len(),
            plan.status
        );
        self.record_experience(
            "self_upgrade",
            "upgrade.completed",
            json!({ "actions": plan.actions.len(), "status": format!("{:?}", plan.status) }),
        );
        Ok(())
    }

    async fn print_status(&self) -> Result<()> {
        println!(
            "Status:\n  Last plan: {}\n  Logs in ./logs/orchestrator",
            if let Some(plan) = &self.last_plan {
                plan.objective.description.as_str()
            } else {
                "none"
            }
        );
        if let Some(ir) = &self.last_natural_ir {
            println!(
                "  Last IR: intent={} confidence={} modules={:?}",
                ir.intent
                    .intent_type
                    .clone()
                    .unwrap_or_else(|| "unknown".into()),
                ir.intent.confidence.unwrap_or(0.0),
                ir.modules
            );
        } else {
            println!("  Last IR: none");
        }
        Ok(())
    }

    fn print_help() {
        println!(
            "Commands:
  plan <objective>    - Generate strategic plan
  schedule            - Build tactical schedule from last plan
  world               - Refresh world model
  simulate [count]    - Run high-fidelity simulation (default count=2)
  upgrade [target]    - Execute self-upgrade workflow
  execute ...         - Dispatch action (see README for syntax)
  autonomy ...        - Autonomy controls (cycle/directive/metrics)
  knowledge ...       - Knowledge ops (sync/search/ingest)
  natural <text>      - Natural language to IR classification
  reflect <text>      - Trigger metacognition reflection
  experience [n]      - Show latest ExperienceHub entries
  policy              - Print active security/ops policies
  status              - Print orchestrator state
  help                - Show this message
  exit                - Quit orchestrator"
        );
    }

    async fn handle_reflect(&self, args: &str) -> Result<()> {
        if args.is_empty() {
            println!("Usage: reflect <observation> [--method=rapid|structured|audit]");
            return Ok(());
        }
        let (method, text) = Self::parse_reflect_args(args);
        let observation = SelfObservation::new(text, 0.55);
        let digest = self.metacognition.reflect(observation, method).await?;
        println!(
            "Reflection summary: {} (resilience {:.2})",
            digest.summary, digest.resilience
        );
        self.record_experience(
            "metacognition",
            "reflection.completed",
            json!({
                "method": format!("{:?}", method),
                "summary": digest.summary,
                "resilience": digest.resilience
            }),
        );
        Ok(())
    }

    async fn handle_execute(&self, args: &str) -> Result<()> {
        let parsed = match ExecutionArgs::parse(args) {
            Ok(v) => v,
            Err(err) => {
                println!("Usage: {}\nError: {err}", ExecutionArgs::usage());
                return Ok(());
            }
        };
        let report = self.execute_request(parsed).await?;
        if !report.success {
            if let Some(reason) = report.failure_reason {
                println!("Execution finished with failure: {reason}");
            }
        }
        Ok(())
    }

    async fn execute_request(&self, exec: ExecutionArgs) -> Result<ActionExecutionReport> {
        let ExecutionArgs {
            domain,
            intent,
            summary,
            narrative,
            priority,
            safety,
            tags,
            attachments,
        } = exec;
        let domain_label = domain.label().to_string();
        let intent_label = intent.label().to_string();
        let payload = ActionPayload {
            summary: summary.clone(),
            narrative: narrative.clone(),
            attachments,
        };
        let mut builder = ActionRequest::builder(domain, intent, payload).priority(priority);
        let mut metadata = ActionMetadata::default();
        for tag in tags {
            metadata = metadata.with_tag(tag);
        }
        builder = builder.metadata(metadata);
        builder = builder.constraints(ActionConstraints {
            safety,
            ..ActionConstraints::default()
        });
        let request = builder.build();
        println!(
            "Dispatching action [{} -> {}]: {}",
            domain_label, intent_label, summary
        );
        let handle = match self.actions.submit(request).await {
            Ok(handle) => handle,
            Err(err) => {
                println!("Action rejected: {err}");
                self.record_experience(
                    "actions",
                    "action.rejected",
                    json!({ "error": err.to_string(), "domain": domain_label }),
                );
                return Ok(ActionExecutionReport::failure(err.to_string()));
            }
        };
        match handle.outcome().await {
            Ok(outcome) => {
                println!("Action completed: {}", outcome.summary);
                if !outcome.follow_up.is_empty() {
                    println!("Follow-up: {}", outcome.follow_up.join("; "));
                }
                self.record_experience(
                    "actions",
                    "action.completed",
                    json!({
                        "summary": outcome.summary,
                        "domain": domain_label,
                        "intent": intent_label,
                        "cpu_ms": outcome.metrics.cpu_time_ms,
                        "energy_kwh": outcome.metrics.energy_kwh
                    }),
                );
                Ok(ActionExecutionReport::success())
            }
            Err(err) => {
                let reason = err.to_string();
                println!("Action execution failed: {reason}");
                self.record_experience(
                    "actions",
                    "action.failed",
                    json!({ "error": reason, "domain": domain_label }),
                );
                Ok(ActionExecutionReport::failure(reason))
            }
        }
    }

    fn handle_experience(&self, args: &str) -> Result<()> {
        let limit = args.parse().unwrap_or(5);
        let events = self.experience_hub.snapshot(limit);
        if events.is_empty() {
            println!("ExperienceHub is empty.");
        } else {
            for event in events {
                println!(
                    "[{}] {}::{} => {}",
                    event.timestamp.to_rfc3339(),
                    event.module,
                    event.signal,
                    event.payload
                );
            }
        }
        Ok(())
    }

    fn handle_policy(&self) -> Result<()> {
        println!(
            "Loaded policies from {}:\n{}",
            self.policies.path.display(),
            self.policies.content
        );
        Ok(())
    }

    fn record_experience(&self, module: &str, signal: &str, payload: Value) {
        let summary_view = payload.clone();
        self.experience_hub.publish(module, signal, payload);
        self.print_natural_summary(module, signal, &summary_view);
    }

    fn print_natural_summary(&self, module: &str, signal: &str, payload: &Value) {
        let preview = Self::payload_preview(payload);
        println!("{module} 모듈 {signal}: {preview}");
    }

    fn payload_preview(payload: &Value) -> String {
        match payload {
            Value::Null => "내용 없음".into(),
            Value::Bool(b) => format!("값: {b}"),
            Value::Number(num) => num.to_string(),
            Value::String(text) => Self::truncate_text(text),
            Value::Array(arr) => format!("{}개의 항목 요약", arr.len()),
            Value::Object(map) => {
                let summary = map
                    .iter()
                    .take(4)
                    .map(|(k, v)| format!("{k}={}", Self::value_atom(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                if map.len() > 4 {
                    format!("{summary}, ...")
                } else if summary.is_empty() {
                    "빈 객체".into()
                } else {
                    summary
                }
            }
        }
    }

    fn value_atom(value: &Value) -> String {
        match value {
            Value::String(text) => Self::truncate_text(text),
            Value::Number(num) => num.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => format!("배열({})", arr.len()),
            Value::Object(obj) => format!("객체({}키)", obj.len()),
            Value::Null => "null".into(),
        }
    }

    fn truncate_text(text: &str) -> String {
        const LIMIT: usize = 120;
        if text.chars().count() <= LIMIT {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(LIMIT).collect();
            format!("{truncated}…")
        }
    }

    fn parse_reflect_args(input: &str) -> (ReflectionMethod, String) {
        let mut method = ReflectionMethod::StructuredAnalysis;
        let mut text = input.trim().to_string();
        if let Some(token) = input.split_whitespace().next() {
            if let Some(rest) = token.strip_prefix("--method=") {
                method = match rest {
                    "rapid" => ReflectionMethod::RapidReview,
                    "structured" => ReflectionMethod::StructuredAnalysis,
                    "audit" => ReflectionMethod::ComprehensiveAudit,
                    _ => ReflectionMethod::StructuredAnalysis,
                };
                text = input[token.len()..].trim().to_string();
            }
        }
        (method, text)
    }

    async fn handle_knowledge(&mut self, args: &str) -> Result<()> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            Self::print_knowledge_help();
            return Ok(());
        }
        let mut parts = trimmed.split_whitespace();
        let subcmd = parts.next().unwrap();
        match subcmd {
            "sync" => {
                let limit = parts
                    .next()
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(64);
                let events = self.experience_replay.latest(limit)?;
                let mut ingested = 0usize;
                for envelope in events {
                    if let Some(record) = self.knowledge.ingest_experience(&envelope)? {
                        ingested += 1;
                        self.record_experience(
                            "knowledge",
                            "experience.ingested",
                            json!({
                                "record_id": record.id,
                                "module": envelope.module,
                                "signal": envelope.signal
                            }),
                        );
                    }
                }
                println!("Knowledge sync complete: {ingested} new records.");
            }
            "search" => {
                let query_text = parts.collect::<Vec<_>>().join(" ");
                if query_text.is_empty() {
                    println!("Usage: knowledge search <query text>");
                    return Ok(());
                }
                let outcome = self
                    .perform_enriched_knowledge_search(&query_text)
                    .await?;
                if outcome.snippets.is_empty() {
                    println!("No knowledge snippets found for '{query_text}'.");
                } else {
                    println!(
                        "Knowledge results for '{query_text}' (웹 신규 {}건 포함):",
                        outcome.new_records
                    );
                    for snippet in outcome.snippets.iter().take(5) {
                        println!(
                            "- [{} | score {:.2}] {}",
                            snippet.record_id,
                            snippet.score,
                            snippet.excerpt.replace('\n', " ")
                        );
                    }
                }
            }
            "ingest" => {
                let payload = trimmed.splitn(2, ' ').nth(1).unwrap_or("").trim();
                let segments: Vec<&str> = payload.split('|').map(|s| s.trim()).collect();
                if segments.len() < 3 {
                    println!("Usage: knowledge ingest <source> | <title> | <body>");
                    return Ok(());
                }
                let artifact = KnowledgeArtifact::new(segments[0], segments[1], segments[2]);
                match self.knowledge.ingest(artifact) {
                    Ok(record) => {
                        println!("Knowledge record ingested: {}", record.id);
                        self.record_experience(
                            "knowledge",
                            "record.manual_ingested",
                            json!({ "record_id": record.id }),
                        );
                    }
                    Err(err) => println!("Ingestion failed: {err}"),
                }
            }
            _ => {
                println!("Unknown knowledge command: {subcmd}");
                Self::print_knowledge_help();
            }
        }
        Ok(())
    }

    async fn perform_enriched_knowledge_search(
        &mut self,
        query_text: &str,
    ) -> Result<KnowledgeSearchOutcome> {
        let baseline = self
            .knowledge
            .search(KnowledgeQuery::new(query_text.to_string()));
        let new_records = match self.knowledge.enrich_from_web(query_text).await {
            Ok(records) => records.len(),
            Err(err) => {
                println!("Web enrichment failed: {err}");
                self.record_experience(
                    "knowledge",
                    "web_search.error",
                    json!({ "query": query_text, "error": err.to_string() }),
                );
                0
            }
        };
        let enriched = self
            .knowledge
            .search(KnowledgeQuery::new(query_text.to_string()));
        let snippets = if enriched.is_empty() { baseline } else { enriched };
        self.record_experience(
            "knowledge",
            "search.enriched",
            json!({
                "query": query_text,
                "results": snippets.len(),
                "new_records": new_records
            }),
        );
        Ok(KnowledgeSearchOutcome { snippets, new_records })
    }

    fn print_knowledge_help() {
        println!(
            "Knowledge commands:
  knowledge sync [limit]       - ingest latest experience events as records
  knowledge search <query>     - search local knowledge corpus
  knowledge ingest <src>|<title>|<body> - manually add a record"
        );
    }

    async fn handle_autonomy(&mut self, args: &str) -> Result<()> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            Self::print_autonomy_help();
            return Ok(());
        }
        let mut parts = trimmed.split_whitespace();
        let subcmd = parts.next().unwrap();
        let remainder = trimmed[subcmd.len()..].trim();
        match subcmd {
            "cycle" => {
                let signal = parse_autonomy_signal(remainder)?;
                let report = self.autonomy.run_cycle(signal).await?;
                println!(
                    "Autonomy cycle {:?}: {} directives, confidence {:.2}",
                    report.cycle_id,
                    report.verdict.directives.len(),
                    report.verdict.confidence
                );
                self.record_experience(
                    "autonomy",
                    "cycle.completed",
                    json!({
                        "cycle_id": report.cycle_id,
                        "directives": report.verdict.directives.len(),
                        "confidence": report.verdict.confidence,
                        "hypothesis": report.verdict.hypothesis.summary
                    }),
                );
            }
            "directive" => {
                let mut tokens = remainder.split_whitespace();
                let kind_token = tokens.next().ok_or_else(|| {
                    anyhow!("usage: autonomy directive <kind> <priority> <text...>")
                })?;
                let priority_token = tokens.next().ok_or_else(|| {
                    anyhow!("usage: autonomy directive <kind> <priority> <text...>")
                })?;
                let description = tokens.collect::<Vec<_>>().join(" ");
                if description.is_empty() {
                    println!("Directive description required.");
                    return Ok(());
                }
                let kind = parse_module_kind(kind_token)?;
                let priority = parse_directive_priority(priority_token)?;
                let directive =
                    self.autonomy
                        .issue_directive(kind.clone(), priority, description.clone());
                println!(
                    "Directive {} issued to {:?} ({:?})",
                    directive.id, kind, priority
                );
                self.record_experience(
                    "autonomy",
                    "directive.issued",
                    json!({
                        "directive_id": directive.id,
                        "priority": priority.to_string(),
                        "target": format!("{:?}", kind),
                        "instructions": directive.instructions
                    }),
                );
            }
            "metrics" => {
                let metrics = self.autonomy.metrics();
                println!(
                    "Autonomy metrics:\n  last_cycle: {}\n  directives: {}\n  avg_confidence: {:.2}\n  modules_active: {}\n  updated_at: {}",
                    metrics
                        .last_cycle
                        .clone()
                        .unwrap_or_else(|| "none".into()),
                    metrics.directives_issued,
                    metrics.avg_confidence,
                    metrics.modules_active,
                    metrics.updated_at
                );
            }
            _ => {
                println!("Unknown autonomy command: {subcmd}");
                Self::print_autonomy_help();
            }
        }
        Ok(())
    }

    fn print_autonomy_help() {
        println!(
            "Autonomy commands:
  autonomy cycle <narrative...> [--scope=global|domain:<name>|local:<name>] [--metric=key=value] [--tag=key=value]
  autonomy directive <kind> <priority> <description...>
  autonomy metrics"
        );
    }

    async fn handle_natural_language(&mut self, args: &str) -> Result<()> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            println!("Usage: natural <free-form command>");
            return Ok(());
        }
        let ir = match self.natural_language.generate_ir(trimmed).await {
            Ok(ir) => ir,
            Err(err) => {
                println!("Natural language parsing failed: {err}");
                self.record_experience(
                    "nlp",
                    "ir.error",
                    json!({ "error": err.to_string(), "input": trimmed }),
                );
                return Ok(());
            }
        };
        if let Err(err) = validate_ir(&ir) {
            println!("IR validation failed: {err}");
            self.record_experience(
                "nlp",
                "ir.invalid",
                json!({ "error": err.to_string(), "input": trimmed }),
            );
            return Ok(());
        }
        self.last_natural_ir = Some(ir.clone());
        let ambiguous =
            ir.ambiguous_targets.unwrap_or(false) || ir.intent.confidence.unwrap_or(1.0) < 0.7;
        if ambiguous {
            println!(
                "IR indicates low confidence or ambiguous targets. Route to human confirmation.\n{}",
                serde_json::to_string_pretty(&ir)?
            );
        } else {
            println!(
                "IR generated (intent={}, modules={:?}). Executing pipeline...",
                ir.intent
                    .intent_type
                    .clone()
                    .unwrap_or_else(|| "unknown".into()),
                ir.modules
            );
            if let Err(err) = self.process_ir(&ir, trimmed).await {
                println!("IR pipeline failed: {err}");
                self.record_experience(
                    "nlp",
                    "ir.pipeline.error",
                    json!({ "error": err.to_string(), "intent": ir.intent.intent_type }),
                );
            }
        }
        self.record_experience(
            "nlp",
            "ir.generated",
            json!({
                "intent": ir.intent.intent_type,
                "confidence": ir.intent.confidence,
                "modules": ir.modules,
                "ambiguous": ambiguous
            }),
        );
        Ok(())
    }

    async fn process_ir(&mut self, ir: &NaturalLanguageIr, user_text: &str) -> Result<()> {
        let modules: HashSet<String> = ir.modules.iter().map(|m| m.to_lowercase()).collect();
        let objective = derive_objective(ir, user_text);
        let plan_needed = modules.contains("planning")
            || ir
                .intent
                .intent_type
                .as_ref()
                .map(|s| s.eq_ignore_ascii_case("plan"))
                .unwrap_or(false);
        if plan_needed {
            self.handle_plan(&objective).await?;
            self.handle_schedule().await?;
        }
        if modules.contains("knowledge") {
            let query = if user_text.trim().is_empty() {
                objective.as_str()
            } else {
                user_text
            };
            let outcome = self.perform_enriched_knowledge_search(query).await?;
            if outcome.snippets.is_empty() {
                println!("지식 모듈이 '{query}'와 관련된 자료를 아직 찾지 못했습니다.");
            } else {
                println!(
                    "지식 모듈이 {count}개의 참고 자료를 제공합니다 (웹 신규 {new}건).",
                    count = outcome.snippets.len(),
                    new = outcome.new_records
                );
            }
        }
        if modules.contains("autonomy") {
            let auto_cmd = format!("cycle {}", objective);
            self.handle_autonomy(&auto_cmd).await?;
        }
        if modules.contains("actions") {
            match ExecutionArgs::from_ir(ir, user_text) {
                Ok(mut exec) => {
                    if let Err(err) = self.populate_action_context(&mut exec, ir, user_text).await {
                        println!("Skipping actions: {err}");
                        self.record_experience(
                            "nlp",
                            "ir.actions.skipped",
                            json!({ "reason": err.to_string() }),
                        );
                    } else {
                        let action_report = self.execute_request(exec).await?;
                        if !action_report.success {
                            let reason = action_report
                                .failure_reason
                                .as_deref()
                                .unwrap_or("unknown failure");
                            self.handle_action_failure(ir, user_text, reason).await?;
                        }
                    }
                }
                Err(err) => {
                    println!("Skipping actions: {err}");
                    self.record_experience(
                        "nlp",
                        "ir.actions.skipped",
                        json!({ "reason": err.to_string() }),
                    );
                }
            }
        }
        self.run_post_cycle_analysis(ir, &objective, user_text)
            .await?;
        Ok(())
    }

    async fn populate_action_context(
        &self,
        exec: &mut ExecutionArgs,
        ir: &NaturalLanguageIr,
        user_text: &str,
    ) -> Result<()> {
        if exec.domain != ActionDomain::Programming {
            return Ok(());
        }
        if !exec.attachments.is_empty() {
            return Ok(());
        }
        let objective = derive_objective(ir, user_text);
        let contexts = self
            .natural_language
            .generate_code_context(user_text, &objective)
            .await?;
        if contexts.is_empty() {
            bail!("code context generator returned no entries");
        }
        exec.attachments = contexts
            .into_iter()
            .map(|ctx| self.hydrate_code_attachment(ctx))
            .collect::<Result<_>>()?;
        let command_specs = match self
            .natural_language
            .generate_command_plan(user_text, &objective)
            .await
        {
            Ok(specs) => specs,
            Err(err) => {
                self.record_experience(
                    "nlp",
                    "command_plan.error",
                    json!({ "error": err.to_string(), "objective": objective }),
                );
                Vec::new()
            }
        };
        if !command_specs.is_empty() {
            for spec in command_specs.into_iter().take(MAX_COMMANDS_PER_REQUEST) {
                if let Some(attachment) = command_attachment_from_spec(spec) {
                    exec.attachments.push(attachment);
                }
            }
        }
        Ok(())
    }

    async fn handle_action_failure(
        &mut self,
        ir: &NaturalLanguageIr,
        user_text: &str,
        reason: &str,
    ) -> Result<()> {
        let objective = derive_objective(ir, user_text);
        self.record_experience(
            "actions",
            "auto_replan.triggered",
            json!({ "objective": objective, "reason": reason }),
        );

        let retry_objective = format!("{objective} (auto-replan)");
        match self.handle_plan(&retry_objective).await {
            Ok(_) => {
                if let Err(err) = self.handle_schedule().await {
                    println!("Auto schedule failed: {err}");
                    self.record_experience(
                        "actions",
                        "auto_replan.schedule_failed",
                        json!({ "error": err.to_string(), "objective": objective }),
                    );
                } else {
                    self.record_experience(
                        "actions",
                        "auto_replan.completed",
                        json!({ "objective": objective }),
                    );
                }
            }
            Err(err) => {
                println!("Auto replan failed: {err}");
                self.record_experience(
                    "actions",
                    "auto_replan.plan_failed",
                    json!({ "error": err.to_string(), "objective": objective }),
                );
            }
        }

        let directive = UpgradeDirective::new(
            format!("Self-improve after failure: {objective}"),
            "stability:auto",
            72,
        );
        match self.self_upgrade.execute(directive) {
            Ok(plan) => {
                self.record_experience(
                    "self_upgrade",
                    "auto_improvement.completed",
                    json!({
                        "actions": plan.actions.len(),
                        "status": format!("{:?}", plan.status),
                        "objective": objective
                    }),
                );
            }
            Err(err) => {
                println!("Self-improvement run failed: {err}");
                self.record_experience(
                    "self_upgrade",
                    "auto_improvement.failed",
                    json!({ "error": err.to_string(), "objective": objective }),
                );
            }
        }
        Ok(())
    }

    async fn run_post_cycle_analysis(
        &mut self,
        ir: &NaturalLanguageIr,
        objective: &str,
        user_text: &str,
    ) -> Result<()> {
        let observation_text = format!(
            "Objective: {objective}\nModules: {:?}\nUser Input: {user_input}",
            ir.modules,
            user_input = user_text
        );
        let observation = SelfObservation::new(observation_text.clone(), 0.6);
        let reflection = match self
            .metacognition
            .reflect(observation, ReflectionMethod::StructuredAnalysis)
            .await
        {
            Ok(summary) => summary,
            Err(err) => {
                println!("Metacognition reflection failed: {err}");
                self.record_experience(
                    "metacognition",
                    "ir.pipeline.reflection_failed",
                    json!({ "error": err.to_string(), "objective": objective }),
                );
                return Ok(());
            }
        };
        self.record_experience(
            "metacognition",
            "ir.pipeline.reflection",
            json!({
                "objective": objective,
                "summary": reflection.summary,
                "resilience": reflection.resilience,
                "diagnostics": reflection
                    .diagnostics
                    .iter()
                    .take(4)
                    .map(|diag| json!({
                        "verb": format!("{:?}", diag.verb),
                        "outcome": format!("{:?}", diag.outcome),
                        "impact": diag.impact
                    }))
                    .collect::<Vec<_>>()
            }),
        );

        let sim_report = match self
            .simulation
            .run_advanced(SimulationMethod::HighFidelity, 3)
            .await
        {
            Ok(report) => report,
            Err(err) => {
                println!("Simulation failed: {err}");
                self.record_experience(
                    "simulation",
                    "ir.pipeline.simulation_failed",
                    json!({ "error": err.to_string() }),
                );
                return Ok(());
            }
        };
        self.record_experience(
            "simulation",
            "ir.pipeline.simulation",
            json!({
                "scenarios": sim_report.scenario_count,
                "insights": sim_report.insights
            }),
        );
        let high_load = sim_report
            .insights
            .iter()
            .filter(|insight| insight.finding == "high_load")
            .count();
        let directive_prompt = format!(
            "Assess objective '{objective}'. Reflection summary: {} (resilience {:.2}).",
            reflection.summary, reflection.resilience
        );
        let directive = ReasoningDirective::new(directive_prompt, ReasoningPriority::Medium);
        let mut signals = Vec::new();
        signals.push(SignalPacket::new(
            "reflection.summary",
            json!({
                "summary": reflection.summary,
                "resilience": reflection.resilience,
                "modules": ir.modules,
                "objective": objective
            }),
        ));
        signals.push(SignalPacket::new(
            "simulation.report",
            json!({
                "scenario_count": sim_report.scenario_count,
                "insights": sim_report.insights
            }),
        ));
        if let Some(params) = &ir.parameters {
            signals.push(SignalPacket::new("ir.parameters", params.clone()));
        }
        let verdict = match self.reasoning.reason(directive, signals).await {
            Ok(v) => v,
            Err(err) => {
                println!("Reasoning failed: {err}");
                self.record_experience(
                    "reasoning",
                    "ir.pipeline.verdict_failed",
                    json!({ "error": err.to_string() }),
                );
                return Ok(());
            }
        };
        self.record_experience(
            "reasoning",
            "ir.pipeline.verdict",
            json!({
                "directive_id": verdict.directive_id,
                "notes": verdict.notes,
                "confidence": verdict.hypothesis.as_ref().map(|h| h.confidence)
            }),
        );
        let verdict_confidence = verdict
            .hypothesis
            .as_ref()
            .map(|h| h.confidence)
            .unwrap_or(0.0);
        let world_summary =
            match self.apply_world_update(sim_report.scenario_count, high_load, verdict_confidence)
            {
                Ok(summary) => summary,
                Err(err) => {
                    println!("World update failed: {err}");
                    self.record_experience(
                        "world",
                        "ir.pipeline.update_failed",
                        json!({ "error": err.to_string() }),
                    );
                    return Ok(());
                }
            };
        if let Err(err) = self
            .generate_report_output(
                ir,
                objective,
                user_text,
                &reflection,
                &sim_report,
                &verdict,
                &world_summary,
            )
            .await
        {
            println!("Report generation failed: {err}");
            self.record_experience("nlp", "report.failed", json!({ "error": err.to_string() }));
        }
        Ok(())
    }

    fn apply_world_update(
        &mut self,
        scenarios: usize,
        high_load: usize,
        verdict_confidence: f32,
    ) -> Result<WorldUpdateSummary> {
        let mut region_metrics = IndexMap::new();
        region_metrics.insert(
            "aggregate".into(),
            json!({
                "scenario_count": scenarios,
                "high_load": high_load,
                "verdict_confidence": verdict_confidence
            }),
        );
        let job = AssimilationJob {
            batch_id: Uuid::new_v4(),
            region_metrics,
        };
        let state = self.world.ingest(job)?;
        self.record_experience(
            "world",
            "update.ingested",
            json!({
                "regions": state.regions.len(),
                "anomalies": state.anomalies.len(),
                "high_load": high_load
            }),
        );
        Ok(WorldUpdateSummary {
            regions: state.regions.len(),
            anomalies: state.anomalies.len(),
            high_load,
        })
    }

    async fn generate_report_output(
        &self,
        ir: &NaturalLanguageIr,
        objective: &str,
        user_text: &str,
        reflection: &ReflectionDigest,
        sim_report: &SimulationReport,
        verdict: &Verdict,
        world_summary: &WorldUpdateSummary,
    ) -> Result<()> {
        let insight_summary: Vec<String> = sim_report
            .insights
            .iter()
            .map(|insight| format!("{}={}", insight.label, insight.finding))
            .collect();
        let verdict_confidence = verdict
            .hypothesis
            .as_ref()
            .map(|h| h.confidence)
            .unwrap_or(0.0);
        let verdict_summary = verdict
            .hypothesis
            .as_ref()
            .map(|h| h.summary.clone())
            .unwrap_or_else(|| "No strong hypothesis".into());
        let context = ReportContext {
            objective,
            modules: &ir.modules,
            user_input: user_text,
            reflection_summary: &reflection.summary,
            reflection_resilience: reflection.resilience,
            scenario_count: sim_report.scenario_count,
            high_load: world_summary.high_load,
            insight_summary: &insight_summary,
            verdict_notes: &verdict.notes,
            verdict_summary: &verdict_summary,
            verdict_confidence,
            world_regions: world_summary.regions,
            world_anomalies: world_summary.anomalies,
        };
        let summary = match self.natural_language.generate_report(&context).await {
            Ok(text) => text,
            Err(err) => {
                println!("Natural language report generation failed: {err}");
                return Err(err);
            }
        };
        println!("\n=== Natural Language Report ===\n{}\n", summary.trim());
        self.record_experience(
            "nlp",
            "report.generated",
            json!({
                "objective": objective,
                "summary": summary,
                "verdict_confidence": verdict_confidence
            }),
        );
        Ok(())
    }

    fn hydrate_code_attachment(&self, ctx: CodeContextSpec) -> Result<PayloadAttachment> {
        let CodeContextSpec {
            path,
            original,
            proposed,
            description,
            create,
        } = ctx;
        let mut resolved_original = original.unwrap_or_default();
        if resolved_original.is_empty() {
            if let Some(snapshot) = load_file_snapshot(Path::new(&path))? {
                resolved_original = snapshot;
            }
        }
        let mut content = serde_json::Map::new();
        content.insert("path".into(), Value::String(path.clone()));
        content.insert("original".into(), Value::String(resolved_original));
        content.insert("proposed".into(), Value::String(proposed));
        if let Some(desc) = description {
            if !desc.is_empty() {
                content.insert("description".into(), Value::String(desc));
            }
        }
        let should_create = create.or_else(|| {
            if !Path::new(&path).exists() {
                Some(true)
            } else {
                None
            }
        });
        if let Some(flag) = should_create {
            content.insert("create".into(), Value::Bool(flag));
        }
        Ok(PayloadAttachment {
            label: "code_context".into(),
            content_type: "application/json".into(),
            content: Value::Object(content),
        })
    }
}

fn command_attachment_from_spec(spec: CommandPlanSpec) -> Option<PayloadAttachment> {
    let command = spec.command.trim();
    if command.is_empty() {
        return None;
    }
    let mut content = serde_json::Map::new();
    content.insert("command".into(), Value::String(command.to_string()));
    if !spec.args.is_empty() {
        content.insert(
            "args".into(),
            Value::Array(spec.args.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(cwd) = spec.cwd.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        content.insert("cwd".into(), Value::String(cwd));
    }
    if let Some(stdin) = spec
        .stdin
        .and_then(|v| if v.is_empty() { None } else { Some(v) })
    {
        content.insert("stdin".into(), Value::String(stdin));
    }
    let timeout_ms = spec
        .timeout_ms
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_COMMAND_TIMEOUT_MS)
        .min(MAX_COMMAND_TIMEOUT_MS);
    content.insert("timeout_ms".into(), Value::Number(Number::from(timeout_ms)));
    if let Some(description) = spec.description.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        content.insert("description".into(), Value::String(description));
    }
    Some(PayloadAttachment {
        label: "command_plan".into(),
        content_type: "application/json".into(),
        content: Value::Object(content),
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct IntentSpec {
    #[serde(rename = "type")]
    intent_type: Option<String>,
    label: Option<String>,
    confidence: Option<f32>,
    parameters: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NaturalLanguageIr {
    intent: IntentSpec,
    objective: Option<String>,
    modules: Vec<String>,
    ambiguous_targets: Option<bool>,
    parameters: Option<Value>,
    metadata: Option<Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GenerateRequest<'a> {
    prompt: &'a str,
    max_tokens: usize,
    temperature: f32,
    top_p: f32,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CodeContextSpec {
    path: String,
    #[serde(default)]
    original: Option<String>,
    proposed: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    create: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct CommandPlanSpec {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    stdin: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    description: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut orchestrator = EntireSystemOrchestrator::bootstrap()
        .await
        .context("failed to bootstrap orchestrator")?;
    orchestrator.run().await
}

struct PolicyLedger {
    path: PathBuf,
    content: String,
}

impl PolicyLedger {
    fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let content = fs::read_to_string(&path)
            .with_context(|| format!("loading policies from {:?}", path))?;
        Ok(Self { path, content })
    }
}

#[derive(Debug)]
struct ExecutionArgs {
    domain: ActionDomain,
    intent: ActionIntent,
    summary: String,
    narrative: String,
    priority: ActionPriority,
    safety: ActionSafetyClass,
    tags: Vec<String>,
    attachments: Vec<PayloadAttachment>,
}

struct ActionExecutionReport {
    success: bool,
    failure_reason: Option<String>,
}

struct KnowledgeSearchOutcome {
    snippets: Vec<KnowledgeSnippet>,
    new_records: usize,
}

impl ActionExecutionReport {
    fn success() -> Self {
        Self {
            success: true,
            failure_reason: None,
        }
    }

    fn failure(reason: impl Into<String>) -> Self {
        Self {
            success: false,
            failure_reason: Some(reason.into()),
        }
    }
}

impl ExecutionArgs {
    fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bail!("missing arguments");
        }
        let mut segments = trimmed.splitn(2, '|');
        let command_segment = segments.next().unwrap().trim();
        let narrative_segment = segments.next().map(|s| s.trim()).filter(|s| !s.is_empty());
        let mut tokens = command_segment.split_whitespace();
        let domain_token = tokens
            .next()
            .ok_or_else(|| anyhow!("missing domain (e.g., programming)"))?;
        let intent_token = tokens
            .next()
            .ok_or_else(|| anyhow!("missing intent (e.g., program)"))?;
        let mut summary_tokens = Vec::new();
        let mut priority = ActionPriority::Normal;
        let mut safety = ActionSafetyClass::Green;
        let mut tags = Vec::new();
        for token in tokens {
            if let Some(value) = token.strip_prefix("--priority=") {
                priority = parse_priority(value)?;
                continue;
            }
            if let Some(value) = token.strip_prefix("--safety=") {
                safety = parse_safety(value)?;
                continue;
            }
            if let Some(value) = token.strip_prefix("--tag=") {
                if value.is_empty() {
                    bail!("--tag value cannot be empty");
                }
                tags.push(value.to_string());
                continue;
            }
            summary_tokens.push(token);
        }
        if summary_tokens.is_empty() {
            bail!("summary is required after intent");
        }
        let summary = summary_tokens.join(" ");
        let narrative = narrative_segment
            .map(|s| s.to_string())
            .unwrap_or_else(|| summary.clone());
        Ok(Self {
            domain: parse_domain(domain_token)?,
            intent: parse_intent(intent_token)?,
            summary,
            narrative,
            priority,
            safety,
            tags,
            attachments: Vec::new(),
        })
    }

    const fn usage() -> &'static str {
        "execute <domain> <intent> <summary...> | <details> [--priority=low|normal|high|critical] [--safety=green|yellow|orange|red] [--tag=label]"
    }

    fn from_ir(ir: &NaturalLanguageIr, fallback: &str) -> Result<Self> {
        let domain = infer_domain_from_ir(ir);
        let intent = infer_action_intent(ir);
        let summary = derive_objective(ir, fallback);
        let narrative = field_from_value(ir.metadata.as_ref(), "narrative")
            .or_else(|| field_from_value(ir.parameters.as_ref(), "narrative"))
            .unwrap_or_else(|| summary.clone());
        let priority = field_from_value(ir.metadata.as_ref(), "priority")
            .or_else(|| field_from_value(ir.parameters.as_ref(), "priority"))
            .map(|value| parse_priority(&value))
            .transpose()?
            .unwrap_or(ActionPriority::Normal);
        let safety = field_from_value(ir.metadata.as_ref(), "safety")
            .or_else(|| field_from_value(ir.parameters.as_ref(), "safety"))
            .map(|value| parse_safety(&value))
            .transpose()?
            .unwrap_or(ActionSafetyClass::Green);
        let mut tags = array_field(ir.metadata.as_ref(), "tags");
        if tags.is_empty() {
            if let Some(label) = &ir.intent.label {
                tags.push(label.clone());
            }
        }
        Ok(Self {
            domain,
            intent,
            summary,
            narrative,
            priority,
            safety,
            tags,
            attachments: Vec::new(),
        })
    }
}

fn parse_domain(token: &str) -> Result<ActionDomain> {
    let key = token.to_lowercase();
    Ok(match key.as_str() {
        "economic" => ActionDomain::Economic,
        "security" => ActionDomain::Security,
        "financial" => ActionDomain::Financial,
        "infrastructure" => ActionDomain::Infrastructure,
        "ai" | "ai_research" | "airesearch" => ActionDomain::AiResearch,
        "network" => ActionDomain::Network,
        "manufacturing" => ActionDomain::Manufacturing,
        "medical" => ActionDomain::Medical,
        "simulation" => ActionDomain::Simulation,
        "research" => ActionDomain::Research,
        "programming" => ActionDomain::Programming,
        "selftraining" | "self_training" => ActionDomain::SelfTraining,
        s if s.starts_with("custom:") => ActionDomain::Custom(token["custom:".len()..].to_string()),
        _ => bail!("unknown domain '{token}'"),
    })
}

fn parse_intent(token: &str) -> Result<ActionIntent> {
    Ok(match token.to_lowercase().as_str() {
        "observe" => ActionIntent::Observe,
        "simulate" => ActionIntent::Simulate,
        "optimize" => ActionIntent::Optimize,
        "execute" => ActionIntent::Execute,
        "remediate" => ActionIntent::Remediate,
        "coordinate" => ActionIntent::Coordinate,
        "program" => ActionIntent::Program,
        "learn" => ActionIntent::Learn,
        "audit" => ActionIntent::Audit,
        other => bail!("unknown intent '{other}'"),
    })
}

fn parse_priority(token: &str) -> Result<ActionPriority> {
    Ok(match token.to_lowercase().as_str() {
        "low" => ActionPriority::Low,
        "normal" | "default" => ActionPriority::Normal,
        "high" => ActionPriority::High,
        "critical" => ActionPriority::Critical,
        other => bail!("unknown priority '{other}'"),
    })
}

fn parse_safety(token: &str) -> Result<ActionSafetyClass> {
    Ok(match token.to_lowercase().as_str() {
        "green" => ActionSafetyClass::Green,
        "yellow" => ActionSafetyClass::Yellow,
        "orange" => ActionSafetyClass::Orange,
        "red" => ActionSafetyClass::Red,
        other => bail!("unknown safety class '{other}'"),
    })
}

fn parse_autonomy_signal(input: &str) -> Result<AutonomySignal> {
    let mut scope = SignalScope::Global;
    let mut narrative_tokens = Vec::new();
    let mut metrics = Vec::new();
    let mut tags = Vec::new();
    for token in input.split_whitespace() {
        if let Some(value) = token.strip_prefix("--scope=") {
            scope = parse_signal_scope(value)?;
        } else if let Some(value) = token.strip_prefix("--metric=") {
            let (key, val) = value
                .split_once('=')
                .ok_or_else(|| anyhow!("metric must be key=value"))?;
            let parsed: f64 = val
                .parse()
                .with_context(|| format!("invalid metric value '{val}'"))?;
            metrics.push((key.to_string(), parsed));
        } else if let Some(value) = token.strip_prefix("--tag=") {
            let (key, val) = value
                .split_once('=')
                .ok_or_else(|| anyhow!("tag must be key=value"))?;
            tags.push((key.to_string(), val.to_string()));
        } else if !token.is_empty() {
            narrative_tokens.push(token);
        }
    }
    let narrative = if narrative_tokens.is_empty() {
        "autonomy cycle".to_string()
    } else {
        narrative_tokens.join(" ")
    };
    let mut signal = AutonomySignal::new(scope, narrative);
    for (key, val) in metrics {
        signal = signal.with_metric(key, val);
    }
    for (key, val) in tags {
        signal = signal.with_tag(key, val);
    }
    Ok(signal)
}

fn parse_signal_scope(value: &str) -> Result<SignalScope> {
    let normalized = value.to_lowercase();
    if normalized == "global" {
        Ok(SignalScope::Global)
    } else if let Some(domain) = value.strip_prefix("domain:") {
        Ok(SignalScope::Domain(domain.to_string()))
    } else if let Some(local) = value.strip_prefix("local:") {
        Ok(SignalScope::Local(local.to_string()))
    } else {
        bail!("invalid scope '{value}'")
    }
}

fn parse_module_kind(token: &str) -> Result<ModuleKind> {
    Ok(match token.to_lowercase().as_str() {
        "planner" => ModuleKind::Planner,
        "executor" => ModuleKind::Executor,
        "sensor" => ModuleKind::Sensor,
        "memory" => ModuleKind::Memory,
        "selfhealing" | "self_healing" | "self-healing" => ModuleKind::SelfHealing,
        other if other.starts_with("custom:") => {
            ModuleKind::Custom(token["custom:".len()..].to_string())
        }
        other => bail!("unknown module kind '{other}'"),
    })
}

fn parse_directive_priority(token: &str) -> Result<DirectivePriority> {
    Ok(match token.to_lowercase().as_str() {
        "routine" => DirectivePriority::Routine,
        "elevated" => DirectivePriority::Elevated,
        "critical" => DirectivePriority::Critical,
        other => bail!("unknown directive priority '{other}'"),
    })
}

struct NaturalLanguageClient {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    jwt: Option<String>,
}

impl NaturalLanguageClient {
    async fn new_from_env() -> Result<Self> {
        let endpoint = env::var("LANGMODEL_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:9000/generate".into());
        ensure_langmodel_server(&endpoint).await?;
        let api_key = env::var("LANGMODEL_API_KEY").ok();
        let jwt = env::var("LANGMODEL_JWT").ok();
        let client = Client::builder().build()?;
        Ok(Self {
            client,
            endpoint,
            api_key,
            jwt,
        })
    }

    async fn generate_ir(&self, user_text: &str) -> Result<NaturalLanguageIr> {
        let prompt = build_ir_prompt(user_text);
        let payload = GenerateRequest {
            prompt: &prompt,
            max_tokens: 512,
            temperature: 0.05,
            top_p: 0.9,
        };
        let mut req = self.client.post(&self.endpoint).json(&payload);
        if let Some(key) = &self.api_key {
            req = req.header("x-api-key", key);
        }
        if let Some(token) = &self.jwt {
            req = req.bearer_auth(token);
        }
        let response = req.send().await?.error_for_status()?;
        let body: GenerateResponse = response.json().await?;
        parse_ir_from_text(&body.text).context("unable to parse IR JSON from LLM output")
    }

    async fn generate_report(&self, context: &ReportContext<'_>) -> Result<String> {
        let prompt = build_report_prompt(context);
        let payload = GenerateRequest {
            prompt: &prompt,
            max_tokens: 512,
            temperature: 0.3,
            top_p: 0.9,
        };
        let mut req = self.client.post(&self.endpoint).json(&payload);
        if let Some(key) = &self.api_key {
            req = req.header("x-api-key", key);
        }
        if let Some(token) = &self.jwt {
            req = req.bearer_auth(token);
        }
        let response = req.send().await?.error_for_status()?;
        let body: GenerateResponse = response.json().await?;
        Ok(body.text)
    }

    async fn generate_code_context(
        &self,
        user_text: &str,
        objective: &str,
    ) -> Result<Vec<CodeContextSpec>> {
        let prompt = build_code_context_prompt(user_text, objective);
        let payload = GenerateRequest {
            prompt: &prompt,
            max_tokens: 768,
            temperature: 0.15,
            top_p: 0.9,
        };
        let mut req = self.client.post(&self.endpoint).json(&payload);
        if let Some(key) = &self.api_key {
            req = req.header("x-api-key", key);
        }
        if let Some(token) = &self.jwt {
            req = req.bearer_auth(token);
        }
        let response = req.send().await?.error_for_status()?;
        let body: GenerateResponse = response.json().await?;
        parse_code_context_list(&body.text)
    }

    async fn generate_command_plan(
        &self,
        user_text: &str,
        objective: &str,
    ) -> Result<Vec<CommandPlanSpec>> {
        let prompt = build_command_plan_prompt(user_text, objective);
        let payload = GenerateRequest {
            prompt: &prompt,
            max_tokens: 384,
            temperature: 0.1,
            top_p: 0.85,
        };
        let mut req = self.client.post(&self.endpoint).json(&payload);
        if let Some(key) = &self.api_key {
            req = req.header("x-api-key", key);
        }
        if let Some(token) = &self.jwt {
            req = req.bearer_auth(token);
        }
        let response = req.send().await?.error_for_status()?;
        let body: GenerateResponse = response.json().await?;
        parse_command_plan_list(&body.text)
    }
}

fn validate_ir(ir: &NaturalLanguageIr) -> Result<()> {
    if ir.modules.is_empty() {
        bail!("IR response missing modules");
    }
    if ir.intent.intent_type.is_none() {
        bail!("IR response missing intent.type");
    }
    if ir.intent.confidence.map(|c| c.is_nan()).unwrap_or(false) {
        bail!("IR confidence is invalid");
    }
    Ok(())
}

fn parse_ir_from_text(text: &str) -> Result<NaturalLanguageIr> {
    if let Ok(ir) = serde_json::from_str::<NaturalLanguageIr>(text) {
        return Ok(ir);
    }
    if let Some(slice) = extract_json_block(text) {
        Ok(serde_json::from_str(slice)?)
    } else {
        bail!("LLM output did not contain JSON block");
    }
}

fn extract_json_block(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end >= start {
        Some(&text[start..=end])
    } else {
        None
    }
}

fn extract_json_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if end >= start {
        Some(&text[start..=end])
    } else {
        None
    }
}

fn parse_code_context_list(text: &str) -> Result<Vec<CodeContextSpec>> {
    if let Ok(list) = serde_json::from_str::<Vec<CodeContextSpec>>(text) {
        return Ok(list);
    }
    if let Some(slice) = extract_json_array(text) {
        if let Ok(list) = serde_json::from_str::<Vec<CodeContextSpec>>(slice) {
            return Ok(list);
        }
    }
    if let Some(slice) = extract_json_block(text) {
        let ctx: CodeContextSpec = serde_json::from_str(slice)?;
        return Ok(vec![ctx]);
    }
    bail!("LLM output did not contain JSON block");
}

fn parse_command_plan_list(text: &str) -> Result<Vec<CommandPlanSpec>> {
    if let Ok(list) = serde_json::from_str::<Vec<CommandPlanSpec>>(text) {
        return Ok(list);
    }
    if let Some(slice) = extract_json_array(text) {
        if let Ok(list) = serde_json::from_str::<Vec<CommandPlanSpec>>(slice) {
            return Ok(list);
        }
    }
    if let Some(slice) = extract_json_block(text) {
        let spec: CommandPlanSpec = serde_json::from_str(slice)?;
        return Ok(vec![spec]);
    }
    bail!("LLM output did not contain JSON block");
}

fn load_file_snapshot(path: &Path) -> io::Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => {
            let mut text = String::from_utf8_lossy(&bytes).into_owned();
            if text.len() > MAX_FILE_SNAPSHOT_BYTES {
                text.truncate(MAX_FILE_SNAPSHOT_BYTES);
            }
            Ok(Some(text))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn build_ir_prompt(user_command: &str) -> String {
    format!(
        r#"You are an AGI orchestration router. Convert USER_INPUT into a strict JSON object with this schema:
{{
  "intent": {{
     "type": "Plan|Query|Action|Maintenance|Audit",
     "label": "<short label>",
     "confidence": <0.0-1.0>,
     "parameters": {{ ... optional ... }}
  }},
  "objective": "<optional objective text>",
  "modules": ["planning","autonomy","actions","knowledge","world","simulation","learning"],
  "ambiguous_targets": <true|false>,
  "parameters": {{ ... optional structured fields ... }},
  "metadata": {{ ... optional ... }}
}}
Rules:
- ALWAYS return pure JSON (no prose, no markdown).
- Include modules relevant to the task.
- Set ambiguous_targets=true when the request is vague or missing entities.
- confidence=0.0-1.0. Use <0.7 if uncertain.

Example 1:
Input: "서버 정리해줘"
Output:
{{
  "intent": {{
    "type": "Maintenance",
    "label": "cleanup_servers",
    "confidence": 0.6
  }},
  "modules": ["planning","actions"],
  "ambiguous_targets": true,
  "objective": "정체불명의 서버 정리 요청"
}}

Example 2:
Input: "지난 주 장애 로그 중 오류 코드 503이 몇 번 발생했지?"
Output:
{{
  "intent": {{
    "type": "Query",
    "label": "count_logs",
    "confidence": 0.94,
    "parameters": {{
      "error_code": 503,
      "time_range": "2025-11-14/2025-11-20"
    }}
  }},
  "modules": ["knowledge","learning"],
  "ambiguous_targets": false,
  "objective": "장애 로그 503 발생 횟수 조회"
}}

USER_INPUT: {user_input}
Return JSON:"#,
        user_input = user_command
    )
}

fn build_report_prompt(context: &ReportContext<'_>) -> String {
    let modules = if context.modules.is_empty() {
        "none".into()
    } else {
        context.modules.join(", ")
    };
    let insights = if context.insight_summary.is_empty() {
        "none".into()
    } else {
        context.insight_summary.join(", ")
    };
    format!(
"You are an AGI status reporter. Summarize the latest AGI cycle in clear Korean prose (3-4 paragraphs max) using this data:
- Objective: {objective}
- Modules activated: {modules}
- User input: {user_input}
- Reflection summary: {reflection_summary} (resilience={reflection_resilience:.2})
- Simulation: scenarios={scenario_count}, high_load={high_load}, insights={insights}
- Reasoning verdict: summary={verdict_summary}, notes={verdict_notes}, confidence={verdict_confidence:.2}
- World update: regions={world_regions}, anomalies={world_anomalies}

Guidelines:
1. Provide a concise narrative of what was attempted, what risks were found, and what the world model now believes.
2. Highlight any warnings (low confidence or anomalies).
3. End with next recommended focus or monitoring advice.
4. Avoid markdown or bullet characters; output pure text paragraphs.
",
        objective = context.objective,
        modules = modules,
        user_input = context.user_input,
        reflection_summary = context.reflection_summary,
        reflection_resilience = context.reflection_resilience,
        scenario_count = context.scenario_count,
        high_load = context.high_load,
        insights = insights,
        verdict_summary = context.verdict_summary,
        verdict_notes = context.verdict_notes,
        verdict_confidence = context.verdict_confidence,
        world_regions = context.world_regions,
        world_anomalies = context.world_anomalies,
    )
}

fn build_code_context_prompt(user_command: &str, objective: &str) -> String {
    format!(
        r#"You are a programming code-context generator. Produce a JSON array of objects, each with:
- "path": relative file path (string). Use "generated/<name>" for brand new files.
- "proposed": the full updated file contents (string, exact text to write).
Optional fields:
- "original": expected current contents (string) if you already know them.
- "description": short summary of the change (<=120 chars).
- "create": true if this is a brand new file.

Rules:
1. Include at least one object.
2. "proposed" must be pure code/text (no explanations or fences).
3. Ensure formatting/indentation is production-quality.
4. Return JSON only (no prose before or after).

USER_INPUT: {user_input}
OBJECTIVE: {objective}
Return JSON array:"#,
        user_input = user_command,
        objective = objective
    )
}

fn build_command_plan_prompt(user_command: &str, objective: &str) -> String {
    format!(
        r#"You are a software command planner. Produce a JSON array where each object describes one shell command with fields:
- "command": base executable (e.g., "cargo", "npm", "python").
- "args": array of arguments (strings). Use [] if none.
- "cwd": optional working directory relative to repo root (omit or empty for default).
- "stdin": optional text piped to standard input.
- "timeout_ms": integer timeout in milliseconds (1000-300000).
- "description": short explanation of why the command is needed (<=120 chars).

Rules:
1. Include at most {max_commands} commands prioritized for build/test/validation.
2. Use production-safe commands only (builds, linters, tests, package install).
3. Prefer deterministic commands (e.g., "cargo test", "npm run build").
4. Return JSON only (no commentary or markdown).

USER_INPUT: {user_input}
OBJECTIVE: {objective}
Return JSON array:"#,
        max_commands = MAX_COMMANDS_PER_REQUEST,
        user_input = user_command,
        objective = objective
    )
}

fn derive_objective(ir: &NaturalLanguageIr, fallback: &str) -> String {
    ir.objective
        .clone()
        .or_else(|| field_from_value(ir.parameters.as_ref(), "objective"))
        .or_else(|| field_from_value(ir.metadata.as_ref(), "objective"))
        .or_else(|| ir.intent.label.clone())
        .unwrap_or_else(|| fallback.to_string())
}

fn field_from_value(container: Option<&Value>, key: &str) -> Option<String> {
    match container {
        Some(Value::Object(map)) => map.get(key).and_then(|v| v.as_str().map(|s| s.to_string())),
        _ => None,
    }
}

fn array_field(container: Option<&Value>, key: &str) -> Vec<String> {
    match container {
        Some(Value::Object(map)) => map
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn infer_domain_from_ir(ir: &NaturalLanguageIr) -> ActionDomain {
    if let Some(domain) = field_from_value(ir.metadata.as_ref(), "domain")
        .or_else(|| field_from_value(ir.parameters.as_ref(), "domain"))
        .or_else(|| field_from_value(ir.intent.parameters.as_ref(), "domain"))
    {
        parse_domain(&domain).unwrap_or(ActionDomain::Programming)
    } else {
        ActionDomain::Programming
    }
}

fn infer_action_intent(ir: &NaturalLanguageIr) -> ActionIntent {
    if let Some(intent_type) = &ir.intent.intent_type {
        match intent_type.to_lowercase().as_str() {
            "observe" | "query" => ActionIntent::Observe,
            "simulate" => ActionIntent::Simulate,
            "optimize" => ActionIntent::Optimize,
            "execute" | "action" => ActionIntent::Execute,
            "remediate" | "maintenance" => ActionIntent::Remediate,
            "coordinate" | "plan" => ActionIntent::Coordinate,
            "program" => ActionIntent::Program,
            "learn" => ActionIntent::Learn,
            "audit" => ActionIntent::Audit,
            other => {
                if other.contains("plan") {
                    ActionIntent::Coordinate
                } else if other.contains("query") {
                    ActionIntent::Observe
                } else {
                    ActionIntent::Execute
                }
            }
        }
    } else {
        ActionIntent::Execute
    }
}

#[derive(Debug)]
struct WorldUpdateSummary {
    regions: usize,
    anomalies: usize,
    high_load: usize,
}

#[derive(Debug)]
struct ReportContext<'a> {
    objective: &'a str,
    modules: &'a [String],
    user_input: &'a str,
    reflection_summary: &'a str,
    reflection_resilience: f32,
    scenario_count: usize,
    high_load: usize,
    insight_summary: &'a [String],
    verdict_notes: &'a str,
    verdict_summary: &'a str,
    verdict_confidence: f32,
    world_regions: usize,
    world_anomalies: usize,
}

async fn ensure_langmodel_server(endpoint: &str) -> Result<()> {
    if probe_langmodel(endpoint).await.is_ok() {
        return Ok(());
    }
    let autostart = env::var("LANGMODEL_AUTOSTART").unwrap_or_else(|_| "1".into());
    if autostart == "0" {
        bail!("Langmodel server unreachable at {endpoint} and autostart disabled");
    }
    let script = env::var("LANGMODEL_AUTOSTART_SCRIPT")
        .unwrap_or_else(|_| "ZAPPY--M/langmodel/server.py".into());
    let python = env::var("LANGMODEL_PYTHON").unwrap_or_else(|_| "python".into());
    println!("Langmodel server not reachable. Launching via `{python} {script}`...");
    StdCommand::new(python)
        .arg(&script)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn langmodel server process")?;
    for attempt in 0..10 {
        sleep(Duration::from_millis(500)).await;
        if probe_langmodel(endpoint).await.is_ok() {
            println!("Langmodel server is ready at {endpoint} (attempt {attempt}).");
            return Ok(());
        }
    }
    bail!("langmodel server did not become ready at {endpoint}")
}

async fn probe_langmodel(endpoint: &str) -> Result<()> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let payload = json!({
        "prompt": "system check",
        "max_tokens": 8,
        "temperature": 0.0,
        "top_p": 0.8
    });
    let response = client.post(endpoint).json(&payload).send().await?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("probe status {}", response.status()))
    }
}
