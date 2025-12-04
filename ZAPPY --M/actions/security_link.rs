use std::{collections::VecDeque, sync::Arc};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{
    actions::{ActionError, ActionId, ActionRequest},
    security::{
        commander::{CommanderVerdict, SecurityCommander},
        AdvancedSecurityAnalyzer, SecurityPolicy,
    },
};

/// Granular security grade used by both the commander and downstream services.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityGrade {
    /// Minimal guardrails.
    Low,
    /// Standard production guardrails.
    Medium,
    /// Elevated guardrails with on-call awareness.
    High,
    /// Maximum guardrails requiring executive approval.
    Maximum,
}

impl SecurityGrade {
    /// Returns the stricter of the two grades.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        if self >= other {
            self
        } else {
            other
        }
    }
}

/// Public verdict structure consumed by orchestrators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVerdict {
    /// Approval flag.
    pub approved: bool,
    /// Grade enforced for downstream execution.
    pub grade: SecurityGrade,
    /// Detailed justification.
    pub notes: Vec<String>,
}

/// Config builder for [`SecurityLink`].
#[derive(Debug)]
pub struct SecurityLinkBuilder {
    policy: SecurityPolicy,
    analyzer: AdvancedSecurityAnalyzer,
    cache_size: usize,
}

impl Default for SecurityLinkBuilder {
    fn default() -> Self {
        Self {
            policy: SecurityPolicy::default_global(),
            analyzer: AdvancedSecurityAnalyzer::hardened(),
            cache_size: 128,
        }
    }
}

impl SecurityLinkBuilder {
    /// Overrides the security policy.
    #[must_use]
    pub fn policy(mut self, policy: SecurityPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Overrides the analyzer.
    #[must_use]
    pub fn analyzer(mut self, analyzer: AdvancedSecurityAnalyzer) -> Self {
        self.analyzer = analyzer;
        self
    }

    /// Sets the cache size.
    #[must_use]
    pub fn cache_size(mut self, cache_size: usize) -> Self {
        self.cache_size = cache_size.max(16);
        self
    }

    /// Builds the link.
    #[must_use]
    pub fn build(self) -> SecurityLink {
        let commander = SecurityCommander::new(self.policy, self.analyzer);
        SecurityLink {
            commander: Arc::new(commander),
            cache: RwLock::new(VecDeque::new()),
            cache_size: self.cache_size,
        }
    }
}

/// Facade that exposes security evaluation to the rest of the system.
#[derive(Debug)]
pub struct SecurityLink {
    commander: Arc<SecurityCommander>,
    cache: RwLock<VecDeque<(ActionId, SecurityVerdict)>>,
    cache_size: usize,
}

impl SecurityLink {
    /// Creates a builder using hardened defaults.
    #[must_use]
    pub fn builder() -> SecurityLinkBuilder {
        SecurityLinkBuilder::default()
    }

    /// Evaluates a request and caches the verdict.
    #[must_use]
    pub async fn evaluate(&self, request: &ActionRequest) -> SecurityVerdict {
        if let Some(verdict) = self.cached(request.id) {
            return verdict;
        }

        let verdict = self.translate(self.commander.evaluate(request));
        self.insert_cache(request.id, verdict.clone());
        verdict
    }

    /// Ensures approval or returns an error.
    pub async fn enforce(&self, request: &ActionRequest) -> Result<SecurityVerdict, ActionError> {
        let verdict = self.evaluate(request).await;
        if verdict.approved {
            Ok(verdict)
        } else {
            Err(ActionError::SecurityViolation {
                grade: verdict.grade,
                message: verdict.notes.join("; "),
            })
        }
    }

    fn translate(&self, verdict: CommanderVerdict) -> SecurityVerdict {
        SecurityVerdict {
            approved: verdict.approved,
            grade: verdict.grade,
            notes: verdict.notes,
        }
    }

    fn cached(&self, id: ActionId) -> Option<SecurityVerdict> {
        self.cache
            .read()
            .iter()
            .find(|(cached_id, _)| *cached_id == id)
            .map(|(_, verdict)| verdict.clone())
    }

    fn insert_cache(&self, id: ActionId, verdict: SecurityVerdict) {
        let mut cache = self.cache.write();
        cache.push_front((id, verdict));
        if cache.len() > self.cache_size {
            cache.pop_back();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{ActionDomain, ActionIntent, ActionPayload, ActionRequest};

    #[tokio::test]
    async fn denies_high_risk_request() {
        let payload = ActionPayload::textual("Exploit dev", "zero day exploit kit");
        let request =
            ActionRequest::builder(ActionDomain::Security, ActionIntent::Execute, payload).build();
        let link = SecurityLink::builder().build();

        let verdict = link.evaluate(&request).await;
        assert!(!verdict.approved);
    }
}
