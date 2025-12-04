use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Local, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use shared_event_bus::{EventPublisher, EventRecord, FileEventPublisher};
use shared_logging::{JsonLogger, LogLevel, LogRecord};
use tokio::runtime::Runtime;
use uuid::Uuid;
use zappy_learning::device_manager::{AllocationPlan, DeviceManager, DevicePreference};

#[derive(Parser, Debug)]
#[command(name = "trn", version, about = "Tier-10 AGI training orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launches a new training job.
    Run(RunArgs),
    /// Lists most recent jobs.
    List {
        /// Number of entries to display.
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long, default_value = "ZAPPY--M/learning/logs/jobs/index.jsonl")]
        manifest: PathBuf,
    },
    /// Shows status for a given job id.
    Status {
        job_id: String,
        #[arg(long, default_value = "ZAPPY--M/learning/logs/jobs/index.jsonl")]
        manifest: PathBuf,
    },
    /// Resumes training from an existing checkpoint metadata.
    Resume {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        devices: Option<String>,
        #[arg(long, default_value = "ZAPPY--M/learning/logs")]
        log_dir: PathBuf,
        #[arg(long, default_value = "ZAPPY--M/learning/logs/jobs/index.jsonl")]
        manifest: PathBuf,
        #[arg(long)]
        event_log: Option<PathBuf>,
    },
}

#[derive(Parser, Debug)]
struct RunArgs {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    weights: Option<PathBuf>,
    #[arg(long, default_value = "prod")]
    profile: String,
    #[arg(long)]
    devices: Option<String>,
    #[arg(long)]
    cpu_only: bool,
    #[arg(long, default_value_t = 0)]
    device_count: usize,
    #[arg(long, default_value = "ZAPPY--M/learning/logs")]
    log_dir: PathBuf,
    #[arg(long, default_value = "ZAPPY--M/learning/logs/jobs/index.jsonl")]
    manifest: PathBuf,
    #[arg(long)]
    event_log: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JobManifestEntry {
    job_id: String,
    submitted_at: DateTime<Utc>,
    config: PathBuf,
    weights: Option<PathBuf>,
    profile: String,
    devices: Vec<String>,
    log_path: PathBuf,
    status: String,
}

impl JobManifestEntry {
    fn new(
        config: PathBuf,
        weights: Option<PathBuf>,
        profile: String,
        plan: &AllocationPlan,
        log_path: PathBuf,
    ) -> Self {
        Self {
            job_id: format!("job-{}", Uuid::new_v4()),
            submitted_at: Utc::now(),
            config,
            weights,
            profile,
            devices: plan.devices().iter().map(|dev| dev.id.clone()).collect(),
            log_path,
            status: "pending".into(),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run(args) => handle_run(args),
        Commands::List { limit, manifest } => {
            let entries = read_manifest(&manifest)?;
            for entry in entries.into_iter().rev().take(limit) {
                println!(
                    "{} | {} | {} | {} | {:?}",
                    entry.job_id, entry.profile, entry.status, entry.submitted_at, entry.devices
                );
            }
            Ok(())
        }
        Commands::Status { job_id, manifest } => {
            let entries = read_manifest(&manifest)?;
            if let Some(entry) = entries.into_iter().find(|e| e.job_id == job_id) {
                println!("{}", serde_json::to_string_pretty(&entry)?);
            } else {
                println!("job {job_id} not found");
            }
            Ok(())
        }
        Commands::Resume {
            checkpoint,
            config,
            profile,
            devices,
            log_dir,
            manifest,
            event_log,
        } => {
            if !checkpoint.exists() {
                anyhow::bail!("checkpoint {:?} not found", checkpoint);
            }
            let args = RunArgs {
                config,
                weights: Some(checkpoint),
                profile: profile.unwrap_or_else(|| "resume".into()),
                devices,
                cpu_only: false,
                device_count: 0,
                log_dir,
                manifest,
                event_log,
            };
            handle_run(args)
        }
    }
}

fn handle_run(args: RunArgs) -> Result<()> {
    anyhow::ensure!(args.config.exists(), "config file not found");
    if let Some(weights) = &args.weights {
        anyhow::ensure!(weights.exists(), "weights file not found");
    }

    let manager = DeviceManager::autodetect();
    let preference = match (&args.devices, args.cpu_only) {
        (Some(list), _) => {
            let explicit = list.split(',').map(|s| s.trim().to_string()).collect();
            DevicePreference::Explicit(explicit)
        }
        (None, true) => DevicePreference::CpuOnly,
        (None, false) => DevicePreference::default(),
    };

    let plan = manager.allocate(preference, args.device_count);
    let log_path = compute_log_path(&args.log_dir)?;
    let mut entry = JobManifestEntry::new(
        args.config.clone(),
        args.weights.clone(),
        args.profile.clone(),
        &plan,
        log_path.clone(),
    );

    entry.status = "queued".into();
    append_manifest(&args.manifest, &entry)?;

    let event_sink = if let Some(path) = args.event_log.as_ref() {
        Some(EventSink::new(path)?)
    } else {
        None
    };

    let event = serde_json::json!({
        "event": "job_submitted",
        "job_id": entry.job_id,
        "config": entry.config,
        "weights": entry.weights,
        "profile": entry.profile,
        "devices": entry.devices,
        "log_path": entry.log_path,
        "submitted_at": entry.submitted_at,
    });
    println!("{}", serde_json::to_string_pretty(&event)?);
    log_job_event(
        &entry.log_path,
        LogLevel::Info,
        "job queued",
        json!({ "job_id": entry.job_id, "profile": entry.profile }),
    )?;
    publish_job_event(
        event_sink.as_ref(),
        "training.job_submitted",
        &entry,
        json!({ "config": entry.config, "weights": entry.weights }),
    )?;

    update_status(&args.manifest, &entry.job_id, "running")?;
    log_job_event(
        &entry.log_path,
        LogLevel::Info,
        "job started",
        json!({ "job_id": entry.job_id }),
    )?;
    match spawn_runner(&entry, &args, &plan) {
        Ok(_) => {
            update_status(&args.manifest, &entry.job_id, "completed")?;
            log_job_event(
                &entry.log_path,
                LogLevel::Info,
                "job completed",
                json!({ "job_id": entry.job_id }),
            )?;
            publish_job_event(
                event_sink.as_ref(),
                "training.job_completed",
                &entry,
                json!({}),
            )?;
            Ok(())
        }
        Err(err) => {
            update_status(&args.manifest, &entry.job_id, "failed")?;
            log_job_event(
                &entry.log_path,
                LogLevel::Error,
                "job failed",
                json!({ "job_id": entry.job_id, "error": err.to_string() }),
            )?;
            publish_job_event(
                event_sink.as_ref(),
                "training.job_failed",
                &entry,
                json!({ "error": err.to_string() }),
            )?;
            Err(err)
        }
    }
}

fn append_manifest(path: &Path, entry: &JobManifestEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening manifest {:?}", path))?;
    serde_json::to_writer(&mut file, entry)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn read_manifest(path: &Path) -> Result<Vec<JobManifestEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: JobManifestEntry = serde_json::from_str(&line)?;
        entries.push(entry);
    }
    Ok(entries)
}

fn compute_log_path(base: &Path) -> Result<PathBuf> {
    let now = Local::now();
    let dir = base
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{:02}", now.day()));
    fs::create_dir_all(&dir)?;
    Ok(dir.join(format!(
        "job-{}.log.jsonl",
        Utc::now().format("%Y%m%d-%H%M%S")
    )))
}

fn spawn_runner(entry: &JobManifestEntry, args: &RunArgs, plan: &AllocationPlan) -> Result<()> {
    let runner = Path::new("ZAPPY--M/learning/pytorch_runner/main.py");
    anyhow::ensure!(runner.exists(), "runner script {:?} not found", runner);
    fs::create_dir_all(entry.log_path.parent().unwrap())?;
    let mut cmd = Command::new("python");
    cmd.arg(runner)
        .arg("--config")
        .arg(&args.config)
        .arg("--log-file")
        .arg(&entry.log_path)
        .arg("--profile")
        .arg(&args.profile);
    if let Some(weights) = &args.weights {
        cmd.arg("--weights").arg(weights);
    }
    if !entry.devices.is_empty() {
        cmd.arg("--devices").arg(entry.devices.join(","));
    }
    if let Some(cuda_visible) = cuda_visible_devices(plan) {
        cmd.env("CUDA_VISIBLE_DEVICES", cuda_visible);
    }
    let status = cmd.status().context("running pytorch runner")?;
    if status.success() {
        Ok(())
    } else {
        bail!("runner exited with status {status}")
    }
}

fn cuda_visible_devices(plan: &AllocationPlan) -> Option<String> {
    let ids: Vec<String> = plan
        .devices()
        .iter()
        .filter_map(|dev| dev.id.strip_prefix("cuda:").map(ToOwned::to_owned))
        .collect();
    if ids.is_empty() {
        None
    } else {
        Some(ids.join(","))
    }
}

fn update_status(path: &Path, job_id: &str, status: &str) -> Result<()> {
    let mut entries = read_manifest(path)?;
    let mut changed = false;
    for entry in &mut entries {
        if entry.job_id == job_id {
            entry.status = status.to_string();
            changed = true;
        }
    }
    if !changed {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    for entry in entries {
        serde_json::to_writer(&mut file, &entry)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

struct EventSink {
    runtime: Runtime,
    publisher: FileEventPublisher,
}

impl EventSink {
    fn new(path: &Path) -> Result<Self> {
        let publisher = FileEventPublisher::new(path)?;
        let runtime = Runtime::new()?;
        Ok(Self { runtime, publisher })
    }

    fn publish(&self, event: EventRecord) -> Result<()> {
        self.runtime.block_on(self.publisher.publish(event))
    }
}

fn publish_job_event(
    sink: Option<&EventSink>,
    event_type: &str,
    entry: &JobManifestEntry,
    payload: Value,
) -> Result<()> {
    if let Some(sink) = sink {
        let mut payload = payload;
        payload = match payload {
            Value::Object(mut map) => {
                map.insert("job_id".into(), Value::String(entry.job_id.clone()));
                Value::Object(map)
            }
            other => {
                let mut map = serde_json::Map::new();
                map.insert("job_id".into(), Value::String(entry.job_id.clone()));
                map.insert("data".into(), other);
                Value::Object(map)
            }
        };
        let record = EventRecord {
            id: format!("evt-{}", Uuid::new_v4()),
            source: "trn".into(),
            event_type: event_type.into(),
            timestamp: Utc::now().to_rfc3339(),
            payload,
        };
        sink.publish(record)?;
    }
    Ok(())
}

fn log_job_event(path: &Path, level: LogLevel, message: &str, metadata: Value) -> Result<()> {
    let logger = JsonLogger::new(path)?;
    let mut record = LogRecord::new("trn", level, message);
    if let Some(obj) = metadata.as_object() {
        record.metadata = obj.clone();
    }
    logger.log(&record)
}
