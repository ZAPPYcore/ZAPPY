#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Structured JSON logging utilities shared across modules.

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Log severity level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    /// Debug information.
    Debug,
    /// Informational events.
    Info,
    /// Warning indicator.
    Warn,
    /// Error indicator.
    Error,
}

/// Structured log record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    /// Timestamp in ISO8601.
    pub timestamp: DateTime<Utc>,
    /// Module emitting the log.
    pub module: String,
    /// Severity.
    pub level: LogLevel,
    /// Human-readable message.
    pub message: String,
    /// Arbitrary JSON payload for metrics/fields.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl LogRecord {
    /// Creates a record with the provided info.
    #[must_use]
    pub fn new(module: impl Into<String>, level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            module: module.into(),
            level,
            message: message.into(),
            metadata: serde_json::Map::new(),
        }
    }
}

/// Thread-safe JSON logger with append-only semantics.
#[derive(Debug)]
pub struct JsonLogger {
    path: PathBuf,
    writer: Mutex<File>,
}

impl JsonLogger {
    /// Creates or opens a logger at the desired path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            path,
            writer: Mutex::new(file),
        })
    }

    /// Writes a log record as JSON line.
    pub fn log(&self, record: &LogRecord) -> Result<()> {
        let mut writer = self.writer.lock();
        serde_json::to_writer(&mut *writer, record)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    /// Returns the underlying file path (useful for tests).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_json_lines() {
        let dir = tempdir().unwrap();
        let logger = JsonLogger::new(dir.path().join("test.log")).unwrap();
        logger
            .log(&LogRecord::new("module", LogLevel::Info, "hello"))
            .unwrap();
        let content = fs::read_to_string(logger.path()).unwrap();
        assert!(content.contains("\"message\":\"hello\""));
    }
}
