"""Verify that each candidate Ollama Cloud model id is real and responds.

Run:
    OLLAMA_API_KEY=... python verify_models.py

Writes:
    data/processed/models_verified.json
    {
      "<id>": {"ok": bool, "latency_ms": float, "first_tokens": str, "error": null|str},
      ...
    }
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

import requests

OLLAMA_BASE_URL = os.environ.get("OLLAMA_BASE_URL", "https://ollama.com")
OLLAMA_API_KEY = os.environ.get("OLLAMA_API_KEY", "")

CANDIDATES = [
    "qwen3:235b-cloud",
    "qwen3-coder:480b-cloud",
    "deepseek-v3.1:671b-cloud",
    "deepseek-v3.2:cloud",
    "deepseek-v4-flash:cloud",
    "deepseek-v4-pro:cloud",
    "glm-5:cloud",
    "glm-5.1:cloud",
    "gemma3:27b-cloud",
    "gemma4:31b-cloud",
    "kimi-k2.6:cloud",
    "kimi-k2:1t-cloud",
    "ministral-3:8b-cloud",
    "nemotron-3-super:cloud",
    "mistral-large-3:cloud",
    "devstral-small-2:24b-cloud",
    "minimax-m2.7:cloud",
]


def probe(model: str, timeout: float = 30.0) -> dict:
    headers = {"Authorization": f"Bearer {OLLAMA_API_KEY}", "Content-Type": "application/json"}
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": "Reply with exactly the single word: pong"}],
        "temperature": 0,
        "stream": False,
    }
    t0 = time.perf_counter()
    try:
        r = requests.post(f"{OLLAMA_BASE_URL}/api/chat", headers=headers, json=payload, timeout=timeout)
        elapsed = (time.perf_counter() - t0) * 1000
        if r.status_code != 200:
            return {"ok": False, "latency_ms": elapsed, "first_tokens": "", "error": f"HTTP {r.status_code}: {r.text[:200]}"}
        data = r.json()
        content = data.get("message", {}).get("content") or data.get("response") or ""
        return {"ok": True, "latency_ms": elapsed, "first_tokens": content[:120], "error": None}
    except Exception as e:
        elapsed = (time.perf_counter() - t0) * 1000
        return {"ok": False, "latency_ms": elapsed, "first_tokens": "", "error": f"{type(e).__name__}: {e}"}


def main() -> int:
    if not OLLAMA_API_KEY:
        print("ERROR: OLLAMA_API_KEY not set", file=sys.stderr)
        return 2

    results: dict[str, dict] = {}
    for m in CANDIDATES:
        sys.stdout.write(f"probing {m:<35s} ... ")
        sys.stdout.flush()
        res = probe(m)
        results[m] = res
        if res["ok"]:
            sys.stdout.write(f"OK ({res['latency_ms']:.0f}ms) -> {res['first_tokens'][:60]!r}\n")
        else:
            sys.stdout.write(f"FAIL: {res['error'][:120]}\n")
        sys.stdout.flush()

    out = Path(__file__).resolve().parents[2] / "data" / "processed" / "models_verified.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(results, indent=2))
    print(f"\nWrote {out}")

    ok = [m for m, r in results.items() if r["ok"]]
    print(f"\n{len(ok)}/{len(CANDIDATES)} models reachable")
    print("Suggested shortlist (5 covering dense/MoE diversity):")
    for m in ok[:10]:
        print(f"  - {m}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
