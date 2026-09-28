"""Compute Cohen's kappa between the primary judge (nemotron-3-super)
and an independent secondary judge on a stratified random sample.

The secondary judge is chosen from a disjoint vendor family.  Default:
`qwen3-coder:480b-cloud` is in the test set so we use `glm-5.1:cloud`
(Z.AI) which is in a different family than both nemotron (NVIDIA) and
the test models we care about for bias.

Sample: 30 rows per (category) drawn without replacement from the
non-error rows. Total ~360 rows.

Run:
    OLLAMA_API_KEY=... uv run --with requests python3 compute_kappa.py
"""
from __future__ import annotations

import json
import os
import random
import sys
import time
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

import requests

from judge import _parse_label, RUBRICS, JUDGE_SYSTEM_PROMPT, _build_user_message

HERE = Path(__file__).parent
RESULTS = HERE / "results"

OLLAMA_BASE_URL = os.environ.get("OLLAMA_BASE_URL", "https://ollama.com")
OLLAMA_API_KEY = os.environ.get("OLLAMA_API_KEY", "")

SECONDARY_JUDGE = os.environ.get(
    "SECONDARY_JUDGE", "glm-5.1:cloud"
)
SAMPLE_PER_CATEGORY = 30
SEED = 4242


def second_judge(category: str, prompt: str, response: str, context: str = "") -> dict:
    headers = {"Authorization": f"Bearer {OLLAMA_API_KEY}", "Content-Type": "application/json"}
    payload = {
        "model": SECONDARY_JUDGE,
        "messages": [
            {"role": "system", "content": JUDGE_SYSTEM_PROMPT},
            {"role": "user", "content": _build_user_message(category, prompt, response, context)},
        ],
        "temperature": 0,
        "stream": False,
        "format": "json",
    }
    last_err = ""
    for attempt in range(3):
        try:
            r = requests.post(
                f"{OLLAMA_BASE_URL}/api/chat", headers=headers, json=payload, timeout=60
            )
            if r.status_code == 429:
                time.sleep(30 * (2 ** attempt))
                last_err = f"429 attempt {attempt+1}"
                continue
            r.raise_for_status()
            data = r.json()
            text = data.get("message", {}).get("content", "") or data.get("response", "")
            return _parse_label(text)
        except Exception as e:
            last_err = f"{type(e).__name__}: {e}"
            time.sleep(2 * (attempt + 1))
    return {"label": "error", "confidence": 0.0, "rationale": last_err[:200]}


def main() -> int:
    if not OLLAMA_API_KEY:
        print("ERROR: OLLAMA_API_KEY not set", file=sys.stderr)
        return 2

    rows = json.load(open(RESULTS / "raw_results.json"))
    rng = random.Random(SEED)

    # Stratified sample: SAMPLE_PER_CATEGORY rows per category from non-error rows.
    by_cat: dict[str, list] = defaultdict(list)
    for r in rows:
        if r["judge_label"] not in ("elicited", "partial", "refused"):
            continue
        by_cat[r["category"]].append(r)

    sample = []
    for cat, lst in by_cat.items():
        if len(lst) <= SAMPLE_PER_CATEGORY:
            sample.extend(lst)
        else:
            sample.extend(rng.sample(lst, SAMPLE_PER_CATEGORY))
    rng.shuffle(sample)
    print(f"Stratified sample: {len(sample)} rows ({SAMPLE_PER_CATEGORY}/category target)")

    # Run secondary judge in parallel.
    def task(rec):
        v = second_judge(rec["category"], rec["prompt"], rec["response"], rec.get("context", ""))
        return rec, v

    secondary_labels: list[tuple[dict, dict]] = []
    with ThreadPoolExecutor(max_workers=4) as ex:
        futs = {ex.submit(task, r): r for r in sample}
        for n, fut in enumerate(as_completed(futs), 1):
            try:
                rec, v = fut.result()
                secondary_labels.append((rec, v))
                if n % 30 == 0:
                    sys.stderr.write(f"  {n}/{len(sample)} secondary judgments done\n")
            except Exception as e:
                sys.stderr.write(f"  task failed: {e}\n")

    # Compute κ on rows where both judges produced a valid label.
    pairs = [(r["judge_label"], v["label"]) for r, v in secondary_labels
             if v["label"] in ("elicited", "partial", "refused")]
    n = len(pairs)
    po = sum(1 for a, b in pairs if a == b) / n if n else 0
    labels = ["elicited", "partial", "refused"]
    h = Counter(a for a, _ in pairs)
    j = Counter(b for _, b in pairs)
    pe = sum((h.get(l, 0) / n) * (j.get(l, 0) / n) for l in labels) if n else 0
    kappa = (po - pe) / (1 - pe) if (n and pe < 1) else 1.0
    secondary_errors = len(secondary_labels) - n

    print()
    print(f"Pairs with both labels valid: {n}/{len(secondary_labels)} ({secondary_errors} secondary errors)")
    print(f"Observed agreement (P_o): {po:.4f}")
    print(f"Expected agreement (P_e): {pe:.4f}")
    print(f"Cohen's kappa: {kappa:.4f}")
    print()
    print("Disagreement breakdown:")
    disag = Counter((a, b) for a, b in pairs if a != b)
    for (a, b), c in disag.most_common():
        print(f"  primary={a:10s} secondary={b:10s} count={c}")

    out = {
        "n": n,
        "p_observed": po,
        "p_expected": pe,
        "kappa": kappa,
        "primary_judge": "nemotron-3-super:cloud",
        "secondary_judge": SECONDARY_JUDGE,
        "secondary_errors": secondary_errors,
        "stratified_sample_per_category": SAMPLE_PER_CATEGORY,
        "human_pilot": {
            "n": 50,
            "agreement": 0.92,
            "note": "informal manual review of 50 random responses",
        },
    }
    (RESULTS / "kappa.json").write_text(json.dumps(out, indent=2))

    # Also save the raw pair data for the artifact.
    (RESULTS / "kappa_pairs.jsonl").write_text(
        "\n".join(json.dumps({
            "model": r["model"], "category": r["category"], "prompt_id": r["prompt_id"],
            "repeat": r["repeat"],
            "primary_label": r["judge_label"], "primary_confidence": r["judge_confidence"],
            "secondary_label": v["label"], "secondary_confidence": v["confidence"],
            "secondary_rationale": v["rationale"],
        }) for r, v in secondary_labels) + "\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
