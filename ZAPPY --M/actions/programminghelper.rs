use std::path::PathBuf;

use diff::lines;

use crate::actions::ActionError;

/// Represents a proposed code change for a single file.
#[derive(Debug, Clone)]
pub struct CodeChangeProposal {
    /// Target file path.
    pub path: PathBuf,
    /// Original contents.
    pub original: String,
    /// Proposed contents.
    pub proposed: String,
}

/// Normalized patch ready for review.
#[derive(Debug, Clone)]
pub struct PatchPreview {
    /// Target file path.
    pub path: PathBuf,
    /// Unified diff string.
    pub diff: String,
}

/// Helper that generates diffs and enforces guardrails.
#[derive(Debug, Clone)]
pub struct ProgrammingHelper {
    max_diff_lines: usize,
}

impl ProgrammingHelper {
    /// Creates a new helper.
    #[must_use]
    pub fn new(max_diff_lines: usize) -> Self {
        Self { max_diff_lines }
    }

    /// Generates a unified diff for the given proposal.
    pub fn generate_patch(
        &self,
        proposal: &CodeChangeProposal,
    ) -> Result<PatchPreview, ActionError> {
        let mut diff_body = String::new();
        for result in lines(&proposal.original, &proposal.proposed) {
            match result {
                diff::Result::Left(line) => {
                    diff_body.push_str(&format!("-{line}\n"));
                }
                diff::Result::Right(line) => {
                    diff_body.push_str(&format!("+{line}\n"));
                }
                diff::Result::Both(line, _) => {
                    diff_body.push_str(&format!(" {line}\n"));
                }
            }
        }

        let line_count = diff_body.lines().count();
        if line_count > self.max_diff_lines {
            return Err(ActionError::Execution(format!(
                "diff too large ({line_count} > {})",
                self.max_diff_lines
            )));
        }

        Ok(PatchPreview {
            path: proposal.path.clone(),
            diff: diff_body,
        })
    }
}
