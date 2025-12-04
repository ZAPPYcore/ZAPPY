//! Core comprehension pipeline wiring.

/// Advanced async controller orchestrating comprehension batches.
pub mod advanced;
/// Ranking algorithms and heuristics.
pub mod algo;
/// Base comprehension engine.
pub mod comprehension;
/// Helper utilities for text normalization.
pub mod helper;
/// Method definitions enumerating comprehension strategies.
pub mod method;

pub use advanced::{AdvancedComprehensionController, EvidenceBundle};
pub use algo::{rank_sentences, SentenceScore};
pub use comprehension::{
    ComprehensionEngine, ComprehensionRequest, ComprehensionResult, EvidencePassage,
};
pub use method::ComprehensionMethod;
