use serde::{Deserialize, Serialize};

/// Supported comprehension methods.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ComprehensionMethod {
    /// Dense retrieval followed by abstractive synthesis.
    DenseGenerative,
    /// Extractive QA over token windows.
    Extractive,
    /// Hybrid approach combining dense search with rule heuristics.
    Hybrid,
}

impl ComprehensionMethod {
    /// Returns human-readable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::DenseGenerative => "dense+generative",
            Self::Extractive => "extractive",
            Self::Hybrid => "hybrid",
        }
    }
}
