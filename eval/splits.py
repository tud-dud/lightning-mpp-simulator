#!/usr/bin/env python3

import matplotlib.pyplot as plt
from constants import *
import matplotlib.patches as mpatches
import seaborn as sns
import matplotlib.ticker as ticker


def plot_parts(
    df,
    output_path,
    colours=[
        COLOUR_MaxProbMulti,
        COLOUR_MinFeeMulti,
    ],
):
    print("Evaluating split depth.")
    df_abs = df.melt(id_vars=["scenario", "amount"], value_vars=["num_parts"])
    fig, axes = plt.subplots(ncols=1, nrows=3, sharex=True, figsize=(12, 8))
    axes = axes.flatten()
    plt.tight_layout()
    hue_orders = ["MaxProbMulti", "MinFeeMulti"]
    ax0 = sns.boxplot(
        x="amount",
        y="value",
        hue="scenario",
        data=df_abs,
        showfliers=False,
        hue_order=hue_orders,
        palette=colours,
        ax=axes[0],
        width=0.5,
        boxprops=dict(
            linewidth=0.5,
        ),
        whiskerprops=dict(linestyle="-", linewidth=0.5, color="black"),
    )
    ax0.tick_params("x", labelrotation=45)
    ax0.set_ylabel("Number of parts")
    ax0.set_xlabel("")
    tick_spacing = 5
    ax0.yaxis.set_major_locator(ticker.MultipleLocator(tick_spacing))
    ax0.get_legend().remove()

    for i in range(0, 2):
        ax = sns.boxplot(
            x="amount",
            y="value",
            hue="scenario",
            data=df_abs,
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
            data=df_abs,
            hue_order=[hue_orders[i]],
            ax=ax,
            dodge=True,
            palette=[colours[i]],
            marker="o",
            size=1,
            jitter=True,
            zorder=-20,
        )
        ax.set_rasterization_zorder(-10)
        ax.tick_params("x", labelrotation=45)
        ax.set_ylabel("Number of parts")
        ax.set_xlabel("")
        ax.get_legend().remove()
        ax.set_xticklabels(X_TICKS_LABELS)
        # ax.yaxis.set_major_locator(ticker.MultipleLocator(tick_spacing))

    l1 = mpatches.Patch(
        color=COLOUR_MaxProbSingle, label="Probability/ Single"
    )
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    plt.legend(
        handles=[l1, l2, l3, l4],
        bbox_to_anchor=(0.75, 3.5),
        ncol=4,
        fontsize=8,
        frameon=False,
    )
    plt.xlabel("Payment amount in sat")
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Split depth", output_path))
