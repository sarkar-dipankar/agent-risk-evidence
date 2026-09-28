//! Glue types tying together hierarchies, generalization, ℓ-diversity, and
//! DP into a single `Defense::apply(&Dataset) -> Publication` interface so
//! the attack simulators in [`crate::attacks`] can compare configurations.
//!
//! Convention:
//!
//! * A [`Dataset`] holds raw QI rows, parallel sensitive-attribute values,
//!   and the generalization hierarchies for each QI dimension.
//! * A [`Defense`] specifies the privacy parameters: which generalization
//!   kind to run (`SuppressOnly` vs `Generalize`), the ℓ-diversity floor
//!   `l`, and the DP `epsilon` (use `f64::INFINITY` to disable DP).
//! * [`Defense::apply`] produces a [`Publication`] describing the published
//!   partitions, the suppressed row indices, and a per-row lookup back to
//!   the equivalence-class key.

use crate::generalize::{generalize_to_k, Generalized};
use crate::hierarchy::Hierarchy;
use std::collections::{HashMap, HashSet};

/// Input to a privacy defense.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// `records[i][d]` is the raw value of the `d`-th QI for record `i`.
    pub records: Vec<Vec<String>>,
    /// `sensitive[i]` is the sensitive-attribute value for record `i`.
    pub sensitive: Vec<String>,
    /// QI generalization hierarchies (length must equal `records[*].len()`).
    pub hierarchies: Vec<Hierarchy>,
}

impl Dataset {
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Output of a privacy defense — the public view the adversary observes.
#[derive(Debug, Clone, Default)]
pub struct Publication {
    /// Indices of records that were suppressed (NOT published).
    pub suppressed: HashSet<usize>,
    /// Row → published equivalence-class key (the generalized QI tuple).
    pub row_to_key: HashMap<usize, Vec<String>>,
    /// Published equivalence classes: generalized QI tuple → row indices.
    pub partitions: HashMap<Vec<String>, Vec<usize>>,
    /// Generalization level finally used for each QI dimension. Empty for
    /// suppress-only defenses.
    pub final_levels: Vec<usize>,
}

/// The variety of generalization/suppression applied before any layered
/// ℓ-diversity or DP step.
#[derive(Debug, Clone)]
pub enum DefenseKind {
    /// Bucket by raw QI tuple and suppress any equivalence class below `k`.
    SuppressOnly { k: usize },
    /// Hierarchical generalization until k-anonymity is satisfied; any
    /// remaining sub-k classes are suppressed.
    Generalize { k: usize },
}

/// A composed privacy defense.
#[derive(Debug, Clone)]
pub struct Defense {
    pub kind: DefenseKind,
    /// ℓ-diversity floor. Set to `1` to disable.
    pub l: usize,
    /// Differential-privacy ε for any later count/value queries that the
    /// caller may run on top of the publication. Stored here for
    /// experiment bookkeeping; `apply()` does not itself add noise.
    pub epsilon: f64,
}

impl Defense {
    /// Run the defense and return the publication adversaries observe.
    pub fn apply(&self, dataset: &Dataset) -> Publication {
        let pub_ = match self.kind {
            DefenseKind::SuppressOnly { k } => apply_suppress_only(dataset, k),
            DefenseKind::Generalize { k } => apply_generalize(dataset, k),
        };
        if self.l > 1 {
            apply_ldiversity(pub_, dataset, self.l)
        } else {
            pub_
        }
    }
}

fn apply_suppress_only(dataset: &Dataset, k: usize) -> Publication {
    let mut buckets: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, row) in dataset.records.iter().enumerate() {
        buckets.entry(row.clone()).or_default().push(i);
    }
    let mut publication = Publication::default();
    publication.final_levels = vec![0; dataset.hierarchies.len()];
    for (key, indices) in buckets {
        if indices.len() >= k {
            for i in &indices {
                publication.row_to_key.insert(*i, key.clone());
            }
            publication.partitions.insert(key, indices);
        } else {
            for i in indices {
                publication.suppressed.insert(i);
            }
        }
    }
    publication
}

fn apply_generalize(dataset: &Dataset, k: usize) -> Publication {
    let Generalized {
        records: _,
        suppressed,
        final_levels,
        partitions,
    } = generalize_to_k(&dataset.records, &dataset.hierarchies, k);
    let mut row_to_key = HashMap::new();
    for (key, indices) in &partitions {
        for i in indices {
            row_to_key.insert(*i, key.clone());
        }
    }
    Publication {
        suppressed: suppressed.into_iter().collect(),
        row_to_key,
        partitions,
        final_levels,
    }
}

fn apply_ldiversity(pub_: Publication, dataset: &Dataset, l: usize) -> Publication {
    // Drop any equivalence class whose sensitive attribute has fewer than
    // `l` distinct values.
    let mut keep: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    let mut suppressed = pub_.suppressed.clone();
    for (key, indices) in &pub_.partitions {
        let distinct: HashSet<&String> =
            indices.iter().map(|i| &dataset.sensitive[*i]).collect();
        if distinct.len() >= l {
            keep.insert(key.clone(), indices.clone());
        } else {
            for i in indices {
                suppressed.insert(*i);
            }
        }
    }
    let row_to_key = pub_
        .row_to_key
        .into_iter()
        .filter(|(i, _)| !suppressed.contains(i))
        .collect();
    Publication {
        suppressed,
        row_to_key,
        partitions: keep,
        final_levels: pub_.final_levels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::default_hierarchies;

    #[test]
    fn suppress_only_drops_singletons() {
        let hs = default_hierarchies();
        let ds = Dataset {
            records: vec![
                vec!["nyc".into(), "tech".into(), "small".into(), "openai".into(), "low".into()],
                vec!["nyc".into(), "tech".into(), "small".into(), "openai".into(), "low".into()],
                vec!["boston".into(), "finance".into(), "small".into(), "openai".into(), "low".into()],
            ],
            sensitive: vec!["A".into(), "B".into(), "C".into()],
            hierarchies: hs,
        };
        let d = Defense { kind: DefenseKind::SuppressOnly { k: 2 }, l: 1, epsilon: f64::INFINITY };
        let p = d.apply(&ds);
        // Two rows match exactly → published; the third is a singleton → suppressed.
        assert_eq!(p.suppressed.len(), 1);
        assert!(p.suppressed.contains(&2));
        assert_eq!(p.partitions.len(), 1);
    }
}
