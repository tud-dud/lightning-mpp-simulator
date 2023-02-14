#!/usr/bin/env python3

import matplotlib.pyplot as plt
from constants import *
import matplotlib.patches as mpatches
import seaborn as sns
import scienceplots

plt.style.use(["science", "ieee"])


def plot_fees(
    df,
    ylabel,
    output_path,
    colours=[
        COLOUR_MaxProbSingle,
        COLOUR_MaxProbMulti,
        COLOUR_MinFeeSingle,
        COLOUR_MinFeeMulti,
    ],
):
    print("Evaluating transaction fees data.")
    df_abs = df.melt(id_vars=["scenario", "amount"], value_vars=["total_fees"])
    fig, axes = plt.subplots(
        ncols=1,
        nrows=2,
        sharex=True,
    )
    axes = axes.flatten()
    ax0 = sns.boxplot(
        x="amount",
        y="value",
        hue="scenario",
        data=df_abs,
        showfliers=False,
        hue_order=[
            "MaxProbSingle",
            "MaxProbMulti",
            "MinFeeSingle",
            "MinFeeMulti",
        ],
        palette=colours,
        ax=axes[0],
        width=0.5,
        boxprops=dict(
            linewidth=0.5,
        ),
        whiskerprops=dict(linestyle="-", linewidth=0.5, color="black"),
    )
    ax0.set_yscale("log")
    ax0.tick_params("x", labelrotation=90)
    ax0.set_ylabel("")
    ylabel = "Fees in sat"
    fig.text(0.0, 0.525, ylabel, rotation=90)
    ax0.set_xlabel("")
    ax0.get_legend().remove()
    l1 = mpatches.Patch(
        color=COLOUR_MaxProbSingle, label="Probability/ Single"
    )
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")

    """
    plt.legend(
       handles=[l1, l2, l3, l4],
       bbox_to_anchor=(0.75, 2.05),
       ncol=4,
       fontsize=8,
       frameon=False,
    )
    """

    # relative fees
    df_rel = df.melt(
        id_vars=["scenario", "amount"], value_vars=["relative_fees"]
    )
    ax1 = sns.boxplot(
        x="amount",
        y="value",
        hue="scenario",
        data=df_rel,
        showfliers=False,
        hue_order=[
            "MaxProbSingle",
            "MaxProbMulti",
            "MinFeeSingle",
            "MinFeeMulti",
        ],
        palette=colours,
        ax=axes[1],
        width=0.5,
        boxprops=dict(
            linewidth=0.5,
        ),
        whiskerprops=dict(linestyle="-", linewidth=0.5, color="black"),
    )
    ax1.set_yscale("log")
    ax1.tick_params("x", labelrotation=90)
    ax1.set_xticklabels(X_TICKS_LABELS)
    ax1.set_ylabel("")
    ylabel = "Relative fees in sat"
    fig.text(-0.0, 0.1, ylabel, rotation=90)
    ax1.get_legend().remove()

    fig.subplots_adjust(top=0.8)
    plt.legend(
        handles=[l1, l2, l3, l4],
        loc="upper left",
        bbox_to_anchor=(-0.01, 2.475),
        ncol=2,
        handleheight=0.1,
        handlelength=0.1,
    )
    plt.xlabel("Payment amount in sat")
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format(ylabel, output_path))


def plot_fee_distributions(
    df,
    output_path,
    colours=[
        COLOUR_MaxProbSingle,
        COLOUR_MaxProbMulti,
        COLOUR_MinFeeSingle,
        COLOUR_MinFeeMulti,
    ],
):
    print("Evaluating fee distribution data.")
    df_abs = df.melt(id_vars=["scenario", "amount"], value_vars=["total_fees"])
    # _, axes = plt.subplots(ncols=1, nrows=4, sharex=True, figsize=(12, 10))
    fig, axes = plt.subplots(
        ncols=1,
        nrows=4,
        sharex=True,
        figsize=(IEEE_FIG_WIDTH * 2, IEEE_FIG_HEIGHT),
    )
    axes = axes.flatten()
    plt.tight_layout()
    hue_orders = [
        "MaxProbSingle",
        "MaxProbMulti",
        "MinFeeSingle",
        "MinFeeMulti",
    ]
    for i in range(0, 4):
        ax = sns.boxplot(
            x="amount",
            y="value",
            hue="scenario",
            data=df_abs,
            showfliers=False,
            hue_order=[hue_orders[i]],
            palette=colours,
            ax=axes[i],
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
        ax.set_yscale("symlog")
        ax.set_ylim([1e-3, 1e7])
        ax.tick_params("x", labelrotation=90)
        ax.set_ylabel("")
        ax.set_xlabel("")
        ax.get_legend().remove()
        ax.set_xticklabels(X_TICKS_LABELS)
        plt.setp(ax.get_yticklabels()[::2], visible=False)

    ylabel = "Fees in sat"
    fig.text(-0.025, 0.5, ylabel, rotation=90)
    l1 = mpatches.Patch(
        color=COLOUR_MaxProbSingle, label="Probability/ Single"
    )
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    fig.subplots_adjust(top=0.8)
    plt.tight_layout()
    plt.legend(
        handles=[l1, l2, l3, l4],
        loc="lower center",
        bbox_to_anchor=(0.5, 5.25),
        ncol=4,
        handleheight=0.5,
        handlelength=0.25,
        # fontsize=8,
        # frameon=False,
    )
    plt.xlabel("Payment amount in sat")
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Fee distribution", output_path))
