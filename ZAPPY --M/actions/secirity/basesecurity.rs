use crate::{
    actions::{ActionDomain, ActionIntent, ActionPriority, ActionRequest, ActionSafetyClass},
    security_link::SecurityGrade,
};

/// Result of evaluating a base policy rule set.
#[derive(Debug, Clone)]
pub struct BasePolicyDecision {
    /// Whether the action is permitted, requires escalation, or is denied.
    pub effect: PolicyEffect,
    /// Minimum security grade required to proceed.
    pub required_grade: SecurityGrade,
    /// Human readable notes.
    pub notes: Vec<String>,
}

impl BasePolicyDecision {
    /// Creates an allow decision.
    #[must_use]
    pub fn allow() -> Self {
        Self {
            effect: PolicyEffect::Allow,
            required_grade: SecurityGrade::Low,
            notes: Vec::new(),
        }
    }
}

/// Policy action to apply when a rule matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    /// Allow execution to continue.
    Allow,
    /// Escalate to a higher security grade for additional review.
    Escalate,
    /// Deny the action outright.
    Deny,
}

/// Declarative representation of a safety rule.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    id: &'static str,
    description: &'static str,
    domains: Vec<ActionDomain>,
    intents: Vec<ActionIntent>,
    max_priority: ActionPriority,
    enforced_grade: SecurityGrade,
    effect: PolicyEffect,
    safety_class: ActionSafetyClass,
}

impl PolicyRule {
    /// Evaluates the rule against the provided request.
    #[must_use]
    pub fn matches(&self, request: &ActionRequest) -> bool {
        (self.domains.is_empty() || self.domains.contains(&request.domain))
            && (self.intents.is_empty() || self.intents.contains(&request.intent))
            && request.priority <= self.max_priority
    }
}

/// Global security policy used as a baseline.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    rules: Vec<PolicyRule>,
}

impl SecurityPolicy {
    /// Builds the hardened Tier-9 default policy.
    #[must_use]
    pub fn default_global() -> Self {
        Self {
            rules: vec![
                PolicyRule {
                    id: "deny-critical-self-training",
                    description: "Self-training actions at critical priority require human review",
                    domains: vec![ActionDomain::SelfTraining],
                    intents: vec![ActionIntent::Learn],
                    max_priority: ActionPriority::High,
                    enforced_grade: SecurityGrade::Maximum,
                    effect: PolicyEffect::Escalate,
                    safety_class: ActionSafetyClass::Red,
                },
                PolicyRule {
                    id: "deny-external-ops",
                    description:
                        "Security remediation in external infrastructure cannot auto-execute",
                    domains: vec![ActionDomain::Security],
                    intents: vec![ActionIntent::Execute, ActionIntent::Remediate],
                    max_priority: ActionPriority::Critical,
                    enforced_grade: SecurityGrade::High,
                    effect: PolicyEffect::Escalate,
                    safety_class: ActionSafetyClass::Orange,
                },
                PolicyRule {
                    id: "allow-programming",
                    description: "Programming requests default to medium guardrails",
                    domains: vec![ActionDomain::Programming],
                    intents: Vec::new(),
                    max_priority: ActionPriority::Critical,
                    enforced_grade: SecurityGrade::Medium,
                    effect: PolicyEffect::Allow,
                    safety_class: ActionSafetyClass::Yellow,
                },
            ],
        }
    }

    /// Evaluates the policy returning the strongest triggered effect.
    #[must_use]
    pub fn evaluate(&self, request: &ActionRequest) -> BasePolicyDecision {
        let mut decision = BasePolicyDecision::allow();

        for rule in &self.rules {
            if !rule.matches(request) {
                continue;
            }

            decision.required_grade = decision.required_grade.max(rule.enforced_grade);
            decision.notes.push(format!(
                "{} triggered ({} | safety {:?})",
                rule.id, rule.description, rule.safety_class
            ));

            match rule.effect {
                PolicyEffect::Allow => {}
                PolicyEffect::Escalate => {
                    decision.effect = PolicyEffect::Escalate;
                }
                PolicyEffect::Deny => {
                    decision.effect = PolicyEffect::Deny;
                    decision.required_grade = SecurityGrade::Maximum;
                    break;
                }
            }
        }

        decision
    }
}
