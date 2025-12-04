use anyhow::Result;

use crate::{
    create::{CreativeBrief, CreativePortfolio, CreativityDialect},
    mainfunc::CreativityKernel,
};

/// Builds a creative brief from primitive parameters.
#[must_use]
pub fn build_brief(
    title: impl Into<String>,
    objective: impl Into<String>,
    dialect: CreativityDialect,
) -> CreativeBrief {
    CreativeBrief::new(title, objective, dialect)
}

/// Runs a batch of briefs through the kernel, returning the resulting portfolios.
pub fn run_batch(
    kernel: &mut CreativityKernel,
    briefs: impl IntoIterator<Item = CreativeBrief>,
) -> Result<Vec<CreativePortfolio>> {
    let mut portfolios = Vec::new();
    for brief in briefs {
        portfolios.push(kernel.run_cycle(brief)?);
    }
    Ok(portfolios)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration_entry::CreativityRuntime;

    #[test]
    fn batch_runner_executes() {
        let runtime = CreativityRuntime::default();
        let mut kernel = CreativityKernel::new(runtime);
        let briefs = vec![build_brief(
            "Helio",
            "Craft solar myths",
            CreativityDialect::Poetic,
        )];
        let portfolios = run_batch(&mut kernel, briefs).unwrap();
        assert_eq!(portfolios.len(), 1);
    }
}
