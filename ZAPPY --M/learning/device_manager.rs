use std::{
    fmt,
    process::{Command, Stdio},
    thread,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Type of device that can execute learning workloads.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// CPU fallback device.
    Cpu,
    /// NVIDIA CUDA-capable GPU.
    NvidiaGpu,
    /// AMD ROCm-capable GPU.
    AmdGpu,
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::NvidiaGpu => write!(f, "cuda"),
            Self::AmdGpu => write!(f, "rocm"),
        }
    }
}

/// Device metadata discovered on the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Stable identifier (e.g., `cuda:0`).
    pub id: String,
    /// Ordinal within its device family.
    pub ordinal: usize,
    /// Friendly device name.
    pub name: String,
    /// Device kind.
    pub kind: DeviceKind,
    /// Total memory in bytes (best effort).
    pub memory_total_bytes: u64,
}

impl DeviceInfo {
    fn cpu_default() -> Self {
        let threads = thread::available_parallelism()
            .map(|v| v.get())
            .unwrap_or(1);
        Self {
            id: "cpu:0".to_string(),
            ordinal: 0,
            name: format!("CPU ({threads} threads)"),
            kind: DeviceKind::Cpu,
            memory_total_bytes: 0,
        }
    }
}

/// Preferred allocation strategy.
#[derive(Debug, Clone)]
pub enum DevicePreference {
    /// Prioritize GPUs, fall back to CPU when exhausted.
    GpuFirst,
    /// Restrict execution to CPUs.
    CpuOnly,
    /// Explicit list of device identifiers.
    Explicit(Vec<String>),
}

impl Default for DevicePreference {
    fn default() -> Self {
        Self::GpuFirst
    }
}

/// Result of an allocation request.
#[derive(Debug, Clone)]
pub struct AllocationPlan {
    selected: Vec<DeviceInfo>,
}

impl AllocationPlan {
    /// Devices chosen for the workload.
    #[must_use]
    pub fn devices(&self) -> &[DeviceInfo] {
        &self.selected
    }

    fn new(selected: Vec<DeviceInfo>) -> Self {
        Self { selected }
    }
}

/// Errors emitted while probing for devices.
#[derive(Debug, Error)]
pub enum DeviceDetectionError {
    /// Command execution failed.
    #[error("failed to execute {command}: {message}")]
    CommandFailure {
        /// Command attempted (e.g., `nvidia-smi`).
        command: &'static str,
        /// Raw error payload.
        message: String,
    },
}

/// Central manager responsible for device discovery and allocation policy.
#[derive(Debug, Clone)]
pub struct DeviceManager {
    devices: Vec<DeviceInfo>,
}

impl DeviceManager {
    /// Autodetects devices on the host (best effort).
    #[must_use]
    pub fn autodetect() -> Self {
        match detect_devices() {
            Ok(devices) if !devices.is_empty() => Self { devices },
            _ => Self {
                devices: vec![DeviceInfo::cpu_default()],
            },
        }
    }

    /// Creates a manager from a predefined list of devices.
    #[must_use]
    pub fn from_devices(devices: Vec<DeviceInfo>) -> Self {
        if devices.is_empty() {
            Self {
                devices: vec![DeviceInfo::cpu_default()],
            }
        } else {
            Self { devices }
        }
    }

    /// Immutable view of known devices.
    #[must_use]
    pub fn devices(&self) -> &[DeviceInfo] {
        &self.devices
    }

    /// Picks devices given a preference and desired count (0 = all).
    #[must_use]
    pub fn allocate(&self, preference: DevicePreference, count: usize) -> AllocationPlan {
        let mut candidates: Vec<DeviceInfo> = match preference {
            DevicePreference::CpuOnly => self
                .devices
                .iter()
                .filter(|dev| dev.kind == DeviceKind::Cpu)
                .cloned()
                .collect(),
            DevicePreference::Explicit(ids) => self
                .devices
                .iter()
                .filter(|dev| ids.iter().any(|id| id == &dev.id))
                .cloned()
                .collect(),
            DevicePreference::GpuFirst => {
                let mut ordered: Vec<DeviceInfo> = self.devices.clone();
                ordered.sort_by_key(|dev| match dev.kind {
                    DeviceKind::NvidiaGpu | DeviceKind::AmdGpu => 0,
                    DeviceKind::Cpu => 1,
                });
                ordered
            }
        };

        if candidates.is_empty() {
            candidates = vec![DeviceInfo::cpu_default()];
        }

        let take = if count == 0 {
            candidates.len()
        } else {
            count.min(candidates.len())
        };
        AllocationPlan::new(candidates.into_iter().take(take).collect())
    }
}

fn detect_devices() -> Result<Vec<DeviceInfo>, DeviceDetectionError> {
    let mut devices = Vec::new();
    devices.extend(detect_nvidia()?);
    // TODO: add ROCm detection parity once rocm-smi is available in CI environment.

    if devices.is_empty() {
        Ok(vec![DeviceInfo::cpu_default()])
    } else {
        Ok(devices)
    }
}

fn detect_nvidia() -> Result<Vec<DeviceInfo>, DeviceDetectionError> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=index,name,memory.total")
        .arg("--format=csv,noheader,nounits")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output = match output {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            return Err(DeviceDetectionError::CommandFailure {
                command: "nvidia-smi",
                message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(err) => {
            return Err(DeviceDetectionError::CommandFailure {
                command: "nvidia-smi",
                message: err.to_string(),
            })
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.split(',').map(|p| p.trim());
        let ordinal = parts
            .next()
            .and_then(|p| p.parse::<usize>().ok())
            .unwrap_or(0);
        let name = parts.next().unwrap_or("NVIDIA GPU").to_string();
        let memory_mib = parts
            .next()
            .and_then(|p| p.parse::<u64>().ok())
            .unwrap_or(0);
        devices.push(DeviceInfo {
            id: format!("cuda:{ordinal}"),
            ordinal,
            name,
            kind: DeviceKind::NvidiaGpu,
            memory_total_bytes: memory_mib * 1024 * 1024,
        });
    }
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_devices() -> Vec<DeviceInfo> {
        vec![
            DeviceInfo {
                id: "cuda:0".into(),
                ordinal: 0,
                name: "GPU-0".into(),
                kind: DeviceKind::NvidiaGpu,
                memory_total_bytes: 8 * 1024 * 1024 * 1024,
            },
            DeviceInfo::cpu_default(),
        ]
    }

    #[test]
    fn gpu_first_prefers_gpu() {
        let manager = DeviceManager::from_devices(sample_devices());
        let plan = manager.allocate(DevicePreference::GpuFirst, 1);
        assert_eq!(plan.devices()[0].kind, DeviceKind::NvidiaGpu);
    }

    #[test]
    fn cpu_only_fallback() {
        let manager = DeviceManager::from_devices(sample_devices());
        let plan = manager.allocate(DevicePreference::CpuOnly, 2);
        assert!(plan.devices().iter().all(|d| d.kind == DeviceKind::Cpu));
    }

    #[test]
    fn explicit_selection() {
        let manager = DeviceManager::from_devices(sample_devices());
        let plan = manager.allocate(DevicePreference::Explicit(vec!["cuda:0".into()]), 1);
        assert_eq!(plan.devices()[0].id, "cuda:0");
    }
}
