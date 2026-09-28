"""Plot Exp 3 attestation results: throughput and tamper-detection latency."""
from __future__ import annotations

from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import pandas as pd

HERE = Path(__file__).parent
R = HERE / "results"


def main() -> None:
    df = pd.read_csv(R / "attestation_combined.csv")
    # Use sign throughput vs batch
    fig, axes = plt.subplots(1, 2, figsize=(8, 3.4))

    sign = df[df["op"].isin(["sign", "merkle"])]
    for impl, color in (("rust", "#2255bb"), ("python", "#cc3333")):
        for op, marker in (("sign", "o"), ("merkle", "s")):
            sub = sign[(sign["impl"] == impl) & (sign["op"] == op)].sort_values("batch")
            if sub.empty:
                continue
            axes[0].plot(sub["batch"], sub["throughput_rec_s"], marker=marker,
                         color=color, label=f"{impl} {op}", linewidth=1.5, markersize=5)
    axes[0].set_xscale("log")
    axes[0].set_yscale("log")
    axes[0].set_xlabel("batch size")
    axes[0].set_ylabel("records/second")
    axes[0].set_title("Sign and Merkle-build throughput")
    axes[0].legend(fontsize=8)
    axes[0].grid(True, linewidth=0.3, alpha=0.5)

    # Tamper detection latency vs batch
    tamper = df[df["op"] == "tamper"]
    for impl, color in (("rust", "#2255bb"), ("python", "#cc3333")):
        sub = tamper[tamper["impl"] == impl].sort_values("batch")
        if sub.empty:
            continue
        xs = sub["batch"].values
        ys = sub["median_ms"].values
        # Rust tamper is logged at batch=0 (constant); we plot it as a horizontal line.
        if impl == "rust" and all(b == 0 for b in xs):
            axes[1].axhline(float(ys[0]), color=color, linewidth=1.5,
                            label="rust (proof verify, $O(\\log n)$)")
        else:
            axes[1].plot(xs, ys, marker="o", color=color,
                         label=f"{impl} (full re-hash)", linewidth=1.5, markersize=5)
    axes[1].set_xscale("log")
    axes[1].set_yscale("log")
    axes[1].set_xlabel("batch size")
    axes[1].set_ylabel("tamper-detect latency (ms)")
    axes[1].set_title("Tamper detection")
    axes[1].legend(fontsize=8)
    axes[1].grid(True, linewidth=0.3, alpha=0.5)

    fig.tight_layout()
    fig.savefig(R / "fig_attestation_perf.pdf", bbox_inches="tight")
    fig.savefig(R / "fig_attestation_perf.png", dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"wrote {R / 'fig_attestation_perf.pdf'}")


if __name__ == "__main__":
    main()
