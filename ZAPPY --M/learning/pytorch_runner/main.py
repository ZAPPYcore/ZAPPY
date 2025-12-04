from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import List, Optional

if __package__:
    from .config import TrainingConfig
    from .trainer import RunnerContext, TrainingSession
else:  # pragma: no cover - executed when run as script path
    from config import TrainingConfig
    from trainer import RunnerContext, TrainingSession


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Tier-10 PyTorch runner")
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--weights", type=Path)
    parser.add_argument("--devices", type=str, default="")
    parser.add_argument("--profile", type=str, default="prod")
    parser.add_argument("--log-file", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    config = TrainingConfig.from_file(args.config)
    devices = [token for token in args.devices.split(",") if token]
    ctx = RunnerContext(
        config=config,
        devices=devices,
        weights=args.weights,
        profile=args.profile,
        log_file=args.log_file,
    )
    session = TrainingSession(ctx)
    session.run()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

