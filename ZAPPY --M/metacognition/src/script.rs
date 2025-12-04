use serde::{Deserialize, Serialize};

use crate::methods::ReflectionPlan;

/// Script engine that transforms reflection plans into executable scripts.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScriptEngine;

impl ScriptEngine {
    /// Renders the plan into a script string.
    pub fn render(&self, plan: &ReflectionPlan) -> anyhow::Result<String> {
        let mut script = String::new();
        script.push_str(&format!(
            "# Reflection for {}\nmethod: {:?}\n",
            plan.observation.description, plan.method
        ));
        for (idx, step) in plan.steps.iter().enumerate() {
            script.push_str(&format!("step {}: {}\n", idx + 1, step));
        }
        Ok(script)
    }
}
