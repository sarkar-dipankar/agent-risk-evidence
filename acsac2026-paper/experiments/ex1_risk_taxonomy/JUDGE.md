# Judge Model Choice (Experiment 1)

## Selection

We use **`deepseek-v3.1:671b-cloud`** as the LLM-as-judge for response
classification in Experiment 1. Rationale:

1. **Not in the test set.** The five models under test
   (`qwen3:235b-cloud`, `deepseek-v4-flash`-class variant if tested,
   `glm-5:cloud`, `gemma3:27b-cloud`, `kimi-k2.6:cloud`) do not overlap
   with `deepseek-v3.1:671b-cloud`. We intentionally avoid using a
   newer DeepSeek model (e.g., `deepseek-v4-pro`) for the same vendor
   family as one of the test models to keep judge/judged decoupled.
2. **High capability.** 671B-class reasoning model with strong
   instruction-following, JSON-mode support, and tool-calling
   familiarity. Reduces judge error on subtle category boundaries.
3. **Accessible via the same Ollama Cloud endpoint** used for the test
   models, keeping latency and infrastructure comparable.

If `deepseek-v3.1:671b-cloud` is unavailable when the experiment runs,
the fallback is the next-strongest non-test model (e.g., a top-tier
external API model such as Claude Opus or GPT-4o). The judge identity
is recorded in every results row so any swap is fully traceable.

## Rubric

Per-category rubrics are defined in `judge.py:RUBRICS`. Each rubric:

- Defines the failure mode in one sentence.
- Specifies the three labels (`elicited`, `partial`, `refused`).
- Names concrete observable cues for each label.

The judge is prompted with a system instruction to output JSON only,
deterministic temperature 0, and `format: json` enforced by the
Ollama API so output is parseable.

## Validation

We sample 10% of all (model × category × prompt × repeat) cells for
human re-labeling by two paper co-authors. Disagreements are arbitrated
by a third co-author. We report Cohen's κ between judge and consensus
human labels in §5.1 of the paper. Target κ ≥ 0.6 (substantial
agreement, Landis & Koch 1977).

## Cost / runtime estimate

- 5 test models × 12 categories × 50 prompts × 3 repeats = **9,000** test calls.
- Plus 9,000 judge calls (one per test response). Total ~18,000 API calls.
- Median call latency ~1–3 s; at workers=10, end-to-end ≈ 60–90 min.
- Headroom for retries on timeouts: ~2× wall-clock.
