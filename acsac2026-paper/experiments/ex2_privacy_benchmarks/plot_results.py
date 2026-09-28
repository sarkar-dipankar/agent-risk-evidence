"""Plot Exp 2 privacy results.

Inputs:  results/privacy_results_v2.csv, results/attack_results.csv
Outputs: results/fig_privacy_utility.pdf
         results/fig_attack_defense.pdf
"""
from __future__ import annotations

from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import pandas as pd

HERE = Path(__file__).parent
R = HERE / "results"


METHOD_LABEL = {
    "suppress": "suppress",
    "generalize": "generalize",
    "generalize+ldiv": "generalize + $\\ell$-div",
    "generalize+dp": "generalize + DP",
}
METHOD_ORDER = ["suppress", "generalize", "generalize+ldiv", "generalize+dp"]
COLORS = {
    "suppress": "#cc4444",
    "generalize": "#3366cc",
    "generalize+ldiv": "#33aa33",
    "generalize+dp": "#aa6600",
}


def plot_utility() -> None:
    df = pd.read_csv(R / "privacy_results_v2.csv")
    sizes = sorted(df["dataset_size"].unique())
    fig, axes = plt.subplots(1, len(sizes), figsize=(3.0 * len(sizes), 3.3), sharey=True)
    if len(sizes) == 1:
        axes = [axes]
    for ax, n in zip(axes, sizes):
        sub = df[df["dataset_size"] == n]
        for m in METHOD_ORDER:
            s = sub[sub["method"] == m].sort_values("k")
            if s.empty:
                continue
            # Replace NaN utility with -0.1 just for visualization marker
            y = s["utility_score"].fillna(-0.1)
            ax.plot(s["k"], y, marker="o", label=METHOD_LABEL[m],
                    color=COLORS[m], linewidth=1.5, markersize=5)
        ax.set_title(f"$n={n}$")
        ax.set_xlabel("$k$")
        ax.axhline(0, color="grey", linewidth=0.5, linestyle=":")
        ax.set_xticks([3, 5, 10, 20])
        ax.set_ylim(-0.2, 1.05)
        ax.grid(True, linewidth=0.3, alpha=0.5)
    axes[0].set_ylabel("Utility (Spearman)")
    axes[-1].legend(loc="lower left", fontsize=8, frameon=False)
    fig.suptitle("Privacy--utility trade-off by defense and dataset size", fontsize=10, y=1.02)
    fig.tight_layout()
    fig.savefig(R / "fig_privacy_utility.pdf", bbox_inches="tight")
    fig.savefig(R / "fig_privacy_utility.png", dpi=150, bbox_inches="tight")
    plt.close(fig)


def plot_attacks() -> None:
    df = pd.read_csv(R / "attack_results.csv")
    sizes = sorted(df["dataset_size"].unique())
    fig, axes = plt.subplots(1, 2, figsize=(8, 3.4), sharey=True)
    for ax, metric, title in zip(
        axes,
        ["linkage_attack", "homogeneity_attack"],
        ["Linkage attack", "Homogeneity attack"],
    ):
        for m in METHOD_ORDER:
            ys = []
            for n in sizes:
                sub = df[(df["dataset_size"] == n) & (df["method"] == m) & (df["k"] == 5)]
                ys.append(float(sub[metric].mean()) if not sub.empty else 0.0)
            ax.plot(sizes, ys, marker="o", label=METHOD_LABEL[m],
                    color=COLORS[m], linewidth=1.5, markersize=5)
        ax.set_title(title)
        ax.set_xscale("log")
        ax.set_xlabel("dataset size $n$")
        ax.set_ylim(-0.02, 0.5)
        ax.grid(True, linewidth=0.3, alpha=0.5)
    axes[0].set_ylabel("Attack success rate")
    axes[-1].legend(fontsize=8, frameon=False)
    fig.suptitle("Attack success vs defense ($k{=}5$)", fontsize=10, y=1.02)
    fig.tight_layout()
    fig.savefig(R / "fig_attack_defense.pdf", bbox_inches="tight")
    fig.savefig(R / "fig_attack_defense.png", dpi=150, bbox_inches="tight")
    plt.close(fig)


if __name__ == "__main__":
    plot_utility()
    plot_attacks()
    print(f"wrote {R / 'fig_privacy_utility.pdf'}")
    print(f"wrote {R / 'fig_attack_defense.pdf'}")
