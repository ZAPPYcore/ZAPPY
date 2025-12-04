from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Sequence

if __package__:
    from .config import TrainingConfig
else:  # pragma: no cover - executed when run as script
    from config import TrainingConfig

try:
    import torch

    TORCH_AVAILABLE = True
except Exception:  # pragma: no cover - optional dependency
    torch = None
    TORCH_AVAILABLE = False


@dataclass
class RunnerContext:
    config: TrainingConfig
    devices: Sequence[str]
    weights: Optional[Path]
    profile: str
    log_file: Path


class TrainingSession:
    """Executes a training session using PyTorch when available."""

    def __init__(self, ctx: RunnerContext):
        self.ctx = ctx
        self.ctx.log_file.parent.mkdir(parents=True, exist_ok=True)

    def run(self) -> None:
        self._log("session_start", {"profile": self.ctx.profile, "devices": self.ctx.devices})
        if TORCH_AVAILABLE:
            self._run_torch()
        else:
            self._simulate()
        self._log("session_complete", {})

    def _run_torch(self) -> None:
        assert torch is not None  # mypy quiet
        device = torch.device(self._select_torch_device())
        torch.manual_seed(42)
        model = torch.nn.Sequential(
            torch.nn.Linear(self.ctx.config.input_dim, self.ctx.config.hidden_dim),
            torch.nn.ReLU(),
            torch.nn.Linear(self.ctx.config.hidden_dim, self.ctx.config.output_dim),
        ).to(device)
        optimizer = torch.optim.Adam(model.parameters(), lr=self.ctx.config.learning_rate)
        loss_fn = torch.nn.MSELoss()

        for step in range(self.ctx.config.steps):
            batch = self._fake_batch(self.ctx.config.batch_size, self.ctx.config.input_dim)
            targets = torch.zeros(self.ctx.config.batch_size, self.ctx.config.output_dim).to(device)
            optimizer.zero_grad()
            outputs = model(batch.to(device))
            loss = loss_fn(outputs, targets)
            loss.backward()
            optimizer.step()
            if step % 10 == 0:
                self._log("step", {"step": step, "loss": float(loss.detach().cpu().item())})

        self._write_checkpoint()

    def _simulate(self) -> None:
        loss = 1.0
        for step in range(self.ctx.config.steps):
            loss *= 0.99
            if step % 10 == 0:
                self._log("step", {"step": step, "loss": loss, "simulated": True})
            time.sleep(0.01)
        self._write_checkpoint()

    def _write_checkpoint(self) -> None:
        meta = {
            "id": f"checkpoint-{int(time.time())}",
            "model": "pytorch_runner_stub",
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "training_step": self.ctx.config.steps,
            "devices": list(self.ctx.devices),
            "notes": self.ctx.config.notes,
        }
        self.ctx.config.checkpoint_dir.mkdir(parents=True, exist_ok=True)
        meta_path = self.ctx.config.checkpoint_dir / f"{meta['id']}.meta.json"
        with meta_path.open("w", encoding="utf-8") as f:
            json.dump(meta, f, indent=2)
        self._log("checkpoint", {"meta": str(meta_path)})

    def _fake_batch(self, batch_size: int, dim: int):
        assert torch is not None
        data = torch.randn(batch_size, dim)
        return data

    def _select_torch_device(self) -> str:
        if self.ctx.devices:
            preferred = self.ctx.devices[0]
            if preferred.startswith("cuda") and torch.cuda.is_available():
                ordinal = preferred.split(":")[1] if ":" in preferred else "0"
                os.environ["CUDA_VISIBLE_DEVICES"] = ordinal
                return "cuda:0"
        return "cpu"

    def _log(self, event: str, payload: dict) -> None:
        record = {
            "event": event,
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "payload": payload,
        }
        with self.ctx.log_file.open("a", encoding="utf-8") as f:
            json.dump(record, f)
            f.write("\n")

