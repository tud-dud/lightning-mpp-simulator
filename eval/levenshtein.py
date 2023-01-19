#!/usr/bin/env python3

import matplotlib.pyplot as plt
from constants import *
import matplotlib.patches as mpatches
import seaborn as sns
import matplotlib.ticker as ticker
import os


def plot_edit_distance(
    df,
    output_path,
    colours=[
        COLOUR_MaxProbMulti,
        COLOUR_MinFeeMulti,
    ],
):
    print("Evaluating path distance data.")
    df = df.melt(id_vars=["scenario", "amount"], value_vars=["distance"])
    fig, axes = plt.subplots(ncols=1, nrows=3, sharex=True, figsize=(12, 8))
    axes = axes.flatten()
    plt.tight_layout()
    ax0 = sns.boxplot(
        x="amount",
        y="value",
        hue="scenario",
        data=df,
        showfliers=False,
        hue_order=["MaxProbMulti", "MinFeeMulti"],
        palette=colours,
        ax=axes[0],
        width=0.5,
        boxprops=dict(
            linewidth=0.5,
        ),
        whiskerprops=dict(linestyle="-", linewidth=0.5, color="black"),
    )
    ax0.tick_params("x", labelrotation=45)
    ax0.set_ylabel("Path distance")
    ax0.set_xlabel("")
    ax0.get_legend().remove()

    print("Evaluating distribution of path distances")
    hue_orders = ["MaxProbMulti", "MinFeeMulti"]
    for i in range(0, 2):
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
        ax.set_rasterization_zorder(-10)
        ax.get_legend().remove()
        ax.set_xlabel("")
        ax.tick_params("x", labelrotation=45)
        ax.set_xticklabels(X_TICKS_LABELS)
        #tick_spacing = 5
        #ax.yaxis.set_major_locator(ticker.MultipleLocator(tick_spacing))
        ax.set_ylabel("Path distance")

    l1 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l2 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    plt.xlabel("Payment amount in sat")
    plt.legend(
        handles=[l1, l2],
        bbox_to_anchor=(0.75, 3.5),
        ncol=4,
        fontsize=8,
        frameon=False,
    )
    output_path = os.path.join(output_path, "path_distance.pdf")
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Path distances", output_path))
