//! Generalization hierarchies for quasi-identifiers.
//!
//! Each [`Hierarchy`] maps values at level `i` to their parent at level `i+1`.
//! Level 0 is the most specific (raw value); the top level is the
//! suppression-equivalent constant (e.g. `*`).
//!
//! Five hierarchies ship out of the box, covering the quasi-identifiers used
//! by the Agent Insure portfolio benchmarks: geography, industry (NAICS),
//! organization size band, model provider tier, and agent autonomy level.
//! These are inspired by the generalization trees used by Sweeney (2002) and
//! the Mondrian multidimensional approach of LeFevre et al. (2006).

use std::collections::HashMap;

/// A generalization hierarchy for a single quasi-identifier.
///
/// `levels[i]` maps a value at level `i` (the more specific level) to its
/// parent at level `i+1`. The number of `levels` is the height of the
/// hierarchy minus one (the top is the implicit suppression constant `*`).
#[derive(Debug, Clone)]
pub struct Hierarchy {
    pub name: String,
    pub levels: Vec<HashMap<String, String>>,
    /// Domain at level 0 — useful for tests/utility computations.
    pub leaves: Vec<String>,
}

impl Hierarchy {
    /// Number of generalization levels available (height of the tree).
    ///
    /// Level `0` is the raw value, level `levels.len()` is `*` (full
    /// suppression).
    pub fn height(&self) -> usize {
        self.levels.len() + 1
    }

    /// Generalize `value` from level 0 up to `target_level`.
    ///
    /// When `target_level == 0` the value is returned untouched. When
    /// `target_level >= levels.len()` the result is the suppression constant
    /// `"*"`.
    pub fn generalize(&self, value: &str, target_level: usize) -> String {
        if target_level == 0 {
            return value.to_string();
        }
        if target_level >= self.height() {
            return "*".to_string();
        }
        let mut current = value.to_string();
        for level_map in self.levels.iter().take(target_level) {
            current = level_map
                .get(&current)
                .cloned()
                .unwrap_or_else(|| "*".to_string());
        }
        current
    }
}

fn map_from(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Geography hierarchy: city → census-region → country.
///
/// Uses a small but representative set of US cities so the experiments can
/// run end-to-end without external data.
pub fn geography() -> Hierarchy {
    // city -> census region
    let city_to_region = map_from(&[
        ("nyc", "northeast"),
        ("boston", "northeast"),
        ("philadelphia", "northeast"),
        ("chicago", "midwest"),
        ("detroit", "midwest"),
        ("minneapolis", "midwest"),
        ("austin", "south"),
        ("atlanta", "south"),
        ("miami", "south"),
        ("la", "west"),
        ("sf", "west"),
        ("seattle", "west"),
    ]);
    let region_to_country = map_from(&[
        ("northeast", "us"),
        ("midwest", "us"),
        ("south", "us"),
        ("west", "us"),
    ]);
    Hierarchy {
        name: "geography".to_string(),
        levels: vec![city_to_region, region_to_country],
        leaves: vec![
            "nyc", "boston", "philadelphia", "chicago", "detroit", "minneapolis", "austin",
            "atlanta", "miami", "la", "sf", "seattle",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    }
}

/// Industry hierarchy: NAICS-6 → NAICS-4 → NAICS-2.
///
/// Eight NAICS-2 codes covering technology, finance, healthcare, retail,
/// manufacturing, energy, education and government. NAICS-6 codes are
/// abbreviated for readability.
pub fn industry() -> Hierarchy {
    // NAICS-6 -> NAICS-4
    let six_to_four = map_from(&[
        // 51 — Information / tech
        ("511210", "5112"),
        ("518210", "5182"),
        ("541511", "5415"),
        // 52 — Finance
        ("522110", "5221"),
        ("523120", "5231"),
        ("524113", "5241"),
        // 62 — Healthcare
        ("621111", "6211"),
        ("622110", "6221"),
        // 44 — Retail
        ("441110", "4411"),
        ("445110", "4451"),
        // 33 — Manufacturing
        ("334111", "3341"),
        ("336411", "3364"),
        // 22 — Energy / utilities
        ("221112", "2211"),
        ("221122", "2211"),
        // 61 — Education
        ("611310", "6113"),
        ("611110", "6111"),
        // 92 — Government / public admin
        ("921110", "9211"),
        ("922160", "9221"),
    ]);
    // NAICS-4 -> NAICS-2
    let four_to_two = map_from(&[
        ("5112", "51"),
        ("5182", "51"),
        ("5415", "51"),
        ("5221", "52"),
        ("5231", "52"),
        ("5241", "52"),
        ("6211", "62"),
        ("6221", "62"),
        ("4411", "44"),
        ("4451", "44"),
        ("3341", "33"),
        ("3364", "33"),
        ("2211", "22"),
        ("6113", "61"),
        ("6111", "61"),
        ("9211", "92"),
        ("9221", "92"),
    ]);
    let leaves: Vec<String> = six_to_four.keys().cloned().collect();
    Hierarchy {
        name: "industry".to_string(),
        levels: vec![six_to_four, four_to_two],
        leaves,
    }
}

/// Organization size band: 1-50 → 51-500 → 501-5000 → 5001+ → *.
///
/// Modelled as a three-level hierarchy where smaller bands merge into larger
/// ones (small / medium / enterprise).
pub fn size_band() -> Hierarchy {
    // raw band -> midsize grouping
    let level_one = map_from(&[
        ("1-50", "smb"),
        ("51-500", "smb"),
        ("501-5000", "enterprise"),
        ("5001+", "enterprise"),
    ]);
    let level_two = map_from(&[("smb", "any"), ("enterprise", "any")]);
    Hierarchy {
        name: "size_band".to_string(),
        levels: vec![level_one, level_two],
        leaves: vec!["1-50", "51-500", "501-5000", "5001+"]
            .into_iter()
            .map(String::from)
            .collect(),
    }
}

/// Provider tier hierarchy: specific provider → family → open/closed/hybrid.
pub fn provider_tier() -> Hierarchy {
    // model family -> tier
    let provider_to_family = map_from(&[
        ("gpt-4o", "openai"),
        ("gpt-4-turbo", "openai"),
        ("claude-3-opus", "anthropic"),
        ("claude-3-sonnet", "anthropic"),
        ("gemini-1.5-pro", "google"),
        ("gemini-1.5-flash", "google"),
        ("llama-3-70b", "meta"),
        ("llama-3-8b", "meta"),
        ("mistral-large", "mistral"),
        ("mixtral-8x7b", "mistral"),
    ]);
    let family_to_tier = map_from(&[
        ("openai", "closed"),
        ("anthropic", "closed"),
        ("google", "closed"),
        ("meta", "open"),
        ("mistral", "hybrid"),
    ]);
    let leaves: Vec<String> = provider_to_family.keys().cloned().collect();
    Hierarchy {
        name: "provider_tier".to_string(),
        levels: vec![provider_to_family, family_to_tier],
        leaves,
    }
}

/// Autonomy hierarchy: level (0..3) → category (low|medium|high) → *.
pub fn autonomy() -> Hierarchy {
    let level_to_category = map_from(&[
        ("0", "low"),
        ("1", "low"),
        ("2", "medium"),
        ("3", "high"),
    ]);
    let category_to_root = map_from(&[
        ("low", "any"),
        ("medium", "any"),
        ("high", "any"),
    ]);
    Hierarchy {
        name: "autonomy".to_string(),
        levels: vec![level_to_category, category_to_root],
        leaves: vec!["0", "1", "2", "3"]
            .into_iter()
            .map(String::from)
            .collect(),
    }
}

/// Convenience helper: build the canonical 5-QI hierarchy set used by the
/// Agent Insure benchmark experiments.
pub fn default_hierarchies() -> Vec<Hierarchy> {
    vec![
        geography(),
        industry(),
        size_band(),
        provider_tier(),
        autonomy(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geography_generalizes_to_us() {
        let h = geography();
        assert_eq!(h.generalize("nyc", 0), "nyc");
        assert_eq!(h.generalize("nyc", 1), "northeast");
        assert_eq!(h.generalize("nyc", 2), "us");
        // beyond the top is full suppression
        assert_eq!(h.generalize("nyc", 3), "*");
    }

    #[test]
    fn industry_six_to_two_path() {
        let h = industry();
        assert_eq!(h.generalize("511210", 0), "511210");
        assert_eq!(h.generalize("511210", 1), "5112");
        assert_eq!(h.generalize("511210", 2), "51");
    }

    #[test]
    fn size_band_collapses() {
        let h = size_band();
        assert_eq!(h.generalize("1-50", 1), "smb");
        assert_eq!(h.generalize("5001+", 1), "enterprise");
        assert_eq!(h.generalize("5001+", 2), "any");
    }

    #[test]
    fn unknown_value_suppressed() {
        let h = geography();
        assert_eq!(h.generalize("atlantis", 1), "*");
    }

    #[test]
    fn default_set_contains_five() {
        assert_eq!(default_hierarchies().len(), 5);
    }
}
