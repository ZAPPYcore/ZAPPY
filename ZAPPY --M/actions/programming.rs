use std::{
    path::{Component, PathBuf},
    process::Stdio,
    time::{Duration as StdDuration, Instant},
};

use serde::Deserialize;
use serde_json::Value;
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

use crate::{
    actions::{
        ActionArtifact, ActionError, ActionOutcome, ActionPlan, ActionRequest, ArtifactContent,
    },
    programminghelper::{CodeChangeProposal, ProgrammingHelper},
};

const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 60_000;
const MAX_COMMAND_TIMEOUT_MS: u64 = 300_000;
const MAX_COMMAND_OUTPUT_BYTES: usize = 32_768;
const SAFE_COMMANDS: &[&str] = &[
    "cargo",
    "python",
    "pip",
    "pytest",
    "npm",
    "yarn",
    "pnpm",
    "node",
    "bun",
    "bash",
    "sh",
    "make",
    "go",
    "mvn",
    "gradle",
    "deno",
    "pwsh",
    "powershell",
    "cmd",
    "dotnet",
    "npx",
    "java",
    "perl",
    "ruby",
];

/// Executes programming-oriented plans by generating code patches and running validation commands.
#[derive(Debug, Clone)]
pub struct ProgrammingActionExecutor {
    helper: ProgrammingHelper,
}

impl ProgrammingActionExecutor {
    /// Creates a new executor.
    #[must_use]
    pub fn new(helper: ProgrammingHelper) -> Self {
        Self { helper }
    }

    /// Executes the plan returning patch artifacts.
    pub async fn execute_plan(
        &self,
        request: &ActionRequest,
        plan: &ActionPlan,
    ) -> Result<ActionOutcome, ActionError> {
        let proposals = self.extract_proposals(request)?;
        let commands = self.extract_commands(request)?;

        let mut patches = Vec::new();
        for proposal in &proposals {
            let patch = self.helper.generate_patch(proposal)?;
            patches.push(patch);
        }

        let (command_reports, command_stats) = self.run_commands(commands).await;

        let summary = if command_stats.total > 0 {
            format!(
                "Prepared {} patches across {} plan steps; executed {} commands ({} success, {} failed, {} timeout, {} rejected)",
                patches.len(),
                plan.steps.len(),
                command_stats.total,
                command_stats.success,
                command_stats.failed,
                command_stats.timed_out,
                command_stats.rejected
            )
        } else {
            format!(
                "Prepared {} patches across {} plan steps",
                patches.len(),
                plan.steps.len()
            )
        };

        let mut artifacts = vec![ActionArtifact {
            label: "programming_patches".into(),
            importance: request.priority,
            content: ArtifactContent::Json(serde_json::json!(patches
                .into_iter()
                .map(|p| serde_json::json!({
                    "path": p.path,
                    "diff": p.diff,
                }))
                .collect::<Vec<_>>())),
        }];

        if command_stats.total > 0 {
            artifacts.push(ActionArtifact {
                label: "command_results".into(),
                importance: request.priority,
                content: ArtifactContent::Json(Value::Array(command_reports)),
            });
        }

        Ok(ActionOutcome::textual(summary, artifacts))
    }

    fn extract_proposals(
        &self,
        request: &ActionRequest,
    ) -> Result<Vec<CodeChangeProposal>, ActionError> {
        let mut proposals = Vec::new();

        for attachment in &request.payload.attachments {
            if attachment.label != "code_context" {
                continue;
            }

            let path = attachment
                .content
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ActionError::Invalid("code_context missing path".into()))?;
            let original = attachment
                .content
                .get("original")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let proposed = attachment
                .content
                .get("proposed")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ActionError::Invalid("code_context missing proposed".into()))?
                .to_string();

            proposals.push(CodeChangeProposal {
                path: path.into(),
                original,
                proposed,
            });
        }

        if proposals.is_empty() {
            return Err(ActionError::Invalid(
                "no code_context attachments provided".into(),
            ));
        }

        Ok(proposals)
    }

    fn extract_commands(&self, request: &ActionRequest) -> Result<Vec<CommandSpec>, ActionError> {
        let mut commands = Vec::new();
        for attachment in &request.payload.attachments {
            if attachment.label != "command_plan" {
                continue;
            }
            let spec: CommandAttachmentSpec = serde_json::from_value(attachment.content.clone())
                .map_err(|err| {
                    ActionError::Invalid(format!("invalid command_plan attachment: {err}"))
                })?;
            if spec.command.trim().is_empty() {
                continue;
            }
            commands.push(CommandSpec::try_from(spec)?);
        }
        Ok(commands)
    }

    async fn run_commands(&self, commands: Vec<CommandSpec>) -> (Vec<Value>, CommandStats) {
        let mut stats = CommandStats::default();
        let mut records = Vec::new();
        for spec in commands {
            stats.total += 1;
            let record = if is_command_allowed(&spec.program) {
                execute_command(spec).await
            } else {
                stats.rejected += 1;
                CommandExecutionRecord::rejected(spec, "command not in allowlist")
            };
            match record.status.as_str() {
                "success" => stats.success += 1,
                "failed" => stats.failed += 1,
                "timeout" => stats.timed_out += 1,
                "error" => stats.failed += 1,
                "rejected" => {}
                _ => {}
            }
            records.push(record.into_json());
        }
        (records, stats)
    }
}

#[derive(Default)]
struct CommandStats {
    total: usize,
    success: usize,
    failed: usize,
    timed_out: usize,
    rejected: usize,
}

#[derive(Debug, Clone)]
struct CommandSpec {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    stdin: Option<String>,
    timeout: StdDuration,
    description: Option<String>,
}

impl CommandSpec {
    fn try_from(raw: CommandAttachmentSpec) -> Result<Self, ActionError> {
        let program = raw.command.trim();
        if program.is_empty() {
            return Err(ActionError::Invalid("command_plan missing command".into()));
        }
        let mut timeout_ms = raw.timeout_ms.unwrap_or(DEFAULT_COMMAND_TIMEOUT_MS);
        if timeout_ms == 0 {
            timeout_ms = DEFAULT_COMMAND_TIMEOUT_MS;
        }
        if timeout_ms > MAX_COMMAND_TIMEOUT_MS {
            timeout_ms = MAX_COMMAND_TIMEOUT_MS;
        }
        let args = raw
            .args
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .filter(|arg| !arg.is_empty())
            .collect();
        let cwd = raw.cwd.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        });
        if let Some(path) = &cwd {
            for component in path.components() {
                if matches!(component, Component::ParentDir) {
                    return Err(ActionError::Invalid(
                        "command cwd cannot contain '..' segments".into(),
                    ));
                }
            }
        }
        let description = raw.description.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
        Ok(Self {
            program: program.to_string(),
            args,
            cwd,
            stdin: raw.stdin.filter(|s| !s.is_empty()),
            timeout: StdDuration::from_millis(timeout_ms),
            description,
        })
    }
}

#[derive(Debug, Deserialize)]
struct CommandAttachmentSpec {
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

#[derive(Debug, Clone)]
struct CommandExecutionRecord {
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    description: Option<String>,
    status: String,
    exit_code: Option<i32>,
    duration_ms: u64,
    stdout: String,
    stderr: String,
    error: Option<String>,
}

impl CommandExecutionRecord {
    fn new(spec: &CommandSpec, status: impl Into<String>) -> Self {
        Self {
            command: spec.program.clone(),
            args: spec.args.clone(),
            cwd: spec
                .cwd
                .as_ref()
                .and_then(|path| path.to_str().map(|s| s.to_string())),
            description: spec.description.clone(),
            status: status.into(),
            exit_code: None,
            duration_ms: 0,
            stdout: String::new(),
            stderr: String::new(),
            error: None,
        }
    }

    fn rejected(spec: CommandSpec, reason: impl Into<String>) -> Self {
        let mut record = Self::new(&spec, "rejected");
        record.error = Some(reason.into());
        record
    }

    fn into_json(self) -> Value {
        serde_json::json!({
            "command": self.command,
            "args": self.args,
            "cwd": self.cwd,
            "description": self.description,
            "status": self.status,
            "exit_code": self.exit_code,
            "duration_ms": self.duration_ms,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "error": self.error,
        })
    }
}

async fn execute_command(spec: CommandSpec) -> CommandExecutionRecord {
    let mut record = CommandExecutionRecord::new(&spec, "success");
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    if spec.stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.kill_on_drop(true);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            record.status = "error".into();
            record.error = Some(err.to_string());
            return record;
        }
    };

    if let Some(input) = &spec.stdin {
        if let Some(stdin) = child.stdin.as_mut() {
            if let Err(err) = stdin.write_all(input.as_bytes()).await {
                record.status = "error".into();
                record.error = Some(err.to_string());
                let _ = child.kill().await;
                return record;
            }
        }
    }

    let start = Instant::now();
    match timeout(spec.timeout, child.wait_with_output()).await {
        Ok(output_res) => match output_res {
            Ok(output) => {
                record.duration_ms = start.elapsed().as_millis() as u64;
                record.exit_code = output.status.code();
                record.stdout = sanitize_output(&output.stdout);
                record.stderr = sanitize_output(&output.stderr);
                if !output.status.success() {
                    record.status = "failed".into();
                }
                record
            }
            Err(err) => {
                record.status = "error".into();
                record.duration_ms = start.elapsed().as_millis() as u64;
                record.error = Some(err.to_string());
                record
            }
        },
        Err(_) => {
            record.status = "timeout".into();
            record.duration_ms = spec.timeout.as_millis() as u64;
            record.error = Some("command timed out".into());
            record
        }
    }
}

fn sanitize_output(bytes: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if text.len() > MAX_COMMAND_OUTPUT_BYTES {
        text.truncate(MAX_COMMAND_OUTPUT_BYTES);
        text.push_str("\n...[truncated]");
    }
    text
}

fn is_command_allowed(program: &str) -> bool {
    let lowered = program
        .rsplit(|ch| ch == '/' || ch == '\\')
        .next()
        .unwrap_or(program)
        .trim_end_matches(".exe")
        .trim_end_matches(".bat")
        .trim_end_matches(".cmd")
        .trim_end_matches(".ps1")
        .to_ascii_lowercase();
    SAFE_COMMANDS.iter().any(|allowed| *allowed == lowered)
}
