"""
PyTorch runner package for the Tier-10 AGI learning stack.

This package is invoked by the `trn` CLI and is responsible for validating
training configs, orchestrating PyTorch runs (or simulations when CUDA/PyTorch
is unavailable), and emitting structured JSON logs/checkpoints.
"""

