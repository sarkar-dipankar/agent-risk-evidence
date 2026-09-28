"""Benchmark Ed25519 attestation performance."""
import time
import json
import statistics
from pathlib import Path

import sys
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "src"))

from agent_insure.pipeline.attestation import AttestationManager


def benchmark_signing(batch_sizes: list[int], iterations: int = 10) -> dict:
    manager = AttestationManager(AttestationManager.generate_keypair()[0])
    results = {}
    for batch_size in batch_sizes:
        records = [{"id": i, "data": "x" * 100} for i in range(batch_size)]
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            manifest = manager.create_manifest(records, "1.0.0", "1.0.0")
            elapsed = time.perf_counter() - start
            times.append(elapsed)
        median_time = statistics.median(times)
        throughput = batch_size / median_time
        results[batch_size] = {
            "median_time_ms": median_time * 1000,
            "throughput_rec_per_sec": throughput,
        }
    return results


def benchmark_verification(batch_sizes: list[int], iterations: int = 10) -> dict:
    manager = AttestationManager(AttestationManager.generate_keypair()[0])
    results = {}
    for batch_size in batch_sizes:
        records = [{"id": i, "data": "x" * 100} for i in range(batch_size)]
        manifest = manager.create_manifest(records, "1.0.0", "1.0.0")
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            valid = manager.verify_manifest(manifest, records)
            elapsed = time.perf_counter() - start
            times.append(elapsed)
        median_time = statistics.median(times)
        throughput = batch_size / median_time
        results[batch_size] = {
            "median_time_ms": median_time * 1000,
            "throughput_rec_per_sec": throughput,
        }
    return results


def benchmark_merkle(batch_sizes: list[int], iterations: int = 10) -> dict:
    manager = AttestationManager(AttestationManager.generate_keypair()[0])
    results = {}
    for batch_size in batch_sizes:
        hashes = [manager._hash_record({"id": i, "data": "x" * 100}) for i in range(batch_size)]
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            root = manager._compute_merkle_root(hashes)
            elapsed = time.perf_counter() - start
            times.append(elapsed)
        median_time = statistics.median(times)
        results[batch_size] = {
            "median_time_ms": median_time * 1000,
        }
    return results


def benchmark_tamper_detection(batch_sizes: list[int], iterations: int = 10) -> dict:
    manager = AttestationManager(AttestationManager.generate_keypair()[0])
    results = {}
    for batch_size in batch_sizes:
        records = [{"id": i, "data": "x" * 100} for i in range(batch_size)]
        manifest = manager.create_manifest(records, "1.0.0", "1.0.0")
        # Tamper with one record
        tampered = records.copy()
        tampered[batch_size // 2]["data"] = "tampered"
        times = []
        for _ in range(iterations):
            start = time.perf_counter()
            valid = manager.verify_manifest(manifest, tampered)
            elapsed = time.perf_counter() - start
            times.append(elapsed)
        median_time = statistics.median(times)
        results[batch_size] = {
            "median_time_ms": median_time * 1000,
            "detected": not valid,
        }
    return results


def main():
    batch_sizes = [10, 100, 1000, 10_000, 100_000, 1_000_000]
    print("batch_size,operation,median_time_ms,throughput_rec_per_sec,detected")

    signing = benchmark_signing(batch_sizes)
    for bs, res in signing.items():
        print(f"{bs},sign,{res['median_time_ms']:.4f},{res['throughput_rec_per_sec']:.0f},")

    verification = benchmark_verification(batch_sizes)
    for bs, res in verification.items():
        print(f"{bs},verify,{res['median_time_ms']:.4f},{res['throughput_rec_per_sec']:.0f},")

    merkle = benchmark_merkle(batch_sizes)
    for bs, res in merkle.items():
        print(f"{bs},merkle,{res['median_time_ms']:.4f},,")

    tamper = benchmark_tamper_detection(batch_sizes)
    for bs, res in tamper.items():
        print(f"{bs},tamper,{res['median_time_ms']:.4f},,{res['detected']}")


if __name__ == "__main__":
    main()
