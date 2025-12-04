//! Console command ingestion for the NLP runtime.

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::mpsc::UnboundedSender,
};

use crate::telemetry::NlpTelemetry;

/// Commands accepted from the console.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsoleCommand {
    /// Ask a natural-language question with context.
    Ask {
        /// Question payload.
        question: String,
        /// Inline context passage.
        context: String,
    },
    /// Trigger training with a config path.
    Train {
        /// Path to a training config.
        config: String,
    },
    /// Exit the loop.
    Quit,
}

/// Receives JSON line commands from stdin, dispatching to runtime.
pub struct ConsoleCommandReceiver {
    sender: UnboundedSender<ConsoleCommand>,
    telemetry: Option<NlpTelemetry>,
}

impl ConsoleCommandReceiver {
    /// Creates a new receiver.
    #[must_use]
    pub fn new(sender: UnboundedSender<ConsoleCommand>, telemetry: Option<NlpTelemetry>) -> Self {
        Self { sender, telemetry }
    }

    /// Runs the loop until `Quit`.
    pub async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let cmd: ConsoleCommand =
                serde_json::from_str(&line).with_context(|| "invalid console command JSON")?;
            if matches!(cmd, ConsoleCommand::Quit) {
                break;
            }
            self.sender.send(cmd)?;
        }
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "nlp.console.receiver_shutdown",
                serde_json::json!({}),
            );
        }
        Ok(())
    }
}
