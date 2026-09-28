# Validation and manuscript corrections — 28 September 2026

Passed:

- Offline validation of all 3,600 recorded trials against the 60 per-cell counts, including 2,989 classifiable rows, 605 HTTP errors, and six judge errors.
- Exact replay of the overall, 12 per-category, and five per-model 2,000-resample bootstrap means and intervals (NumPy RNG seed 42).
- Recalculated Cohen's kappa from 359 valid paired labels, matching 0.7705807170479124. The sample had 360 rows and one secondary-judge error.
- All 12 Rust attestation unit tests and four fixed-input Merkle compatibility tests; one illustrative doc test is ignored by upstream.
- Manuscript tables regenerated directly from `cell_errors.csv`, `privacy_results_v2.csv`, and `attestation_rust.csv`.
- Credential-pattern scan of release text files found no private-key blocks or recognizable access-token literals.

Material corrections:

- Results measure a static, single-turn prompt suite, not deployed agent incidents or insured losses.
- The secondary judge shares Z.AI with tested GLM-5; agreement is not an accuracy bound.
- HTTP failures and judge failures are distinguished. Three cells have no valid rows; high-error cells with some valid responses are retained by the analysis.
- The pooling table now matches the saved CSV: Laplace-configuration utility is 0.7143 at n=500 and 0.9762 at n=5,000 (k=5). Suppression of every record is restricted to k>=5 in the recorded sweep, not all k.
- Source hierarchy sizes imply 34,560 possible raw QI combinations. Sensitive buckets are independently sampled, not correlated with industry. Unordered map iteration prevents treating the seed as a complete deterministic specification.
- The full data-dependent pooling release has no established differential-privacy guarantee. The Laplace scale is an ablation parameter; no blanket production privacy recommendation is made.
- Rust verification medians range from 38.9 to 54.3 microseconds across the batch sweep. The separate 36.4-microsecond tamper measurement uses n=10,000 only. Python and Rust schemas and verification workloads differ.
- The Rust signature excludes timestamp/version fields; membership verification does not authenticate ordinal position. Identity, key rotation, authenticated time, omission detection, and an integrated insurance workflow remain outside the evaluated implementation.

These checks validate the archived analysis and the stated implementation scope. They do not constitute independent expert relabelling, new model experiments, new privacy proofs for the released pooling pipeline, or new performance measurements.
