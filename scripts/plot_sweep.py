#!/usr/bin/env python3
"""
Plot PnL/Sharpe surface from the parameter sweep CSV.
Usage: python scripts/plot_sweep.py [sweep.csv]
"""
import sys
import csv
import argparse


def main():
    parser = argparse.ArgumentParser(description="Plot sweep results")
    parser.add_argument("csv", nargs="?", default="sweep.csv")
    parser.add_argument(
        "--metric", choices=["sharpe", "total_pnl"], default="sharpe"
    )
    args = parser.parse_args()

    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        _text_fallback(args.csv, args.metric)
        return

    rows = _load(args.csv)
    thresholds = sorted(set(r["threshold"] for r in rows))
    pos_limits = sorted(set(r["position_limit"] for r in rows))

    Z = np.zeros((len(pos_limits), len(thresholds)))
    for r in rows:
        i = pos_limits.index(r["position_limit"])
        j = thresholds.index(r["threshold"])
        Z[i, j] = r[args.metric]

    X, Y = np.meshgrid(thresholds, pos_limits)

    fig, ax = plt.subplots(figsize=(9, 5))
    cs = ax.contourf(X, Y, Z, levels=20, cmap="RdYlGn")
    fig.colorbar(cs, ax=ax, label=args.metric)
    ax.set_xlabel("threshold (ticks)")
    ax.set_ylabel("position_limit")
    ax.set_title(f"TakerStrategy {args.metric} surface")

    out = f"sweep_{args.metric}.png"
    fig.savefig(out, dpi=150, bbox_inches="tight")
    print(f"saved {out}")


def _load(path):
    rows = []
    with open(path, newline="") as f:
        for r in csv.DictReader(f):
            rows.append(
                {
                    "threshold": float(r["threshold"]),
                    "position_limit": int(r["position_limit"]),
                    "sharpe": float(r["sharpe"]),
                    "total_pnl": float(r["total_pnl"]),
                    "max_drawdown": float(r["max_drawdown"]),
                }
            )
    return rows


def _text_fallback(path, metric):
    """Print an ASCII table when matplotlib is unavailable."""
    rows = _load(path)
    thresholds = sorted(set(r["threshold"] for r in rows))
    pos_limits = sorted(set(r["position_limit"] for r in rows))
    grid = {(r["position_limit"], r["threshold"]): r[metric] for r in rows}

    header = f"{'pos_lim':>8}" + "".join(f"{t:>8.1f}" for t in thresholds)
    print(header)
    print("-" * len(header))
    for pl in pos_limits:
        row_str = f"{pl:>8}" + "".join(
            f"{grid.get((pl, t), float('nan')):>8.3f}" for t in thresholds
        )
        print(row_str)


if __name__ == "__main__":
    main()
