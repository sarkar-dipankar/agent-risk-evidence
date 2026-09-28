//! Criterion benchmarks for the Ed25519 + Merkle attestation pipeline.
//!
//! These benchmarks back the throughput numbers reported in the
//! academic paper.  We measure:
//!
//!   * `sign` — end-to-end `attest_batch_with_merkle` cost (canonical
//!     JSON serialization + Merkle tree build + one Ed25519 signature)
//!     across batch sizes from 10 up to 1,000,000 records.
//!   * `verify` — `verify_merkle_inclusion` cost: signature check +
//!     proof reconstruction.  This is the per-record verifier path.
//!   * `merkle_build` — Merkle tree construction in isolation, again
//!     across batch sizes from 10 up to 1,000,000.
//!   * `inclusion_proof` — proof extraction cost from a pre-built tree.
//!   * `tamper_detection` — verifier behaviour when the record has
//!     been altered (should reject quickly).
//!
//! The 1e6 case uses `sample_size(10)` so criterion does not try to
//! run it 100 times.  All other configuration is left at criterion's
//! defaults so the standard JSON report under
//! `target/criterion/<bench>/...` is produced automatically.

use ai_attestation::{AttestationVerifier, Attester, MerkleTree};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::json;

/// Build a deterministic batch of JSON records of the requested size.
fn make_records(n: usize) -> Vec<serde_json::Value> {
    (0..n)
        .map(|i| {
            json!({
                "id": format!("record-{i:08}"),
                "value": i as u64,
                "tag": "benchmark",
                "nested": {"k": i % 7, "ok": (i % 2) == 0},
            })
        })
        .collect()
}

/// Pre-serialise records into the byte form used as Merkle leaves.
fn make_leaves(records: &[serde_json::Value]) -> Vec<Vec<u8>> {
    records
        .iter()
        .map(|r| serde_json::to_vec(r).unwrap())
        .collect()
}

/// Batch sizes used by the throughput benches.  Picked to span four
/// orders of magnitude so the paper's plots can show scaling.
const BATCH_SIZES: &[usize] = &[10, 100, 1_000, 10_000, 100_000, 1_000_000];

fn bench_sign(c: &mut Criterion) {
    let attester = Attester::generate();
    let mut group = c.benchmark_group("sign");
    for &n in BATCH_SIZES {
        let records = make_records(n);
        group.throughput(Throughput::Elements(n as u64));
        // 1e6 is slow — limit sample count so the run finishes in
        // reasonable wall-clock time.
        if n >= 1_000_000 {
            group.sample_size(10);
        }
        group.bench_with_input(BenchmarkId::from_parameter(n), &records, |b, recs| {
            b.iter(|| {
                let m = attester
                    .attest_batch_with_merkle(recs, "1.0.0")
                    .expect("attest");
                criterion::black_box(m);
            });
        });
    }
    group.finish();
}

fn bench_verify(c: &mut Criterion) {
    let attester = Attester::generate();
    let public_key = attester.public_key();
    let mut group = c.benchmark_group("verify");

    // Verification cost is dominated by the signature check (constant)
    // plus log(n) hashes in the inclusion proof, so we sweep batch
    // sizes to expose the log-n term.
    for &n in BATCH_SIZES {
        let records = make_records(n);
        let manifest = attester
            .attest_batch_with_merkle(&records, "1.0.0")
            .unwrap();
        let leaves = make_leaves(&records);
        let tree = MerkleTree::build(&leaves);
        let root = tree.root();
        // Pick the middle index so the proof has roughly the worst-case
        // length for this batch size.
        let idx = n / 2;
        let proof = tree.inclusion_proof(idx);
        let record = records[idx].clone();

        if n >= 1_000_000 {
            group.sample_size(10);
        }
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                AttestationVerifier::verify_merkle_inclusion(
                    &manifest,
                    &public_key,
                    &root,
                    &proof,
                    &record,
                    idx,
                )
                .expect("verify");
            });
        });
    }
    group.finish();
}

fn bench_merkle_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_build");
    for &n in BATCH_SIZES {
        let records = make_records(n);
        let leaves = make_leaves(&records);
        group.throughput(Throughput::Elements(n as u64));
        if n >= 1_000_000 {
            group.sample_size(10);
        }
        group.bench_with_input(BenchmarkId::from_parameter(n), &leaves, |b, ls| {
            b.iter(|| {
                let t = MerkleTree::build(ls);
                criterion::black_box(t.root());
            });
        });
    }
    group.finish();
}

fn bench_inclusion_proof(c: &mut Criterion) {
    let mut group = c.benchmark_group("inclusion_proof");
    // Proof extraction is O(log n), so smaller batches dominate the
    // setup cost; we still sweep a representative range.
    for &n in &[1_000usize, 10_000, 100_000, 1_000_000] {
        let records = make_records(n);
        let leaves = make_leaves(&records);
        let tree = MerkleTree::build(&leaves);
        let idx = n / 2;
        if n >= 1_000_000 {
            group.sample_size(10);
        }
        group.bench_with_input(BenchmarkId::from_parameter(n), &idx, |b, &i| {
            b.iter(|| {
                let p = tree.inclusion_proof(i);
                criterion::black_box(p);
            });
        });
    }
    group.finish();
}

fn bench_tamper(c: &mut Criterion) {
    // Tamper-detection latency: verifier should reject a doctored
    // record about as fast as it accepts a good one (it still has to
    // recompute the proof to discover the mismatch).
    let attester = Attester::generate();
    let public_key = attester.public_key();
    let records = make_records(10_000);
    let manifest = attester
        .attest_batch_with_merkle(&records, "1.0.0")
        .unwrap();
    let leaves = make_leaves(&records);
    let tree = MerkleTree::build(&leaves);
    let root = tree.root();
    let idx = 5_000;
    let proof = tree.inclusion_proof(idx);
    let tampered = json!({"id": "tampered", "value": -1});

    c.bench_function("tamper_detection", |b| {
        b.iter(|| {
            let res = AttestationVerifier::verify_merkle_inclusion(
                &manifest,
                &public_key,
                &root,
                &proof,
                &tampered,
                idx,
            );
            // Should always be Err — we benchmark the rejection path.
            assert!(res.is_err());
        });
    });
}

criterion_group!(
    benches,
    bench_sign,
    bench_verify,
    bench_merkle_build,
    bench_inclusion_proof,
    bench_tamper,
);
criterion_main!(benches);
