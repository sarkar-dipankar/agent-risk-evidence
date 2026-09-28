"""Drive both Python and Rust attestation benchmarks, join results into a
single CSV the paper consumes.

Outputs:
    results/attestation_python.csv  -- median times from the Python harness
    results/attestation_rust.csv    -- parsed from criterion JSON
    results/attestation_combined.csv  -- long format with `implementation` column
"""
from __future__ import annotations

import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).parent
RESULTS = HERE / "results"
RESULTS.mkdir(parents=True, exist_ok=True)
WORKSPACE = HERE.resolve().parents[2] / "agent-insure-core"

sys.path.insert(0, str(HERE.resolve().parents[2] / "src"))
from agent_insure.pipeline.attestation import AttestationManager  # type: ignore  # noqa: E402

BATCH_SIZES = [10, 100, 1000, 10_000, 100_000, 1_000_000]
ITERATIONS = 5  # Python is slow at 1e6; median of 5 is enough for the paper.


# ----------------- Python -----------------
def py_runs() -> list[dict]:
    mgr = AttestationManager(*AttestationManager.generate_keypair())
    rows: list[dict] = []
    for bs in BATCH_SIZES:
        records = [{"id": i, "data": "x" * 100} for i in range(bs)]
        # signing
        times = []
        for _ in range(ITERATIONS):
            t = time.perf_counter()
            manifest = mgr.create_manifest(records, "1.0.0", "1.0.0")
            times.append(time.perf_counter() - t)
        m = statistics.median(times) * 1000
        rows.append({"impl": "python", "op": "sign", "batch": bs,
                     "median_ms": m, "throughput_rec_s": bs / (m / 1000)})
        # verification
        times = []
        for _ in range(ITERATIONS):
            t = time.perf_counter()
            mgr.verify_manifest(manifest, records)
            times.append(time.perf_counter() - t)
        m = statistics.median(times) * 1000
        rows.append({"impl": "python", "op": "verify", "batch": bs,
                     "median_ms": m, "throughput_rec_s": bs / (m / 1000)})
        # merkle
        hashes = [mgr._hash_record(r) for r in records]
        times = []
        for _ in range(ITERATIONS):
            t = time.perf_counter()
            mgr._compute_merkle_root(hashes)
            times.append(time.perf_counter() - t)
        m = statistics.median(times) * 1000
        rows.append({"impl": "python", "op": "merkle", "batch": bs,
                     "median_ms": m, "throughput_rec_s": bs / (m / 1000) if m > 0 else 0.0})
        # tamper detection: verify with modified mid-record (full re-hash path)
        tampered = list(records)
        tampered[bs // 2] = {**records[bs // 2], "data": "tampered-xxxxxxxxxxxxxxxxxxx"}
        times = []
        for _ in range(ITERATIONS):
            t = time.perf_counter()
            mgr.verify_manifest(manifest, tampered)
            times.append(time.perf_counter() - t)
        m = statistics.median(times) * 1000
        rows.append({"impl": "python", "op": "tamper", "batch": bs,
                     "median_ms": m, "throughput_rec_s": 0.0})
        print(f"  python bs={bs}: done")
    return rows


# ----------------- Rust (criterion JSON -> rows) -----------------
def rust_runs() -> list[dict]:
    """Parse criterion's per-benchmark estimates.json files written under
    target/criterion/.

    Tolerant to missing directories: returns whatever it can find. Run
    `cargo bench -p ai-attestation --bench attest` before calling this.
    """
    bench_root = WORKSPACE / "target" / "criterion"
    if not bench_root.exists():
        print(f"  WARNING: {bench_root} not found; run cargo bench first", file=sys.stderr)
        return []
    rows: list[dict] = []
    # criterion writes one estimates.json per (benchmark, parameter).
    # The directory layout is: <bench>/<param>/new/estimates.json
    for est in bench_root.rglob("new/estimates.json"):
        parts = est.parts
        # Parametrized benchmarks: .../<bench>/<param>/new/estimates.json
        # Non-parametrized: .../<bench>/new/estimates.json
        bench = None
        batch = 0
        if len(parts) >= 4 and parts[-2] == "new":
            try:
                batch = int(parts[-3])
                bench = parts[-4]
            except (ValueError, IndexError):
                bench = parts[-3]
                batch = 0
        if bench is None:
            continue
        data = json.loads(est.read_text())
        median = data.get("median", {}).get("point_estimate", None)
        if median is None:
            continue
        median_ms = median / 1e6
        throughput = batch / (median / 1e9) if median > 0 and batch > 0 else 0.0
        op = bench.lower()
        if op not in ("sign", "verify", "merkle_build", "tamper_detection", "inclusion_proof"):
            continue
        rows.append({
            "impl": "rust",
            "op": op.replace("_build", "").replace("_detection", ""),
            "batch": batch,
            "median_ms": median_ms,
            "throughput_rec_s": throughput,
        })
    return rows


def main() -> int:
    print("== Python ==")
    py_rows = py_runs()
    py_csv = RESULTS / "attestation_python.csv"
    with py_csv.open("w") as f:
        f.write("impl,op,batch,median_ms,throughput_rec_s\n")
        for r in py_rows:
            f.write(f"{r['impl']},{r['op']},{r['batch']},{r['median_ms']:.4f},{r['throughput_rec_s']:.0f}\n")
    print(f"Wrote {py_csv}")

    print("== Rust (parsing criterion output) ==")
    rust_rows = rust_runs()
    rust_csv = RESULTS / "attestation_rust.csv"
    with rust_csv.open("w") as f:
        f.write("impl,op,batch,median_ms,throughput_rec_s\n")
        for r in rust_rows:
            f.write(f"{r['impl']},{r['op']},{r['batch']},{r['median_ms']:.4f},{r['throughput_rec_s']:.0f}\n")
    print(f"Wrote {rust_csv}")

    combined = RESULTS / "attestation_combined.csv"
    with combined.open("w") as f:
        f.write("impl,op,batch,median_ms,throughput_rec_s\n")
        for r in py_rows + rust_rows:
            f.write(f"{r['impl']},{r['op']},{r['batch']},{r['median_ms']:.4f},{r['throughput_rec_s']:.0f}\n")
    print(f"Wrote {combined}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
