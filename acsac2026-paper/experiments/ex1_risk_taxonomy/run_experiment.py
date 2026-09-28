"""Agent Insure: Risk Taxonomy Validation (Experiment 1).

Runs N models x 12 categories x 50 prompts (x R repeats) against Ollama Cloud,
classifies every response via an LLM-as-judge (different model), and writes
results.jsonl checkpoint + raw_results.json for analysis.

Usage:
    OLLAMA_API_KEY=... \
    JUDGE_MODEL=deepseek-v3.1:671b-cloud \
    python run_experiment.py --repeats 3 --workers 10

Resume after interruption:
    python run_experiment.py --resume
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import asdict, dataclass
from pathlib import Path

import requests
from tqdm import tqdm

from judge import judge_response, JUDGE_MODEL

OLLAMA_API_KEY = os.environ.get("OLLAMA_API_KEY", "")
OLLAMA_BASE_URL = os.environ.get("OLLAMA_BASE_URL", "https://ollama.com")

# Production models under test (verified reachable via verify_models.py).
# Chosen for vendor + architecture diversity:
DEFAULT_MODELS = [
    "qwen3-coder:480b-cloud",       # Alibaba, MoE 480B
    "deepseek-v4-pro:cloud",        # DeepSeek, MoE flagship
    "glm-5:cloud",                  # Z.AI, MoE
    "gemma3:27b-cloud",             # Google, Dense 27B
    "kimi-k2.6:cloud",              # Moonshot, agentic
]

CATEGORIES = [
    "prompt_injection_direct",
    "prompt_injection_indirect",
    "jailbreak",
    "in_context_injection",
    "model_extraction",
    "hallucination_financial",
    "hallucination_legal",
    "tool_misuse",
    "context_failure",
    "resource_exhaustion",
    "harmful_content",
    "ip_infringement",
]

HERE = Path(__file__).parent
RESULTS_DIR = HERE / "results"
PROMPTS_DIR = HERE / "prompts"
CHECKPOINT = RESULTS_DIR / "results.jsonl"


@dataclass
class TestResult:
    model: str
    category: str
    prompt_id: str
    repeat: int
    prompt: str
    context: str
    response: str
    judge_label: str
    judge_confidence: float
    judge_rationale: str
    latency_ms: float
    judge_latency_ms: float
    timestamp: str


def load_prompts(category: str) -> list[dict]:
    p = PROMPTS_DIR / f"{category}.json"
    if not p.exists():
        return []
    with open(p) as f:
        return json.load(f)


def call_ollama(model: str, prompt: str, context: str = "", timeout: float = 60.0,
                max_retries: int = 4) -> tuple[str, float]:
    """Call Ollama Cloud with retry-on-429 and exponential backoff.

    Backoff schedule (cumulative wall-clock for retries): 30s, 60s, 120s, 240s.
    """
    headers = {"Authorization": f"Bearer {OLLAMA_API_KEY}", "Content-Type": "application/json"}
    messages = []
    if context:
        messages.append({"role": "user", "content": f"[Reference document]\n{context}\n[End reference]"})
    messages.append({"role": "user", "content": prompt})
    payload = {"model": model, "messages": messages, "temperature": 0, "stream": False}

    t0 = time.perf_counter()
    last_err = ""
    for attempt in range(max_retries + 1):
        try:
            r = requests.post(
                f"{OLLAMA_BASE_URL}/api/chat", headers=headers, json=payload, timeout=timeout
            )
            if r.status_code == 429:
                # Rate-limited: back off and retry.
                wait = 30 * (2 ** attempt)
                last_err = f"429 (attempt {attempt+1}/{max_retries+1}), sleeping {wait}s"
                time.sleep(wait)
                continue
            elapsed = (time.perf_counter() - t0) * 1000
            r.raise_for_status()
            data = r.json()
            return (data.get("message", {}).get("content") or data.get("response") or ""), elapsed
        except requests.exceptions.HTTPError as e:
            if e.response is not None and e.response.status_code == 429:
                wait = 30 * (2 ** attempt)
                last_err = f"429 HTTPError (attempt {attempt+1}/{max_retries+1}), sleeping {wait}s"
                time.sleep(wait)
                continue
            elapsed = (time.perf_counter() - t0) * 1000
            return f"ERROR: {type(e).__name__}: {e}", elapsed
        except Exception as e:
            elapsed = (time.perf_counter() - t0) * 1000
            # Single retry on timeout/conn errors.
            if attempt < 1 and ("Timeout" in type(e).__name__ or "Connection" in type(e).__name__):
                time.sleep(5)
                continue
            return f"ERROR: {type(e).__name__}: {e}", elapsed
    elapsed = (time.perf_counter() - t0) * 1000
    return f"ERROR: RateLimitExceeded: {last_err}", elapsed


def run_one(model: str, category: str, prompt_data: dict, repeat: int) -> TestResult:
    prompt = prompt_data["prompt"]
    context = prompt_data.get("context", "") or ""
    response, latency = call_ollama(model, prompt, context)

    if response.startswith("ERROR:"):
        judge_label, judge_conf, judge_rationale, judge_latency = "error", 0.0, response, 0.0
    else:
        t0 = time.perf_counter()
        verdict = judge_response(category, prompt, response, context)
        judge_latency = (time.perf_counter() - t0) * 1000
        judge_label = verdict["label"]
        judge_conf = verdict["confidence"]
        judge_rationale = verdict["rationale"]

    return TestResult(
        model=model,
        category=category,
        prompt_id=prompt_data["id"],
        repeat=repeat,
        prompt=prompt,
        context=context,
        response=response,
        judge_label=judge_label,
        judge_confidence=judge_conf,
        judge_rationale=judge_rationale,
        latency_ms=latency,
        judge_latency_ms=judge_latency,
        timestamp=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    )


def already_done(checkpoint_path: Path) -> set[tuple[str, str, str, int]]:
    done: set[tuple[str, str, str, int]] = set()
    if not checkpoint_path.exists():
        return done
    with open(checkpoint_path) as f:
        for line in f:
            try:
                rec = json.loads(line)
                done.add((rec["model"], rec["category"], rec["prompt_id"], rec.get("repeat", 0)))
            except Exception:
                continue
    return done


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--models", nargs="*", default=None,
                    help="Override the default model list")
    ap.add_argument("--categories", nargs="*", default=None,
                    help="Limit to specific categories")
    ap.add_argument("--repeats", type=int, default=3,
                    help="Number of repeat trials per (model, prompt)")
    ap.add_argument("--workers", type=int, default=10,
                    help="Parallel request workers")
    ap.add_argument("--limit-per-category", type=int, default=None,
                    help="Use only N prompts per category (for piloting)")
    ap.add_argument("--resume", action="store_true",
                    help="Skip work already in results.jsonl")
    args = ap.parse_args()

    if not OLLAMA_API_KEY:
        print("ERROR: OLLAMA_API_KEY not set", file=sys.stderr)
        return 2

    models = args.models or DEFAULT_MODELS
    categories = args.categories or CATEGORIES
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    done = already_done(CHECKPOINT) if args.resume else set()
    print(f"Resuming with {len(done)} cells already completed.")

    tasks = []
    for model in models:
        for category in categories:
            prompts = load_prompts(category)
            if not prompts:
                print(f"WARNING: no prompts for {category}", file=sys.stderr)
                continue
            if args.limit_per_category:
                prompts = prompts[: args.limit_per_category]
            for prompt in prompts:
                for repeat in range(args.repeats):
                    key = (model, category, prompt["id"], repeat)
                    if key in done:
                        continue
                    tasks.append((model, category, prompt, repeat))

    print(f"Models: {models}")
    print(f"Categories: {len(categories)}")
    print(f"Judge: {JUDGE_MODEL}")
    print(f"Total cells to run: {len(tasks)} (workers={args.workers})")

    with open(CHECKPOINT, "a") as ckpt:
        with ThreadPoolExecutor(max_workers=args.workers) as ex:
            futures = {ex.submit(run_one, m, c, p, r): (m, c, p["id"], r) for (m, c, p, r) in tasks}
            with tqdm(total=len(futures), desc="Tests") as bar:
                for fut in as_completed(futures):
                    try:
                        res = fut.result()
                    except Exception as e:
                        m, c, pid, r = futures[fut]
                        sys.stderr.write(f"FATAL on ({m},{c},{pid},{r}): {e}\n")
                        bar.update(1)
                        continue
                    ckpt.write(json.dumps(asdict(res)) + "\n")
                    ckpt.flush()
                    bar.update(1)

    # Consolidate checkpoint into raw_results.json
    rows = []
    with open(CHECKPOINT) as f:
        for line in f:
            try:
                rows.append(json.loads(line))
            except Exception:
                pass
    with open(RESULTS_DIR / "raw_results.json", "w") as f:
        json.dump(rows, f, indent=2)
    print(f"Wrote {len(rows)} rows to {RESULTS_DIR/'raw_results.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
