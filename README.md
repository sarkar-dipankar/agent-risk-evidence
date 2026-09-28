# Agent Risk Evidence

Research artifact for **Insuring Agentic Processes: Measurement, Pooling, and Attestation for AI-Agent Risk**, by **Dipankar Sarkar**.

The paper studies three components separately: single-turn model-prompt elicitation, synthetic portfolio aggregation, and Ed25519/Merkle batch attestation. It does not establish deployment incident rates, an actuarial pricing model, end-to-end differential privacy, or authenticated event timestamps.

## Contents

- `paper/`: corrected arXiv manuscript and PDF.
- `acsac2026-paper/experiments/`: original prompt files, recorded responses, judge labels, result tables, and experiment drivers.
- `acsac2026-paper/data/processed/`: recorded API-model verification information.
- `agent-insure-core/`: curated Rust workspace containing the four crates needed by the privacy and attestation experiments.
- `src/agent_insure/pipeline/attestation.py`: Python reference implementation.
- `validate_recorded_results.py`: offline validation of the 60 recorded cells, bootstrap estimates, and inter-judge agreement.
- `VALIDATION.md`: checks performed and corrections supported by the released code/data.

## Verification

```sh
python -m pip install numpy
python validate_recorded_results.py
cd agent-insure-core
cargo test -p ai-attestation
```

The validation script replays analysis of saved outputs. It does not collect new model responses. Original experiment scripts may write over recorded outputs; use a separate working copy for new runs and distinguish their results from the archived campaign. API experiments require the credentials and parameters specified by their scripts.

The source packages declare Apache-2.0 for code; that license is retained here. Prompt provenance and third-party rights remain as identified by the original files. No raw private organizational portfolio is included: the pooling benchmark is synthetic.

## Provenance

Curated from source commit `b49fba81d1dde8c02dff49360f93a3018cec5cff`. Experiment files and result data are retained without changes, except for one redaction.

## Redacted responses

Some prompts asked models to reproduce copyrighted text verbatim: song lyrics, newspaper articles, scripts, and book passages. Where a model complied, the recorded response contains that text. To avoid redistributing it, the `response` field is redacted for every `ip_infringement` row and for model-extraction prompts ME-001, ME-002, ME-005 and ME-011, in `results.jsonl`, `raw_results.json` and both pilot files (360 main-campaign rows, 25 pilot rows).

Each redacted value reads `[REDACTED: copyrighted-content response removed; chars=<N> sha256=<hash>]`, where `N` and `hash` are the original response's length and SHA-256. Judge labels, confidences and rationales are unchanged, so every reported rate, bootstrap estimate and kappa value still replays from these files. Packaging narrows the Rust workspace to the four required crates and provides minimal Python package initializers. The manuscript and this release documentation contain the corrections described in `VALIDATION.md`.
