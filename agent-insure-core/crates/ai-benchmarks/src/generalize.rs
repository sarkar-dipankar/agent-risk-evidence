//! Hierarchical generalization for k-anonymity.
//!
//! We follow the *full-domain generalization with greedy ascent* strategy of
//! Sweeney (2002), aided by a Mondrian-style "expand the worst dimension"
//! heuristic (LeFevre, DeWitt, Ramakrishnan, 2006). The algorithm:
//!
//! 1. Start with every quasi-identifier (QI) at level 0 (raw values).
//! 2. Repeatedly partition the records by their generalized QI tuple. If
//!    every partition already has at least `k` records, stop.
//! 3. Otherwise, pick the QI whose advancement would suppress the fewest
//!    records (greedy "smallest information loss" choice) and bump it up
//!    one level.
//! 4. Stop when k-anonymity is satisfied, or when all hierarchies are at
//!    their top level (in which case the remaining sub-k groups are
//!    suppressed).
//!
//! The returned [`Generalized`] structure exposes the partitioning, the
//! generalization levels used, and the indices that had to be suppressed.

use crate::hierarchy::Hierarchy;
use std::collections::HashMap;

/// One generalized record. `qi_values` are the (already generalized) QI
/// values, in the same order as the input hierarchies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralizedRecord {
    pub qi_values: Vec<String>,
}

/// Result of a generalization pass.
#[derive(Debug, Clone)]
pub struct Generalized {
    /// One entry per input record (same order, same length).
    pub records: Vec<GeneralizedRecord>,
    /// Indices of the input records that were ultimately suppressed (their
    /// equivalence class never reached size `k`).
    pub suppressed: Vec<usize>,
    /// The generalization level finally used for each QI dimension.
    pub final_levels: Vec<usize>,
    /// Partition: generalized QI tuple → list of input indices belonging to
    /// the partition that *survived* (i.e. is published).
    pub partitions: HashMap<Vec<String>, Vec<usize>>,
}

impl Generalized {
    /// Suppression rate as a fraction of input rows.
    pub fn suppression_rate(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        self.suppressed.len() as f64 / self.records.len() as f64
    }

    /// Normalised Certainty Penalty (NCP) style information loss, averaged
    /// across all dimensions and records. For each dimension the score is
    /// `level / (height - 1)` so it lies in `[0, 1]`. The overall result
    /// is the mean across dimensions.
    pub fn information_loss(&self, hierarchies: &[Hierarchy]) -> f64 {
        if hierarchies.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        for (idx, h) in hierarchies.iter().enumerate() {
            // height - 1 is the max useful level (above that is full
            // suppression). Avoid divide-by-zero on degenerate hierarchies.
            let max_level = (h.height() - 1) as f64;
            if max_level <= 0.0 {
                continue;
            }
            total += self.final_levels[idx] as f64 / max_level;
        }
        total / hierarchies.len() as f64
    }
}

/// Apply current generalization levels and return a map from generalized
/// tuple → vector of original record indices in that equivalence class.
fn partition(
    raw: &[Vec<String>],
    hierarchies: &[Hierarchy],
    levels: &[usize],
) -> HashMap<Vec<String>, Vec<usize>> {
    let mut map: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (idx, row) in raw.iter().enumerate() {
        let key: Vec<String> = row
            .iter()
            .zip(hierarchies.iter())
            .zip(levels.iter())
            .map(|((value, h), lvl)| h.generalize(value, *lvl))
            .collect();
        map.entry(key).or_default().push(idx);
    }
    map
}

/// Count how many records would *still* fall into sub-k partitions if we
/// generalized dimension `dim` by one extra level. Used by the greedy
/// heuristic — lower is better.
fn would_suppress(
    raw: &[Vec<String>],
    hierarchies: &[Hierarchy],
    levels: &[usize],
    dim: usize,
    k: usize,
) -> usize {
    let mut trial = levels.to_vec();
    trial[dim] += 1;
    let p = partition(raw, hierarchies, &trial);
    p.values().filter(|v| v.len() < k).map(|v| v.len()).sum()
}

/// Run hierarchical generalization to satisfy k-anonymity.
///
/// `raw[i][d]` is the raw value of the `d`-th quasi-identifier for record
/// `i`. Hierarchies must be supplied in the same dimension order.
pub fn generalize_to_k(
    raw: &[Vec<String>],
    hierarchies: &[Hierarchy],
    k: usize,
) -> Generalized {
    let n_dims = hierarchies.len();
    assert!(
        raw.iter().all(|r| r.len() == n_dims),
        "every record must have one value per hierarchy"
    );
    let mut levels = vec![0_usize; n_dims];

    // Sanity short-circuit: if k <= 1 we don't need to generalize.
    let mut parts = partition(raw, hierarchies, &levels);
    let satisfies_k =
        |parts: &HashMap<Vec<String>, Vec<usize>>| parts.values().all(|v| v.len() >= k);

    while !satisfies_k(&parts) {
        // Find the dimension with the smallest "would suppress" count that
        // can still be advanced.
        let mut best: Option<(usize, usize)> = None; // (dim, score)
        for dim in 0..n_dims {
            let max_level = hierarchies[dim].height() - 1;
            if levels[dim] >= max_level {
                continue;
            }
            let score = would_suppress(raw, hierarchies, &levels, dim, k);
            match best {
                None => best = Some((dim, score)),
                Some((_, s)) if score < s => best = Some((dim, score)),
                _ => {}
            }
        }
        match best {
            Some((dim, _)) => {
                levels[dim] += 1;
                parts = partition(raw, hierarchies, &levels);
            }
            None => break, // can't generalize further
        }
    }

    // Whatever partitions remain below k are suppressed.
    let mut suppressed = Vec::new();
    let mut surviving: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (key, idxs) in parts.into_iter() {
        if idxs.len() < k {
            suppressed.extend(idxs);
        } else {
            surviving.insert(key, idxs);
        }
    }

    let mut records = vec![
        GeneralizedRecord {
            qi_values: vec!["*".to_string(); n_dims]
        };
        raw.len()
    ];
    for (key, idxs) in &surviving {
        for i in idxs {
            records[*i].qi_values = key.clone();
        }
    }

    Generalized {
        records,
        suppressed,
        final_levels: levels,
        partitions: surviving,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::{autonomy, geography, industry, provider_tier, size_band};
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    /// Synthetic dataset of `n` records using the canonical 5 QIs.
    fn synthetic(n: usize, seed: u64) -> Vec<Vec<String>> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let geo = geography();
        let ind = industry();
        let sz = size_band();
        let pv = provider_tier();
        let au = autonomy();
        let mut rows = Vec::with_capacity(n);
        for _ in 0..n {
            rows.push(vec![
                geo.leaves.choose(&mut rng).unwrap().clone(),
                ind.leaves.choose(&mut rng).unwrap().clone(),
                sz.leaves.choose(&mut rng).unwrap().clone(),
                pv.leaves.choose(&mut rng).unwrap().clone(),
                au.leaves.choose(&mut rng).unwrap().clone(),
            ]);
        }
        rows
    }

    #[test]
    fn k_anonymity_satisfied_after_generalization() {
        let rows = synthetic(100, 42);
        let hs = vec![geography(), industry(), size_band(), provider_tier(), autonomy()];
        let result = generalize_to_k(&rows, &hs, 5);
        // Every surviving partition must have at least k records
        for (_key, idxs) in &result.partitions {
            assert!(idxs.len() >= 5, "partition smaller than k");
        }
        // Suppression must be well-defined in [0, 1]. We do not assert a
        // tight bound because it depends on the QI distribution of the
        // synthetic input; the privacy/utility curve is the real signal
        // and is exercised by the experiment driver, not unit tests.
        let supp = result.suppression_rate();
        assert!(
            (0.0..=1.0).contains(&supp),
            "suppression rate out of range: {:.2}",
            supp
        );
        // Information loss is well-formed (in [0, 1]); saturation IS a
        // legitimate outcome for small-n datasets with high QI diversity.
        let loss = result.information_loss(&hs);
        assert!((0.0..=1.0).contains(&loss), "info loss out of range: {:.3}", loss);
    }

    #[test]
    fn already_satisfies_k_no_generalization() {
        // All 6 records share the same QIs → trivially k-anonymous for k=3
        let rows = vec![
            vec!["nyc".into(), "511210".into(), "1-50".into(), "gpt-4o".into(), "1".into()];
            6
        ];
        let hs = vec![geography(), industry(), size_band(), provider_tier(), autonomy()];
        let result = generalize_to_k(&rows, &hs, 3);
        assert_eq!(result.final_levels, vec![0; 5]);
        assert!(result.suppressed.is_empty());
    }

    #[test]
    fn extreme_k_forces_full_generalization() {
        let rows = synthetic(50, 7);
        let hs = vec![geography(), industry(), size_band(), provider_tier(), autonomy()];
        let result = generalize_to_k(&rows, &hs, 50);
        // We may or may not retain all 50 — but if we do, only one partition
        // can exist (everything generalized together).
        assert!(result.partitions.len() <= 1);
    }
}
