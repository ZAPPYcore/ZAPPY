use std::{path::PathBuf, sync::Arc};

use tokio::{fs, sync::Semaphore};

use crate::actions::{ActionError, ActionOutcome, ActionPlan, ActionRequest};

/// Represents a deterministic file mutation to apply.
#[derive(Debug, Clone)]
pub struct FileMutation {
    /// Relative path from the executor root.
    pub relative_path: PathBuf,
    /// New file contents.
    pub contents: String,
}

/// Report returned after executing offline actions.
#[derive(Debug, Clone)]
pub struct OfflineActionReport {
    /// Files that were touched.
    pub mutated_files: Vec<PathBuf>,
    /// Operator friendly log entries.
    pub logs: Vec<String>,
}

/// Executor responsible for sandboxed filesystem interactions.
#[derive(Debug, Clone)]
pub struct OfflineActionExecutor {
    root: Arc<PathBuf>,
    semaphore: Arc<Semaphore>,
}

impl OfflineActionExecutor {
    /// Creates a new executor rooted at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, max_concurrency: usize) -> Self {
        Self {
            root: Arc::new(root.into()),
            semaphore: Arc::new(Semaphore::new(max_concurrency.max(1))),
        }
    }

    /// Applies file mutations derived from the action plan.
    pub async fn execute_plan(
        &self,
        request: &ActionRequest,
        plan: &ActionPlan,
        mutations: Vec<FileMutation>,
    ) -> Result<ActionOutcome, ActionError> {
        let mut logs = Vec::new();
        let mut mutated_files = Vec::new();

        for mutation in mutations {
            let path = self
                .sanitize(&mutation.relative_path)
                .map_err(|err| ActionError::Execution(err.to_string()))?;
            let permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|err| ActionError::Infrastructure(err.to_string()))?;
            let parent = path.parent().unwrap_or_else(|| self.root.as_path());
            {
                let _permit = permit;
                fs::create_dir_all(parent)
                    .await
                    .map_err(|err| ActionError::Execution(err.to_string()))?;
                fs::write(&path, mutation.contents.as_bytes())
                    .await
                    .map_err(|err| ActionError::Execution(err.to_string()))?;
            }
            logs.push(format!("Mutated {:?}", path));
            mutated_files.push(path);
        }

        let summary = format!(
            "Applied {} offline steps for action {} ({:?})",
            plan.steps.len(),
            request.id,
            request.intent
        );

        Ok(ActionOutcome::textual(
            summary,
            vec![crate::actions::ActionArtifact {
                label: "offline_report".into(),
                importance: request.priority,
                content: crate::actions::ArtifactContent::Json(serde_json::json!({
                    "mutated_files": mutated_files,
                    "logs": logs,
                })),
            }],
        ))
    }

    fn sanitize(&self, relative: &PathBuf) -> Result<PathBuf, &'static str> {
        let mut candidate = self.root.as_ref().clone();
        candidate.push(relative);
        let canonical = candidate
            .canonicalize()
            .map_err(|_| "unable to canonicalize path")?;
        if !canonical.starts_with(&*self.root) {
            return Err("path traversal detected");
        }
        Ok(canonical)
    }
}
