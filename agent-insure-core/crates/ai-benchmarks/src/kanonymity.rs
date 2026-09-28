//! K-anonymity implementation

use crate::BenchmarkError;
use std::collections::HashMap;

/// K-anonymity enforcer
pub struct KAnonymity {
    k: usize,
}

impl KAnonymity {
    pub fn new(k: usize) -> Self {
        Self { k }
    }

    /// Check if a group meets k-anonymity threshold
    pub fn check_group(&self, size: usize) -> bool {
        size >= self.k
    }

    /// Filter groups that don't meet k-anonymity
    pub fn filter_groups<T>(&self, groups: HashMap<String, Vec<T>>) -> (HashMap<String, Vec<T>>, Vec<String>) {
        let mut passing = HashMap::new();
        let mut suppressed = Vec::new();

        for (key, values) in groups {
            if self.check_group(values.len()) {
                passing.insert(key, values);
            } else {
                suppressed.push(key);
            }
        }

        (passing, suppressed)
    }

    /// Validate that publishing is safe
    pub fn validate_for_publishing(
        &self,
        sample_size: usize,
    ) -> Result<(), BenchmarkError> {
        if sample_size < self.k {
            return Err(BenchmarkError::KAnonymityViolation {
                k: self.k,
                size: sample_size,
            });
        }
        Ok(())
    }
}

impl Default for KAnonymity {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k_anonymity_check() {
        let ka = KAnonymity::new(3);

        assert!(!ka.check_group(1));
        assert!(!ka.check_group(2));
        assert!(ka.check_group(3));
        assert!(ka.check_group(10));
    }

    #[test]
    fn test_filter_groups() {
        let ka = KAnonymity::new(3);

        let mut groups = HashMap::new();
        groups.insert("a".to_string(), vec![1, 2, 3]);
        groups.insert("b".to_string(), vec![1, 2]);
        groups.insert("c".to_string(), vec![1, 2, 3, 4, 5]);

        let (passing, suppressed) = ka.filter_groups(groups);

        assert_eq!(passing.len(), 2);
        assert!(passing.contains_key("a"));
        assert!(passing.contains_key("c"));
        assert_eq!(suppressed.len(), 1);
        assert!(suppressed.contains(&"b".to_string()));
    }
}
