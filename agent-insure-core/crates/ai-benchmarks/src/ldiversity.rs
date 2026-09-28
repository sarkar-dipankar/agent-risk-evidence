//! ℓ-diversity check on top of k-anonymous equivalence classes.
//!
//! Citation: Machanavajjhala, Kifer, Gehrke, Venkitasubramaniam, *"ℓ-Diversity:
//! Privacy Beyond k-Anonymity"*, ACM TKDD 1(1), 2007.
//!
//! Distinct ℓ-diversity requires each equivalence class to contain at least
//! `ℓ` *distinct* values of the sensitive attribute. We implement that
//! "distinct ℓ-diversity" variant — the strongest of the three families
//! defined in the original paper that does not need an attacker model.
//!
//! Groups that fail the check are suppressed: their record indices are
//! removed from the partition map and reported back to the caller so they
//! can be cleared from the published statistics.

use std::collections::{HashMap, HashSet};

/// Result of applying an ℓ-diversity filter on top of a partition map.
#[derive(Debug, Clone)]
pub struct LDiversityResult {
    /// Partitions that satisfy ℓ-diversity (subset of the input).
    pub partitions: HashMap<Vec<String>, Vec<usize>>,
    /// Row indices that were suppressed because their equivalence class
    /// failed ℓ-diversity.
    pub suppressed: Vec<usize>,
    /// ℓ used for the check.
    pub l: usize,
}

impl LDiversityResult {
    /// Convenience helper to compute the additional suppression rate this
    /// filter introduced on top of generalization.
    pub fn suppression_rate(&self, total_rows: usize) -> f64 {
        if total_rows == 0 {
            return 0.0;
        }
        self.suppressed.len() as f64 / total_rows as f64
    }
}

/// Apply distinct ℓ-diversity to a partition map. `sensitive_values[i]` is
/// the sensitive attribute of row `i`.
pub fn enforce_ldiversity(
    partitions: &HashMap<Vec<String>, Vec<usize>>,
    sensitive_values: &[String],
    l: usize,
) -> LDiversityResult {
    let mut surviving: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    let mut suppressed = Vec::new();
    for (key, idxs) in partitions {
        let distinct: HashSet<&String> =
            idxs.iter().map(|i| &sensitive_values[*i]).collect();
        if distinct.len() >= l {
            surviving.insert(key.clone(), idxs.clone());
        } else {
            suppressed.extend(idxs.iter().copied());
        }
    }
    LDiversityResult {
        partitions: surviving,
        suppressed,
        l,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_when_diverse_enough() {
        let mut parts: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
        parts.insert(vec!["a".into()], vec![0, 1, 2]);
        let sens = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let r = enforce_ldiversity(&parts, &sens, 3);
        assert_eq!(r.partitions.len(), 1);
        assert!(r.suppressed.is_empty());
    }

    #[test]
    fn fails_when_homogeneous() {
        let mut parts: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
        parts.insert(vec!["a".into()], vec![0, 1, 2, 3]);
        let sens = vec!["x".to_string(); 4];
        let r = enforce_ldiversity(&parts, &sens, 2);
        assert!(r.partitions.is_empty());
        assert_eq!(r.suppressed.len(), 4);
    }

    #[test]
    fn mixed_groups() {
        let mut parts: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
        parts.insert(vec!["g1".into()], vec![0, 1, 2]); // diverse
        parts.insert(vec!["g2".into()], vec![3, 4]); // homogeneous
        let sens = vec![
            "x".to_string(),
            "y".to_string(),
            "z".to_string(),
            "p".to_string(),
            "p".to_string(),
        ];
        let r = enforce_ldiversity(&parts, &sens, 2);
        assert_eq!(r.partitions.len(), 1);
        assert_eq!(r.suppressed.len(), 2);
    }
}
