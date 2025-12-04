from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional


@dataclass
class TrainingConfig:
    """Configuration describing a training run."""

    dataset_index: Path
    checkpoint_dir: Path
    steps: int
    batch_size: int
    learning_rate: float
    input_dim: int
    output_dim: int
    hidden_dim: int
    notes: Optional[str] = None

    @classmethod
    def from_file(cls, path: Path) -> "TrainingConfig":
        data = cls._load_json(path)
        return cls(
            dataset_index=cls._resolve(path, data["dataset_index"]),
            checkpoint_dir=cls._resolve(path, data["checkpoint_dir"]),
            steps=int(data.get("steps", 100)),
            batch_size=int(data.get("batch_size", 32)),
            learning_rate=float(data.get("learning_rate", 1e-3)),
            input_dim=int(data.get("input_dim", 128)),
            output_dim=int(data.get("output_dim", 64)),
            hidden_dim=int(data.get("hidden_dim", 128)),
            notes=data.get("notes"),
        )

    @staticmethod
    def _load_json(path: Path) -> Dict[str, Any]:
        with path.open("r", encoding="utf-8") as f:
            return json.load(f)

    @staticmethod
    def _resolve(base: Path, value: str) -> Path:
        path = Path(value)
        if path.is_absolute():
            return path
        return (base.parent / path).resolve()

