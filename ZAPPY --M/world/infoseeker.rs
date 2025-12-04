use std::{collections::HashMap, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{fs, time::sleep};

use crate::{
    feed_config::{FeedKind, FeedsDocument, FieldMapping, FileFeedConfig, HttpFeedConfig},
    telemetry::WorldTelemetry,
};

#[cfg(feature = "kafka")]
use crate::feed_config::KafkaFeedConfig;

/// Signal consumed by the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoSignal {
    /// Region impacted.
    pub region_id: String,
    /// Metric payload.
    pub metrics: Value,
    /// Severity estimate.
    pub severity: f32,
}

/// Trait for providers that fetch environmental signals.
#[async_trait]
pub trait InfoProvider: Send + Sync {
    /// Pulls signals from the provider.
    async fn pull(&self) -> Result<Vec<InfoSignal>>;
}

/// Info seeker orchestrates providers, validation, and telemetry.
pub struct InfoSeeker {
    providers: Vec<ProviderHandle>,
    telemetry: Option<WorldTelemetry>,
    validator: SignalValidator,
    batch_limit: usize,
}

impl InfoSeeker {
    /// Builder entry point.
    #[must_use]
    pub fn builder() -> InfoSeekerBuilder {
        InfoSeekerBuilder::default()
    }

    /// Builds a seeker from a feeds document.
    pub fn from_feeds_document(
        document: &FeedsDocument,
        telemetry: Option<WorldTelemetry>,
    ) -> Result<Self> {
        let mut builder = InfoSeeker::builder()
            .telemetry_opt(telemetry)
            .validator(
                SignalValidator::new(
                    document.global.required_metrics.clone(),
                    document.global.min_severity,
                    document.global.max_severity,
                )
                .context("invalid signal validator configuration")?,
            )
            .batch_limit(document.global.max_batch);
        for feed in &document.feeds {
            if !feed.enabled {
                continue;
            }
            let name = feed.name.clone();
            match &feed.kind {
                FeedKind::Http(cfg) => {
                    builder = builder.provider_named(
                        name,
                        Box::new(HttpJsonInfoProvider::new(feed.name.clone(), cfg.clone())?),
                        feed.weight,
                    );
                }
                FeedKind::File(cfg) => {
                    builder = builder.provider_named(
                        name,
                        Box::new(FileInfoProvider::new(cfg.clone())?),
                        feed.weight,
                    );
                }
                FeedKind::Kafka(cfg) => {
                    #[cfg(feature = "kafka")]
                    {
                        builder = builder.provider_named(
                            name,
                            Box::new(KafkaInfoProvider::new(feed.name.clone(), cfg.clone())?),
                            feed.weight,
                        );
                    }
                    #[cfg(not(feature = "kafka"))]
                    {
                        let _ = cfg;
                        bail!(
                            "feed '{}' requires kafka feature; recompile with --features kafka",
                            name
                        );
                    }
                }
            }
        }
        Ok(builder.build())
    }

    /// Pulls signals and returns them.
    pub async fn collect(&self) -> Result<Vec<InfoSignal>> {
        let mut aggregated = Vec::new();
        let mut failures = Vec::new();
        for handle in &self.providers {
            match handle.provider.pull().await {
                Ok(batch) => {
                    let valid = batch
                        .into_iter()
                        .filter_map(|signal| match self.validator.validate(&signal) {
                            Ok(_) => Some(signal),
                            Err(err) => {
                                failures.push(format!("{} invalid signal: {err:#}", handle.name));
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    if let Some(tel) = &self.telemetry {
                        let _ = tel.log(
                            shared_logging::LogLevel::Info,
                            "world.infoseeker.batch",
                            serde_json::json!({
                                "provider": handle.name,
                                "count": valid.len()
                            }),
                        );
                    }
                    aggregated.extend(valid);
                }
                Err(err) => {
                    failures.push(format!("{} failed: {err:#}", handle.name));
                    if let Some(tel) = &self.telemetry {
                        let _ = tel.log(
                            shared_logging::LogLevel::Warn,
                            "world.infoseeker.error",
                            serde_json::json!({
                                "provider": handle.name,
                                "error": err.to_string()
                            }),
                        );
                    }
                }
            }
        }
        if aggregated.is_empty() && !failures.is_empty() {
            bail!("all providers failed: {}", failures.join("; "));
        }
        aggregated = dedupe_by_region(aggregated);
        if aggregated.len() > self.batch_limit {
            aggregated.truncate(self.batch_limit);
        }
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "world.infoseeker.batch",
                serde_json::json!({ "signals": aggregated.len() }),
            );
        }
        Ok(aggregated)
    }
}

fn dedupe_by_region(mut signals: Vec<InfoSignal>) -> Vec<InfoSignal> {
    let mut seen: HashMap<String, InfoSignal> = HashMap::new();
    for signal in signals.drain(..) {
        seen.entry(signal.region_id.clone())
            .and_modify(|existing| {
                if signal.severity > existing.severity {
                    *existing = signal.clone();
                }
            })
            .or_insert(signal);
    }
    seen.into_values().collect()
}

/// Builder for `InfoSeeker`.
pub struct InfoSeekerBuilder {
    providers: Vec<ProviderHandle>,
    telemetry: Option<WorldTelemetry>,
    validator: SignalValidator,
    batch_limit: usize,
}

impl Default for InfoSeekerBuilder {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            telemetry: None,
            validator: SignalValidator::default(),
            batch_limit: 256,
        }
    }
}

impl InfoSeekerBuilder {
    /// Adds a provider with default metadata.
    #[must_use]
    pub fn provider(mut self, provider: Box<dyn InfoProvider>) -> Self {
        self.providers.push(ProviderHandle {
            name: "custom".into(),
            weight: 1,
            provider,
        });
        self
    }

    /// Adds a provider with explicit metadata.
    #[must_use]
    pub fn provider_named(
        mut self,
        name: impl Into<String>,
        provider: Box<dyn InfoProvider>,
        weight: usize,
    ) -> Self {
        self.providers.push(ProviderHandle {
            name: name.into(),
            weight: weight.max(1),
            provider,
        });
        self
    }

    /// Sets telemetry.
    #[must_use]
    pub fn telemetry(mut self, telemetry: WorldTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets telemetry optionally.
    #[must_use]
    pub fn telemetry_opt(mut self, telemetry: Option<WorldTelemetry>) -> Self {
        self.telemetry = telemetry;
        self
    }

    /// Overrides validator.
    #[must_use]
    pub fn validator(mut self, validator: SignalValidator) -> Self {
        self.validator = validator;
        self
    }

    /// Sets batch limit.
    #[must_use]
    pub fn batch_limit(mut self, limit: usize) -> Self {
        if limit > 0 {
            self.batch_limit = limit;
        }
        self
    }

    /// Builds the seeker.
    pub fn build(mut self) -> InfoSeeker {
        if self.providers.is_empty() {
            self.providers.push(ProviderHandle {
                name: "random".into(),
                weight: 1,
                provider: Box::new(RandomInfoProvider::new()),
            });
        }
        InfoSeeker {
            providers: self.providers,
            telemetry: self.telemetry,
            validator: self.validator,
            batch_limit: self.batch_limit,
        }
    }
}

struct ProviderHandle {
    name: String,
    #[allow(dead_code)]
    weight: usize,
    provider: Box<dyn InfoProvider>,
}

/// Validator ensuring required fields are present.
#[derive(Clone)]
pub struct SignalValidator {
    required_metrics: Vec<String>,
    min_severity: f32,
    max_severity: f32,
}

impl SignalValidator {
    /// Creates a validator with severity bounds.
    pub fn new(required_metrics: Vec<String>, min: f32, max: f32) -> Result<Self> {
        if !(0.0..=1.0).contains(&min) || !(0.0..=1.0).contains(&max) || min >= max {
            bail!("invalid severity bounds {}..{}", min, max);
        }
        Ok(Self {
            required_metrics,
            min_severity: min,
            max_severity: max,
        })
    }

    fn validate(&self, signal: &InfoSignal) -> Result<()> {
        if signal.region_id.trim().is_empty() {
            bail!("region id missing");
        }
        if !(self.min_severity..=self.max_severity).contains(&signal.severity) {
            bail!("severity {} out of bounds", signal.severity);
        }
        let metrics = signal
            .metrics
            .as_object()
            .ok_or_else(|| anyhow!("metrics must be object"))?;
        for key in &self.required_metrics {
            if !metrics.contains_key(key) {
                bail!("missing metric '{}'", key);
            }
        }
        Ok(())
    }
}

impl Default for SignalValidator {
    fn default() -> Self {
        Self {
            required_metrics: vec!["load".into(), "demand".into()],
            min_severity: 0.0,
            max_severity: 1.0,
        }
    }
}

/// Simple provider used for testing and fallback.
pub struct RandomInfoProvider {
    rng: SmallRng,
}

impl RandomInfoProvider {
    /// Creates the provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
        }
    }
}

#[async_trait]
impl InfoProvider for RandomInfoProvider {
    async fn pull(&self) -> Result<Vec<InfoSignal>> {
        let mut rng = self.rng.clone();
        sleep(Duration::from_millis(25)).await;
        let mut signals = Vec::new();
        for idx in 0..3 {
            signals.push(InfoSignal {
                region_id: format!("region-{}", idx),
                metrics: serde_json::json!({
                    "load": rng.gen_range(0.2..0.95),
                    "demand": rng.gen_range(0.3..0.9),
                }),
                severity: rng.gen_range(0.0..1.0),
            });
        }
        Ok(signals)
    }
}

struct SignalExtractor {
    mapping: FieldMapping,
}

impl SignalExtractor {
    fn new(mapping: FieldMapping) -> Self {
        Self { mapping }
    }

    fn extract(&self, node: &Value) -> Result<InfoSignal> {
        let region = self
            .get(node, &self.mapping.region_field)
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("region field '{}' missing", self.mapping.region_field))?;
        let metrics = self
            .get(node, &self.mapping.metrics_field)
            .cloned()
            .ok_or_else(|| anyhow!("metrics field '{}' missing", self.mapping.metrics_field))?;
        if !metrics.is_object() {
            bail!("metrics must be JSON object");
        }
        let severity = self
            .get(node, &self.mapping.severity_field)
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .or_else(|| {
                metrics
                    .get("load")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32)
            })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        Ok(InfoSignal {
            region_id: region,
            metrics,
            severity,
        })
    }

    fn get<'a>(&self, node: &'a Value, path: &str) -> Option<&'a Value> {
        value_at_path(node, path)
    }
}

struct HttpJsonInfoProvider {
    name: String,
    client: Client,
    config: HttpFeedConfig,
    extractor: SignalExtractor,
}

impl HttpJsonInfoProvider {
    fn new(name: impl Into<String>, config: HttpFeedConfig) -> Result<Self> {
        let client = Client::builder().user_agent("zappy-world/alpha").build()?;
        let extractor = SignalExtractor::new(config.mapping());
        Ok(Self {
            name: name.into(),
            client,
            config,
            extractor,
        })
    }

    fn array_entries<'a>(&self, value: &'a Value) -> Result<Vec<&'a Value>> {
        if let Some(root) = &self.config.array_root {
            let target = value_at_path(value, root)
                .ok_or_else(|| anyhow!("array root '{}' missing", root))?;
            if let Some(arr) = target.as_array() {
                return Ok(arr.iter().collect());
            } else {
                bail!("array root '{}' must be array", root);
            }
        }
        if let Some(arr) = value.as_array() {
            Ok(arr.iter().collect())
        } else {
            Ok(vec![value])
        }
    }

    fn parse_payload(&self, payload: Value) -> Result<Vec<InfoSignal>> {
        let entries = self.array_entries(&payload)?;
        entries
            .into_iter()
            .map(|entry| self.extractor.extract(entry))
            .collect()
    }
}

#[async_trait]
impl InfoProvider for HttpJsonInfoProvider {
    async fn pull(&self) -> Result<Vec<InfoSignal>> {
        let method: reqwest::Method = (&self.config.method).into();
        let mut request = self.client.request(method, &self.config.url);
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        let response = request
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .with_context(|| format!("http provider {} request failed", self.name))?;
        let payload = response
            .error_for_status()
            .with_context(|| format!("http provider {} bad status", self.name))?
            .json::<Value>()
            .await
            .with_context(|| format!("http provider {} invalid json", self.name))?;
        self.parse_payload(payload)
    }
}

struct FileInfoProvider {
    path: PathBuf,
    batch_max: usize,
    extractor: SignalExtractor,
}

impl FileInfoProvider {
    fn new(config: FileFeedConfig) -> Result<Self> {
        let FileFeedConfig {
            path,
            batch_max,
            mapping,
        } = config;
        Ok(Self {
            path,
            batch_max: batch_max.max(1),
            extractor: SignalExtractor::new(mapping.unwrap_or_default()),
        })
    }
}

#[async_trait]
impl InfoProvider for FileInfoProvider {
    async fn pull(&self) -> Result<Vec<InfoSignal>> {
        let data = fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("reading {}", self.path.display()))?;
        let mut signals = Vec::new();
        for line in data.lines().rev().filter(|l| !l.trim().is_empty()) {
            if signals.len() >= self.batch_max {
                break;
            }
            let value: Value = serde_json::from_str(line)
                .with_context(|| format!("invalid json line in {}", self.path.display()))?;
            signals.push(self.extractor.extract(&value)?);
        }
        signals.reverse();
        Ok(signals)
    }
}

#[cfg(feature = "kafka")]
struct KafkaInfoProvider {
    name: String,
    consumer: rdkafka::consumer::StreamConsumer,
    extractor: SignalExtractor,
    batch_max: usize,
    timeout: Duration,
}

#[cfg(feature = "kafka")]
impl KafkaInfoProvider {
    fn new(name: impl Into<String>, config: KafkaFeedConfig) -> Result<Self> {
        use rdkafka::{consumer::Consumer, ClientConfig};
        let KafkaFeedConfig {
            brokers,
            topic,
            group_id,
            batch_max,
            timeout_ms,
            mapping,
        } = config;
        let consumer: rdkafka::consumer::StreamConsumer = ClientConfig::new()
            .set("group.id", &group_id)
            .set("bootstrap.servers", &brokers)
            .set("enable.auto.commit", "true")
            .create()
            .context("creating kafka consumer")?;
        consumer
            .subscribe(&[topic.as_str()])
            .context("subscribing topic")?;
        Ok(Self {
            name: name.into(),
            consumer,
            extractor: SignalExtractor::new(mapping.unwrap_or_default()),
            batch_max: batch_max.max(1),
            timeout: Duration::from_millis(timeout_ms),
        })
    }
}

#[cfg(feature = "kafka")]
#[async_trait]
impl InfoProvider for KafkaInfoProvider {
    async fn pull(&self) -> Result<Vec<InfoSignal>> {
        use rdkafka::Message;
        let mut signals = Vec::new();
        while signals.len() < self.batch_max {
            match tokio::time::timeout(self.timeout, self.consumer.recv()).await {
                Ok(Ok(message)) => {
                    if let Some(payload) = message.payload_view::<str>().transpose()? {
                        let value: Value = serde_json::from_str(payload)
                            .with_context(|| format!("kafka {} invalid json", self.name))?;
                        signals.push(self.extractor.extract(&value)?);
                    }
                }
                Ok(Err(err)) => {
                    bail!("kafka {} error: {err}", self.name);
                }
                Err(_) => break,
            }
        }
        Ok(signals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn random_provider_returns_signals() {
        let seeker = InfoSeeker::builder()
            .provider(Box::new(RandomInfoProvider::new()))
            .build();
        let signals = seeker.collect().await.unwrap();
        assert!(!signals.is_empty());
    }

    #[test]
    fn validator_rejects_missing_metrics() {
        let validator = SignalValidator::default();
        let result = validator.validate(&InfoSignal {
            region_id: "alpha".into(),
            metrics: Value::Null,
            severity: 0.5,
        });
        assert!(result.is_err());
    }
}

fn value_at_path<'a>(mut node: &'a Value, path: &str) -> Option<&'a Value> {
    for key in path.split('.') {
        if key.is_empty() {
            continue;
        }
        node = node.get(key)?;
    }
    Some(node)
}
