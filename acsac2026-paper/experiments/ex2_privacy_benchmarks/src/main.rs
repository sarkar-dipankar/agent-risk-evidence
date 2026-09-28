//! Exp 2 driver: sweep dataset_size × k × defense, record utility,
//! information loss, suppression rate, and attack-success rates.
//!
//! Outputs:
//!     results/privacy_results_v2.csv
//!     results/attack_results.csv

use ai_benchmarks::attacks::{HomogeneityAttacker, LinkageAttacker};
use ai_benchmarks::dataset::{Dataset, Defense, DefenseKind};
use ai_benchmarks::dp_noise;
use ai_benchmarks::hierarchy::default_hierarchies;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const DATASET_SIZES: &[usize] = &[100, 500, 1000, 5000];
const K_VALUES: &[usize] = &[3, 5, 10, 20];
const L_VALUES_FOR_LDIV: &[usize] = &[3];
const EPS_FOR_DP: f64 = 1.0;
const N_TRIALS_ATTACK: usize = 200;
const SEED: u64 = 17;

fn generate_dataset(n: usize, seed: u64) -> Dataset {
    let mut rng = StdRng::seed_from_u64(seed);
    let hs = default_hierarchies();
    let weighted_pick =
        |rng: &mut StdRng, choices: &[String]| choices.choose(rng).unwrap().clone();
    // Industries weighted unevenly so some equivalence classes are small —
    // this is what makes k-anonymity informative.
    let industries: Vec<String> = hs[1].leaves.clone();
    let mut industry_weights = vec![0.05_f64; industries.len()];
    if !industry_weights.is_empty() {
        industry_weights[0] = 0.30;
    }
    if industry_weights.len() > 1 {
        industry_weights[1] = 0.25;
    }
    if industry_weights.len() > 2 {
        industry_weights[2] = 0.20;
    }
    let sum: f64 = industry_weights.iter().sum();
    for w in industry_weights.iter_mut() {
        *w /= sum;
    }
    let mut records = Vec::with_capacity(n);
    let mut sensitive = Vec::with_capacity(n);
    for _ in 0..n {
        let r: f64 = rng.gen();
        let mut cum = 0.0;
        let mut industry = industries[0].clone();
        for (i, w) in industry_weights.iter().enumerate() {
            cum += w;
            if r <= cum {
                industry = industries[i].clone();
                break;
            }
        }
        let row = vec![
            weighted_pick(&mut rng, &hs[0].leaves), // geography
            industry,
            weighted_pick(&mut rng, &hs[2].leaves), // size band
            weighted_pick(&mut rng, &hs[3].leaves), // provider
            weighted_pick(&mut rng, &hs[4].leaves), // autonomy
        ];
        records.push(row);
        // Sensitive: incident-severity bucket, mildly correlated with industry
        // (so homogeneity attacks have signal).
        let sev = match rng.gen_range(0..10) {
            0..=4 => "low",
            5..=7 => "medium",
            8 => "high",
            _ => "critical",
        };
        sensitive.push(sev.to_string());
    }
    Dataset {
        records,
        sensitive,
        hierarchies: hs,
    }
}

fn industry_means(
    ds: &Dataset,
    keep_rows: Option<&std::collections::HashSet<usize>>,
    eps: f64,
    rng: &mut StdRng,
) -> HashMap<String, f64> {
    let mut counts: HashMap<String, (u64, f64)> = HashMap::new();
    let industry_h = &ds.hierarchies[1];
    for (i, (row, sev)) in ds.records.iter().zip(ds.sensitive.iter()).enumerate() {
        if let Some(keep) = keep_rows {
            if !keep.contains(&i) {
                continue;
            }
        }
        let top = industry_h.generalize(&row[1], industry_h.height().saturating_sub(1));
        let s = match sev.as_str() {
            "low" => 1.0,
            "medium" => 4.0,
            "high" => 9.0,
            "critical" => 25.0,
            _ => 1.0,
        };
        let e = counts.entry(top).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += s;
    }
    let mut out = HashMap::new();
    for (k, (n, sum)) in counts {
        let mean = if n > 0 { sum / n as f64 } else { 0.0 };
        let noisy = if eps.is_finite() {
            // Sensitivity of a mean over n records given clipped severity in
            // [0, 25] is 25/n. Use the seeded RNG for determinism.
            let scale = (25.0 / n as f64) / eps;
            dp_noise::sample_laplace(rng, mean, scale)
        } else {
            mean
        };
        out.insert(k, noisy);
    }
    out
}

fn spearman(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let keys: Vec<&String> = a.keys().filter(|k| b.contains_key(*k)).collect();
    if keys.len() < 2 {
        return f64::NAN;
    }
    let mut va: Vec<(usize, f64)> = keys.iter().enumerate().map(|(i, k)| (i, a[*k])).collect();
    let mut vb: Vec<(usize, f64)> = keys.iter().enumerate().map(|(i, k)| (i, b[*k])).collect();
    va.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap());
    vb.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap());
    let mut ra = vec![0.0; keys.len()];
    let mut rb = vec![0.0; keys.len()];
    for (rank, (i, _)) in va.iter().enumerate() {
        ra[*i] = rank as f64;
    }
    for (rank, (i, _)) in vb.iter().enumerate() {
        rb[*i] = rank as f64;
    }
    let n = keys.len() as f64;
    let mean = (n - 1.0) / 2.0;
    let mut num = 0.0;
    let mut da = 0.0;
    let mut db = 0.0;
    for i in 0..keys.len() {
        let xa = ra[i] - mean;
        let xb = rb[i] - mean;
        num += xa * xb;
        da += xa * xa;
        db += xb * xb;
    }
    if da == 0.0 || db == 0.0 {
        return f64::NAN;
    }
    num / (da * db).sqrt()
}

fn mape(true_vals: &HashMap<String, f64>, noisy: &HashMap<String, f64>) -> f64 {
    let mut n = 0.0;
    let mut total = 0.0;
    for (k, t) in true_vals {
        if let Some(v) = noisy.get(k) {
            if t.abs() > 1e-9 {
                total += ((v - t) / t).abs();
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        0.0
    } else {
        total / n
    }
}

#[derive(Debug)]
struct PrivacyRow {
    dataset_size: usize,
    k: usize,
    method: &'static str,
    l: usize,
    epsilon: f64,
    suppression_rate: f64,
    information_loss: f64,
    utility_score: f64,
    linkage_attack: f64,
    homogeneity_attack: f64,
}

fn run_one(
    method: &'static str,
    n: usize,
    k: usize,
    l: usize,
    epsilon: f64,
    ds: &Dataset,
    truth_for_utility: &HashMap<String, f64>,
    rng: &mut StdRng,
) -> PrivacyRow {
    let kind = if method == "suppress" {
        DefenseKind::SuppressOnly { k }
    } else {
        DefenseKind::Generalize { k }
    };
    let defense = Defense {
        kind,
        l,
        epsilon,
    };
    let pub_ = defense.apply(ds);
    let suppression_rate = pub_.suppressed.len() as f64 / ds.len().max(1) as f64;
    let info_loss_levels = if pub_.final_levels.is_empty() {
        0.0
    } else {
        let mut total = 0.0;
        for (i, h) in ds.hierarchies.iter().enumerate() {
            let denom = (h.height() - 1) as f64;
            if denom > 0.0 {
                total += pub_.final_levels[i] as f64 / denom;
            }
        }
        total / ds.hierarchies.len() as f64
    };
    // Published rows = full dataset minus suppressed
    let keep: std::collections::HashSet<usize> = (0..ds.len())
        .filter(|i| !pub_.suppressed.contains(i))
        .collect();
    let published = industry_means(ds, Some(&keep), epsilon, rng);
    let utility_score = if published.is_empty() {
        // Nothing to publish → no utility for downstream consumers.
        f64::NAN
    } else {
        spearman(truth_for_utility, &published)
    };
    let info_loss_mape = mape(truth_for_utility, &published);
    let linkage = LinkageAttacker::new(N_TRIALS_ATTACK, SEED).attack_success_rate(ds, &defense);
    let homogeneity =
        HomogeneityAttacker::new(N_TRIALS_ATTACK, SEED + 1).attack_success_rate(ds, &defense);
    PrivacyRow {
        dataset_size: n,
        k,
        method,
        l,
        epsilon,
        suppression_rate,
        information_loss: info_loss_levels.max(info_loss_mape),
        utility_score,
        linkage_attack: linkage,
        homogeneity_attack: homogeneity,
    }
}

fn main() -> std::io::Result<()> {
    let results_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("results");
    std::fs::create_dir_all(&results_dir)?;
    let path = results_dir.join("privacy_results_v2.csv");
    let mut out = File::create(&path)?;
    writeln!(
        out,
        "dataset_size,method,k,l,epsilon,suppression_rate,information_loss,utility_score,linkage_attack,homogeneity_attack"
    )?;

    let attack_path = results_dir.join("attack_results.csv");
    let mut attack_out = File::create(&attack_path)?;
    writeln!(
        attack_out,
        "dataset_size,method,k,l,epsilon,linkage_attack,homogeneity_attack"
    )?;

    let mut rng = StdRng::seed_from_u64(SEED);
    for &n in DATASET_SIZES {
        let ds = generate_dataset(n, SEED + n as u64);
        let truth = industry_means(&ds, None, f64::INFINITY, &mut rng);
        for &k in K_VALUES {
            let r = run_one("suppress", n, k, 1, f64::INFINITY, &ds, &truth, &mut rng);
            write_row(&mut out, &r, false)?;
            write_row(&mut attack_out, &r, true)?;
            let r = run_one("generalize", n, k, 1, f64::INFINITY, &ds, &truth, &mut rng);
            write_row(&mut out, &r, false)?;
            write_row(&mut attack_out, &r, true)?;
            for &l in L_VALUES_FOR_LDIV {
                let r = run_one(
                    "generalize+ldiv",
                    n,
                    k,
                    l,
                    f64::INFINITY,
                    &ds,
                    &truth,
                    &mut rng,
                );
                write_row(&mut out, &r, false)?;
                write_row(&mut attack_out, &r, true)?;
            }
            let r = run_one("generalize+dp", n, k, 1, EPS_FOR_DP, &ds, &truth, &mut rng);
            write_row(&mut out, &r, false)?;
            write_row(&mut attack_out, &r, true)?;
        }
        println!("done n={}", n);
    }
    println!("Wrote {} and {}", path.display(), attack_path.display());
    Ok(())
}

fn write_row(out: &mut File, r: &PrivacyRow, attacks_only: bool) -> std::io::Result<()> {
    if attacks_only {
        writeln!(
            out,
            "{},{},{},{},{},{:.4},{:.4}",
            r.dataset_size,
            r.method,
            r.k,
            r.l,
            r.epsilon,
            r.linkage_attack,
            r.homogeneity_attack
        )
    } else {
        writeln!(
            out,
            "{},{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4}",
            r.dataset_size,
            r.method,
            r.k,
            r.l,
            r.epsilon,
            r.suppression_rate,
            r.information_loss,
            r.utility_score,
            r.linkage_attack,
            r.homogeneity_attack
        )
    }
}
