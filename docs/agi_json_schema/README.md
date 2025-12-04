# AGI JSON Schemas

This directory defines the canonical JSON schemas used across the AGI
stack. They document how datasets, models, and checkpoints must be described so
that any module (Rust, Python, or other languages) can reason about assets
without bespoke adapters.

## Contents

| File | Purpose |
| --- | --- |
| `dataset.schema.json` | Schema for dataset descriptors (shards, features, index). |
| `model.schema.json` | Schema for model definitions (architecture, sharding, limits). |
| `checkpoint_meta.schema.json` | Schema describing persisted checkpoints. |

## Usage

1. Validate JSON files using `jsonschema` or any compatible validator.
2. Embed the schema `$id` in produced JSON for traceability.
3. Store dataset/model metadata alongside the associated binary payloads (see
   `learning/README.md` for directory layout guidelines).

Example validation (Python):

```bash
pip install jsonschema
python -m jsonschema --instance /learning/datasets/foo/schema.json docs/agi_json_schema/dataset.schema.json
```


