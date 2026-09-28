//! Privacy attack simulators.
//!
//! Two complementary adversaries:
//!
//! * [`LinkageAttacker`] — knows the QI tuple of a target organisation and
//!   tries to locate it in the *published* (post-defense) view. The attack
//!   succeeds when the target maps to a *unique* equivalence class so the
//!   attacker can read off the per-class statistics with certainty.
//! * [`HomogeneityAttacker`] — works on the published equivalence classes
//!   themselves and counts those whose sensitive attribute takes a single
//!   value across the class. Any victim in such a class has their sensitive
//!   value disclosed even without identification, the canonical motivation
//!   for ℓ-diversity (Machanavajjhala et al., 2007).
//!
//! Both attackers expose `attack_success_rate(...)` which evaluates the
//! given defense over a configurable number of random victims (default
//! `100`) and returns the empirical success rate in `[0, 1]`.

use crate::dataset::{Dataset, Defense, Publication};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

/// Linkage attack: external auxiliary data → equivalence class in
/// publication → re-identification when the class is a singleton.
pub struct LinkageAttacker {
    pub trials: usize,
    pub seed: u64,
}

impl LinkageAttacker {
    pub fn new(trials: usize, seed: u64) -> Self {
        Self { trials, seed }
    }

    pub fn attack_success_rate(&self, dataset: &Dataset, defense: &Defense) -> f64 {
        let publication = defense.apply(dataset);
        if dataset.records.is_empty() {
            return 0.0;
        }
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let mut hits = 0_usize;
        for _ in 0..self.trials {
            // Pick a random victim by index
            let victim = rng.gen_range(0..dataset.records.len());
            if Self::single_trial(victim, &publication) {
                hits += 1;
            }
        }
        hits as f64 / self.trials as f64
    }

    fn single_trial(victim: usize, publication: &Publication) -> bool {
        // If the victim was suppressed, the attacker can't link them at all.
        if publication.suppressed.contains(&victim) {
            return false;
        }
        // The attacker knows the victim's raw QI tuple; they apply the same
        // generalization levels to find the equivalence class. We just look
        // up the published key for the victim.
        let key = match publication.row_to_key.get(&victim) {
            Some(k) => k,
            None => return false,
        };
        let class_size = publication
            .partitions
            .get(key)
            .map(|v| v.len())
            .unwrap_or(0);
        // Re-identification succeeds when the equivalence class is a
        // singleton — the attacker is then certain who the victim is.
        class_size == 1
    }
}

/// Homogeneity attack — succeeds when the equivalence class is sensitive-
/// attribute homogeneous (Machanavajjhala et al., 2007).
pub struct HomogeneityAttacker {
    pub trials: usize,
    pub seed: u64,
}

impl HomogeneityAttacker {
    pub fn new(trials: usize, seed: u64) -> Self {
        Self { trials, seed }
    }

    pub fn attack_success_rate(&self, dataset: &Dataset, defense: &Defense) -> f64 {
        let publication = defense.apply(dataset);
        if dataset.records.is_empty() {
            return 0.0;
        }
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed.wrapping_add(7));
        let mut hits = 0_usize;
        for _ in 0..self.trials {
            let victim = rng.gen_range(0..dataset.records.len());
            if Self::single_trial(victim, dataset, &publication) {
                hits += 1;
            }
        }
        hits as f64 / self.trials as f64
    }

    fn single_trial(
        victim: usize,
        dataset: &Dataset,
        publication: &Publication,
    ) -> bool {
        if publication.suppressed.contains(&victim) {
            return false;
        }
        let key = match publication.row_to_key.get(&victim) {
            Some(k) => k,
            None => return false,
        };
        let idxs = match publication.partitions.get(key) {
            Some(v) => v,
            None => return false,
        };
        // Class is "homogeneous" iff every member shares the same sensitive
        // value.
        let first = &dataset.sensitive[idxs[0]];
        idxs.iter().all(|i| &dataset.sensitive[*i] == first)
    }
}

/// Utility helper: pick `n` victims uniformly at random — used by tests.
#[allow(dead_code)]
pub(crate) fn sample_victims(n_records: usize, n_samples: usize, seed: u64) -> Vec<usize> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut pool: Vec<usize> = (0..n_records).collect();
    pool.shuffle(&mut rng);
    pool.truncate(n_samples);
    pool
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::DefenseKind;
    use crate::hierarchy::default_hierarchies;
    use rand::seq::SliceRandom;
    use rand::SeedableRng;

    fn build_dataset(n: usize, seed: u64) -> Dataset {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let hs = default_hierarchies();
        let mut rows = Vec::with_capacity(n);
        let mut sensitive = Vec::with_capacity(n);
        for _ in 0..n {
            rows.push(vec![
                hs[0].leaves.choose(&mut rng).unwrap().clone(),
                hs[1].leaves.choose(&mut rng).unwrap().clone(),
                hs[2].leaves.choose(&mut rng).unwrap().clone(),
                hs[3].leaves.choose(&mut rng).unwrap().clone(),
                hs[4].leaves.choose(&mut rng).unwrap().clone(),
            ]);
            sensitive.push(format!("sev_{}", rng.gen_range(0..4)));
        }
        Dataset {
            records: rows,
            sensitive,
            hierarchies: hs,
        }
    }

    #[test]
    fn linkage_rate_drops_with_k_anonymity() {
        let ds = build_dataset(200, 11);
        // Suppress-only with k=10 will eliminate most singletons.
        let weak = Defense {
            kind: DefenseKind::SuppressOnly { k: 1 },
            l: 1,
            epsilon: f64::INFINITY,
        };
        let strong = Defense {
            kind: DefenseKind::Generalize { k: 10 },
            l: 1,
            epsilon: f64::INFINITY,
        };
        let attacker = LinkageAttacker::new(200, 99);
        let weak_rate = attacker.attack_success_rate(&ds, &weak);
        let strong_rate = attacker.attack_success_rate(&ds, &strong);
        assert!(
            strong_rate <= weak_rate,
            "stronger defense should not increase linkage rate (weak={:.2} strong={:.2})",
            weak_rate,
            strong_rate
        );
    }

    #[test]
    fn ldiversity_reduces_homogeneity_attack() {
        let ds = build_dataset(200, 21);
        let no_l = Defense {
            kind: DefenseKind::Generalize { k: 5 },
            l: 1,
            epsilon: f64::INFINITY,
        };
        let with_l = Defense {
            kind: DefenseKind::Generalize { k: 5 },
            l: 3,
            epsilon: f64::INFINITY,
        };
        let attacker = HomogeneityAttacker::new(200, 33);
        let plain = attacker.attack_success_rate(&ds, &no_l);
        let diverse = attacker.attack_success_rate(&ds, &with_l);
        assert!(
            diverse <= plain + 1e-9,
            "ℓ-diversity should not increase homogeneity attack rate"
        );
    }
}
