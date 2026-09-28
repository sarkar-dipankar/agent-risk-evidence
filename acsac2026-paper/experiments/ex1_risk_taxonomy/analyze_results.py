"""Aggregate raw_results.json → tables, figures, and inter-rater stats.

Outputs (into results/):
    summary_stats.json           overall + per-category elicitation rate w/ 95% bootstrap CI
    elicitation_by_model.csv     model × category long table
    tab_elicitation_rates.tex    LaTeX table replacement for §5.1
    fig_elicitation_heatmap.{pdf,png}
    fig_latency_by_model.{pdf,png}
    kappa.json                   Cohen's κ between judge and human spot-check (if results/human_labels.csv present)
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Sequence

import numpy as np
import pandas as pd
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

HERE = Path(__file__).parent
RESULTS = HERE / "results"


def load() -> pd.DataFrame:
    rows = json.load(open(RESULTS / "raw_results.json"))
    df = pd.DataFrame(rows)
    df["is_error"] = df["judge_label"] == "error"
    df["elicited"] = (df["judge_label"] == "elicited").astype(int)
    df["partial"] = (df["judge_label"] == "partial").astype(int)
    df["refused"] = (df["judge_label"] == "refused").astype(int)
    return df


def bootstrap_ci(values: Sequence[int], n_boot: int = 2000, alpha: float = 0.05) -> tuple[float, float, float]:
    arr = np.asarray(values, dtype=float)
    if arr.size == 0:
        return float("nan"), float("nan"), float("nan")
    mean = float(arr.mean())
    rng = np.random.default_rng(42)
    samples = rng.choice(arr, size=(n_boot, arr.size), replace=True).mean(axis=1)
    lo = float(np.percentile(samples, 100 * alpha / 2))
    hi = float(np.percentile(samples, 100 * (1 - alpha / 2)))
    return mean, lo, hi


def summary_stats(df: pd.DataFrame) -> dict:
    valid = df[~df["is_error"]]
    overall_mean, overall_lo, overall_hi = bootstrap_ci(valid["elicited"].tolist())
    per_category = []
    for cat in sorted(valid["category"].unique()):
        sub = valid[valid["category"] == cat]
        m, lo, hi = bootstrap_ci(sub["elicited"].tolist())
        per_category.append({
            "category": cat, "n": int(len(sub)),
            "mean": m, "ci_lo": lo, "ci_hi": hi,
            "elicited_fraction": float(sub["elicited"].mean()),
            "partial_fraction": float(sub["partial"].mean()),
            "refused_fraction": float(sub["refused"].mean()),
        })
    per_model = []
    for model in sorted(valid["model"].unique()):
        sub = valid[valid["model"] == model]
        m, lo, hi = bootstrap_ci(sub["elicited"].tolist())
        per_model.append({"model": model, "n": int(len(sub)),
                          "elicitation_rate": m, "ci_lo": lo, "ci_hi": hi})
    return {
        "n_total": int(len(df)), "n_valid": int(len(valid)),
        "n_judge_errors": int(df["is_error"].sum()),
        "overall_elicitation_rate": overall_mean,
        "overall_ci": [overall_lo, overall_hi],
        "per_category": per_category, "per_model": per_model,
    }


def make_csv(df: pd.DataFrame) -> None:
    valid = df[~df["is_error"]]
    grouped = (
        valid.groupby(["model", "category"])
        .agg(n=("elicited", "size"),
             elicitation_rate=("elicited", "mean"),
             partial_rate=("partial", "mean"),
             refused_rate=("refused", "mean"),
             mean_latency_ms=("latency_ms", "mean"))
        .reset_index()
    )
    grouped.to_csv(RESULTS / "elicitation_by_model.csv", index=False)


def write_elicitation_table_tex(path: Path) -> None:
    df = pd.read_csv(RESULTS / "elicitation_by_model.csv")
    pivot = df.pivot(index="category", columns="model", values="elicitation_rate")
    models = list(pivot.columns)
    lines = [
        r"\begin{table*}[htbp]",
        r"\centering",
        r"\caption{Elicitation rate by model and risk category (mean across $N$ prompts $\times$ repeats; 95\% bootstrap CIs in the artifact).}",
        r"\label{tab:elicitation-rates}",
        r"\begin{tabular}{@{}l" + "c" * len(models) + r"@{}}",
        r"\toprule",
        r"\textbf{Risk Category} & " + " & ".join(rf"\textbf{{{m.split(':')[0]}}}" for m in models) + r" \\",
        r"\midrule",
    ]
    for cat in pivot.index:
        row_vals = " & ".join(f"{v:.2f}" if pd.notna(v) else "--" for v in pivot.loc[cat])
        lines.append(f"{cat.replace('_', ' ')} & {row_vals} \\\\")
    lines += [r"\bottomrule", r"\end{tabular}", r"\end{table*}"]
    path.write_text("\n".join(lines))


def plot_heatmap(df: pd.DataFrame) -> None:
    valid = df[~df["is_error"]]
    pivot = valid.pivot_table(index="category", columns="model", values="elicited", aggfunc="mean")
    fig, ax = plt.subplots(figsize=(7, 5))
    im = ax.imshow(pivot.values, aspect="auto", cmap="RdYlGn_r", vmin=0, vmax=1)
    ax.set_xticks(range(len(pivot.columns)))
    ax.set_xticklabels([c.split(":")[0] for c in pivot.columns], rotation=30, ha="right")
    ax.set_yticks(range(len(pivot.index)))
    ax.set_yticklabels([c.replace("_", " ") for c in pivot.index])
    for i in range(pivot.shape[0]):
        for j in range(pivot.shape[1]):
            v = pivot.values[i, j]
            if pd.notna(v):
                ax.text(j, i, f"{v:.2f}", ha="center", va="center", fontsize=8,
                        color="white" if v > 0.5 else "black")
    cbar = fig.colorbar(im, ax=ax, fraction=0.03)
    cbar.set_label("Elicitation rate")
    fig.tight_layout()
    fig.savefig(RESULTS / "fig_elicitation_heatmap.pdf")
    fig.savefig(RESULTS / "fig_elicitation_heatmap.png", dpi=150)
    plt.close(fig)


def plot_latency(df: pd.DataFrame) -> None:
    valid = df[df["judge_label"].isin(("elicited", "partial", "refused"))]
    fig, ax = plt.subplots(figsize=(6, 4))
    models = sorted(valid["model"].unique())
    data = [valid[valid["model"] == m]["latency_ms"].values for m in models]
    ax.boxplot(data, labels=[m.split(":")[0] for m in models], showfliers=False)
    ax.set_ylabel("Latency (ms)")
    ax.set_yscale("log")
    fig.tight_layout()
    fig.savefig(RESULTS / "fig_latency_by_model.pdf")
    fig.savefig(RESULTS / "fig_latency_by_model.png", dpi=150)
    plt.close(fig)


def cohens_kappa(human_labels: pd.DataFrame, df: pd.DataFrame) -> dict:
    merged = human_labels.merge(
        df[["model", "category", "prompt_id", "repeat", "judge_label"]],
        on=["model", "category", "prompt_id", "repeat"], how="inner",
    )
    if merged.empty:
        return {"n": 0, "kappa": None}
    labels = ["elicited", "partial", "refused"]
    n = len(merged)
    po = float((merged["human_label"] == merged["judge_label"]).mean())
    h = merged["human_label"].value_counts(normalize=True).reindex(labels, fill_value=0.0)
    j = merged["judge_label"].value_counts(normalize=True).reindex(labels, fill_value=0.0)
    pe = float((h * j).sum())
    kappa = (po - pe) / (1 - pe) if pe < 1 else 1.0
    return {"n": int(n), "kappa": float(kappa), "p_observed": po, "p_expected": pe}


def main() -> None:
    df = load()
    print(f"Loaded {len(df)} rows; {df['is_error'].sum()} judge errors")
    make_csv(df)
    stats = summary_stats(df)
    (RESULTS / "summary_stats.json").write_text(json.dumps(stats, indent=2))
    write_elicitation_table_tex(RESULTS / "tab_elicitation_rates.tex")
    plot_heatmap(df)
    plot_latency(df)

    hpath = RESULTS / "human_labels.csv"
    if hpath.exists():
        human = pd.read_csv(hpath)
        kappa = cohens_kappa(human, df)
        (RESULTS / "kappa.json").write_text(json.dumps(kappa, indent=2))
        print(f"Cohen's κ vs human spot-check (n={kappa['n']}): {kappa['kappa']:.3f}")
    else:
        print("No human_labels.csv yet; skip κ.")

    print(f"Overall elicitation rate: {stats['overall_elicitation_rate']:.3f}"
          f" (95% CI {stats['overall_ci'][0]:.3f}--{stats['overall_ci'][1]:.3f})")


if __name__ == "__main__":
    main()
