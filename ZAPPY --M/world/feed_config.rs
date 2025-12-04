use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Top-level document describing external feeds.
#[derive(Debug, Clone)]
pub struct FeedsDocument {
    /// Schema version (currently informational).
    pub version: u32,
    /// Global settings applied to all feeds.
    pub global: GlobalFeedSettings,
    /// Individual feed declarations.
    pub feeds: Vec<FeedConfig>,
    source_dir: PathBuf,
}

impl FeedsDocument {
    /// Loads configuration from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading feeds config {}", path.display()))?;
        let mut document: FeedsDocumentSerde =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        if document.global.max_batch == 0 {
            document.global.max_batch = default_max_batch();
        }
        if (document.global.min_severity..document.global.max_severity).is_empty() {
            bail!("invalid severity bounds: min >= max");
        }
        if document.global.required_metrics.is_empty() {
            document.global.required_metrics = default_required_metrics();
        }
        let source_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        document
            .feeds
            .iter_mut()
            .filter_map(|feed| match &mut feed.kind {
                FeedKind::File(file) => Some(file),
                _ => None,
            })
            .for_each(|file| {
                if file.path.is_relative() {
                    file.path = source_dir.join(&file.path);
                }
            });
        Ok(Self {
            version: document.version,
            global: document.global,
            feeds: document.feeds,
            source_dir,
        })
    }

    /// Resolves a path relative to the configuration file.
    #[must_use]
    pub fn resolve_path(&self, candidate: impl AsRef<Path>) -> PathBuf {
        let candidate = candidate.as_ref();
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.source_dir.join(candidate)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct FeedsDocumentSerde {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    global: GlobalFeedSettings,
    #[serde(default)]
    feeds: Vec<FeedConfig>,
}

/// Settings shared across all feeds.
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalFeedSettings {
    /// Maximum number of signals retained per refresh cycle.
    #[serde(default = "default_max_batch")]
    pub max_batch: usize,
    /// Minimum accepted severity (inclusive).
    #[serde(default = "default_min_severity")]
    pub min_severity: f32,
    /// Maximum accepted severity (inclusive).
    #[serde(default = "default_max_severity")]
    pub max_severity: f32,
    /// Metrics that must be present on every signal.
    #[serde(default = "default_required_metrics")]
    pub required_metrics: Vec<String>,
}

impl Default for GlobalFeedSettings {
    fn default() -> Self {
        Self {
            max_batch: default_max_batch(),
            min_severity: default_min_severity(),
            max_severity: default_max_severity(),
            required_metrics: default_required_metrics(),
        }
    }
}

/// Definition of a single external feed.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedConfig {
    /// Human friendly name.
    pub name: String,
    /// Whether the feed is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Relative priority when sampling (currently informational).
    #[serde(default = "default_weight")]
    pub weight: usize,
    /// Concrete feed configuration to use.
    #[serde(flatten)]
    pub kind: FeedKind,
}

/// Supported feed kinds.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FeedKind {
    /// HTTP/JSON array feed.
    Http(HttpFeedConfig),
    /// Kafka/streaming feed.
    Kafka(KafkaFeedConfig),
    /// File-based JSONL feed.
    File(FileFeedConfig),
}

/// Mapping of JSON fields to InfoSignal structure.
#[derive(Debug, Clone, Deserialize)]
pub struct FieldMapping {
    /// JSON path used to extract the region identifier.
    #[serde(default = "default_region_field")]
    pub region_field: String,
    /// JSON path used to extract the metrics payload.
    #[serde(default = "default_metrics_field")]
    pub metrics_field: String,
    /// JSON path used to extract severity (falls back to metrics.load).
    #[serde(default = "default_severity_field")]
    pub severity_field: String,
}

impl Default for FieldMapping {
    fn default() -> Self {
        Self {
            region_field: default_region_field(),
            metrics_field: default_metrics_field(),
            severity_field: default_severity_field(),
        }
    }
}

/// HTTP feed configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct HttpFeedConfig {
    /// Endpoint returning JSON payloads.
    pub url: String,
    /// HTTP method used to call the endpoint.
    #[serde(default)]
    pub method: HttpMethod,
    /// Extra headers attached to the request.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Optional JSON path pointing to the array within the response.
    #[serde(default)]
    pub array_root: Option<String>,
    /// Request timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Optional bearer token applied to the request.
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Field mappings overriding the defaults.
    #[serde(default)]
    pub mapping: Option<FieldMapping>,
}

impl HttpFeedConfig {
    /// Returns effective field mapping.
    #[must_use]
    pub fn mapping(&self) -> FieldMapping {
        self.mapping.clone().unwrap_or_default()
    }
}

/// File feed configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct FileFeedConfig {
    /// Absolute or relative path to the JSONL file.
    pub path: PathBuf,
    /// Maximum number of lines consumed per refresh.
    #[serde(default = "default_batch_max")]
    pub batch_max: usize,
    /// Field mappings overriding the defaults.
    #[serde(default)]
    pub mapping: Option<FieldMapping>,
}

impl FileFeedConfig {
    /// Returns effective mapping, applying defaults if unspecified.
    #[must_use]
    pub fn mapping(&self) -> FieldMapping {
        self.mapping.clone().unwrap_or_default()
    }
}

/// Kafka feed configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaFeedConfig {
    /// Kafka bootstrap servers (comma separated).
    pub brokers: String,
    /// Topic containing JSON signal records.
    pub topic: String,
    /// Consumer group identifier.
    #[serde(default = "default_group_id")]
    pub group_id: String,
    /// Maximum number of messages fetched per refresh.
    #[serde(default = "default_batch_max")]
    pub batch_max: usize,
    /// Poll timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Field mappings overriding the defaults.
    #[serde(default)]
    pub mapping: Option<FieldMapping>,
}

impl KafkaFeedConfig {
    /// Returns effective mapping, applying defaults if unspecified.
    #[must_use]
    pub fn mapping(&self) -> FieldMapping {
        self.mapping.clone().unwrap_or_default()
    }
}

/// Supported HTTP verbs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// HTTP GET request.
    Get,
    /// HTTP POST request.
    Post,
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Get
    }
}

impl From<&HttpMethod> for reqwest::Method {
    fn from(value: &HttpMethod) -> Self {
        match value {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_weight() -> usize {
    1
}

const fn default_max_batch() -> usize {
    256
}

const fn default_batch_max() -> usize {
    64
}

const fn default_timeout_ms() -> u64 {
    4_000
}

const fn default_min_severity() -> f32 {
    0.0
}

const fn default_max_severity() -> f32 {
    1.0
}

fn default_required_metrics() -> Vec<String> {
    vec!["load".into(), "demand".into()]
}

fn default_region_field() -> String {
    "region".into()
}

fn default_metrics_field() -> String {
    "metrics".into()
}

fn default_severity_field() -> String {
    "severity".into()
}

fn default_group_id() -> String {
    "zappy-world".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_and_normalizes_document() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("feeds.toml");
        let data_path = dir.path().join("signals.jsonl");
        fs::write(
            &config_path,
            r#"
version = 1

[global]
max_batch = 128

[[feeds]]
name = "file"
kind = "file"
path = "signals.jsonl"
batch_max = 10
"#,
        )
        .unwrap();
        fs::write(&data_path, "").unwrap();
        let doc = FeedsDocument::load(&config_path).unwrap();
        assert_eq!(doc.feeds.len(), 1);
        match &doc.feeds[0].kind {
            FeedKind::File(file) => assert!(file.path.is_absolute()),
            _ => panic!("expected file feed"),
        }
    }
}
