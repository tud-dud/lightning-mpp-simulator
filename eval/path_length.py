#!/usr/bin/env python3

import matplotlib.pyplot as plt
from constants import *
import matplotlib.patches as mpatches
import seaborn as sns
import numpy as np


def plot_path_len(
    df,
    output_path,
    colours=[
        COLOUR_MaxProbSingle,
        COLOUR_MaxProbMulti,
        COLOUR_MinFeeSingle,
        COLOUR_MinFeeMulti,
    ],
):
    print("Evaluating path length data.")
    df = df.melt(id_vars=["scenario", "amount"], value_vars=["path_len"])
    fig, axes = plt.subplots(ncols=1, nrows=5, sharex=True, figsize=(12, 10))
    axes = axes.flatten()
    plt.tight_layout()
    ax0 = sns.boxplot(
        x="amount",
        y="value",
        hue="scenario",
        data=df,
        showfliers=False,
        hue_order=["MaxProbSingle", "MaxProbMulti", "MinFeeSingle", "MinFeeMulti"],
        palette=colours,
        ax=axes[0],
        width=0.5,
        boxprops=dict(
            linewidth=0.5,
        ),
        whiskerprops=dict(linestyle="-", linewidth=0.5, color="black"),
    )
    ax0.tick_params("x", labelrotation=45)
    ax0.set_ylabel("Path length (hop count)")
    ax0.set_xlabel("")
    ax0.get_legend().remove()

    print("Evaluating distribution of path lengths")
    # distributions
    yticks = [
        np.arange(0, 45, 10),
        np.arange(0, 45, 10),
        np.arange(0, 85, 10),
        np.arange(0, 85, 10),
    ]
    hue_orders = ["MaxProbSingle", "MaxProbMulti", "MinFeeSingle", "MinFeeMulti"]
    for i in range(0, 4):
        ax = sns.boxplot(
            x="amount",
            y="value",
            hue="scenario",
            data=df,
            showfliers=False,
            hue_order=[hue_orders[i]],
            palette=colours,
            ax=axes[i + 1],
            width=0.4,
            boxprops=dict(
                linewidth=0.5,
                color=colours[i],
                alpha=0.6,
            ),
            whiskerprops=dict(linestyle="--", linewidth=0.5, color="black"),
        )
        sns.stripplot(
            x="amount",
            y="value",
            hue="scenario",
            data=df,
            hue_order=[hue_orders[i]],
            ax=ax,
            dodge=True,
            palette=[colours[i]],
            marker="o",
            size=2,
            jitter=True,
            zorder=-20,
        )
        ax.set_yticks(yticks[i])
        ax.set_rasterization_zorder(-10)
        ax.get_legend().remove()
        ax.set_xlabel("")
        ax.tick_params("x", labelrotation=45)
        ax.set_xticklabels(X_TICKS_LABELS)
        ax.set_ylabel("Hop count")

    l1 = mpatches.Patch(color=COLOUR_MaxProbSingle, label="Probability/ Single")
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    plt.xlabel("Payment amount in msat")
    plt.legend(
        handles=[l1, l2, l3, l4],
        bbox_to_anchor=(0.75, 5.9),
        ncol=4,
        fontsize=8,
        frameon=False,
    )
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Path length", output_path))
