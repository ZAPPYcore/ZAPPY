#!/usr/bin/env python3
"""
Environment diagnostic utility for the Tier-10 AGI stack.

The script inspects GPU/CPU availability, CUDA/ROCm installations, Python/PyTorch
versions, and emits a JSON report. Use it before launching heavy training jobs
or when provisioning new nodes.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Optional


def _run(cmd: list[str]) -> Optional[str]:
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, check=True  # noqa: S603,S607
        )
        return result.stdout.strip()
    except FileNotFoundError:
        return None
    except subprocess.CalledProcessError as exc:  # pragma: no cover - diagnostics only
        return (exc.stdout or exc.stderr or "").strip()


def _detect_nvidia() -> Dict[str, Any]:
    nvidia_smi = shutil.which("nvidia-smi")
    if not nvidia_smi:
        return {"present": False}
    raw = _run([nvidia_smi, "--query-gpu=name,driver_version,memory.total", "--format=csv,noheader"])
    gpus = []
    driver_version = None
    if raw:
        for line in raw.splitlines():
            parts = [part.strip() for part in line.split(",")]
            if len(parts) >= 3:
                gpus.append({"name": parts[0], "memory_total": parts[2]})
                driver_version = parts[1]
    return {
        "present": True,
        "driver_version": driver_version,
        "gpus": gpus,
    }


def _detect_rocm() -> Dict[str, Any]:
    rocm_smi = shutil.which("rocm-smi")
    if not rocm_smi:
        return {"present": False}
    raw = _run([rocm_smi, "-i"])
    return {"present": True, "raw_output": raw}


def _torch_info() -> Dict[str, Any]:
    info: Dict[str, Any] = {"installed": False}
    try:
        import torch  # type: ignore

        info["installed"] = True
        info["version"] = torch.__version__
        info["cuda_available"] = torch.cuda.is_available()
        if torch.cuda.is_available():
            info["cuda_device_count"] = torch.cuda.device_count()
            info["cuda_capabilities"] = [
                torch.cuda.get_device_capability(i) for i in range(torch.cuda.device_count())
            ]
            info["cuda_device_names"] = [torch.cuda.get_device_name(i) for i in range(torch.cuda.device_count())]
        info["cuda_version"] = torch.version.cuda
        info["cudnn_version"] = torch.backends.cudnn.version() if torch.backends.cudnn.is_available() else None
    except Exception as exc:  # pragma: no cover - info only
        info["error"] = repr(exc)
    return info


def build_report() -> Dict[str, Any]:
    report: Dict[str, Any] = {
        "host": platform.node(),
        "os": {
            "platform": platform.platform(),
            "python": sys.version.split()[0],
        },
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "nvidia": _detect_nvidia(),
        "rocm": _detect_rocm(),
        "torch": _torch_info(),
    }
    gpu_present = report["nvidia"].get("present") or report["rocm"].get("present")
    torch_cuda = report["torch"].get("cuda_available")
    if gpu_present and torch_cuda:
        report["recommended_action"] = "ok"
    elif gpu_present and not torch_cuda:
        report["recommended_action"] = "install_torch_cuda_build"
    else:
        report["recommended_action"] = "gpu_not_detected"
    return report


def main() -> None:
    parser = argparse.ArgumentParser(description="Emit environment diagnostics as JSON")
    parser.add_argument(
        "--output",
        type=Path,
        help="Optional path to write the JSON report (defaults to stdout only)",
    )
    args = parser.parse_args()

    report = build_report()
    json_payload = json.dumps(report, indent=2)
    print(json_payload)

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json_payload + os.linesep, encoding="utf-8")


if __name__ == "__main__":
    main()

