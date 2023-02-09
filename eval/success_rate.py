#!/usr/bin/env python3

import pandas as pd
import matplotlib.pyplot as plt
from constants import *
import matplotlib.patches as mpatches
import os

"""
Expects a JSON file for each run
Returns a scatter plot with the median success rate per amount and pathfinfing combination
"""

percentages = [1, 2, 3]


def plot_success_rate(json_data, output_path):
    print("Evaluating success rate data.")
    d1 = []
    d2 = []
    for json in json_data:
        for j in json:
            run = j["run"]
            scenario = j["scenario"]
            for x in j["reports"]:
                amount = x["amount"]
                total = x["totalNum"]
                successful = x["numSuccesful"]
                success_rate = successful / total
                d1.append(
                    {
                        "run": run,
                        "amount": amount,
                        "scenario": scenario,
                        "total": total,
                        "success_rate": success_rate,
                    }
                )
                for a in x["adversaries"]:
                    strategy = a["selection_strategy"]
                    for s in a["statistics"]:
                        percent = s["percentage"]
                        if percent in percentages:
                            target = s["targeted_attack"]
                            num_successful = target["num_succesful"]
                            num_failed = target["num_failed"]
                            num_total = num_failed + num_successful
                            success_rate = num_successful / num_total
                            d2.append(
                                {
                                    "run": run,
                                    "amount": amount,
                                    "scenario": scenario,
                                    "strategy": strategy,
                                    "percent": percent,
                                    "total": total,
                                    "success_rate": success_rate,
                                }
                            )

    df_normal = pd.DataFrame(d1)
    df_normal = (
        df_normal.groupby(["amount", "scenario"])["success_rate"].mean().unstack()
    )
    df_attacks = pd.DataFrame(d2)
    df_attacks = (
        df_attacks.groupby(["strategy", "scenario", "amount", "percent"])[
            "success_rate"
        ]
        .mean()
        .unstack()
    )
    plot(
        df_normal,
        xlabel="Amount in sats",
        ylabel="Success rate",
        output_path=os.path.join(output_path, "success_rate_line.pdf"),
    )
    scatter_plot(
        df_normal,
        xlabel="Amount in sats",
        ylabel="Success rate",
        output_path=os.path.join(output_path, "success_rate_scatter.pdf"),
    )
    plot_multiple(
        df_attacks,
        xlabel="Amount in sats",
        ylabel="Success rate",
        output_path=os.path.join(output_path, "success_rate_node_removal.pdf"),
    )


def plot_multiple(
    df,
    xlabel,
    ylabel,
    output_path,
):
    fig, ax = plt.subplots(
        3, 3, sharex=False, sharey=False, constrained_layout=True, figsize=(12, 10)
    )
    axes = ax.flatten()
    x_ticks_labels = []
    strategies = [
        "HighBetweenness",
        "HighDegree",
        "Random",
    ]
    scenarios = [
        "MaxProbMulti",
        "MaxProbSingle",
        "MinFeeMulti",
        "MinFeeSingle",
    ]
    amounts = [
        100,
        500,
        1000,
        5000,
        10000,
        50000,
        100000,
        500000,
        1000000,
        5000000,
        10000000,
    ]

    plot_styles = [
        (
            COLOUR_MinFeeSingle,
            LINESTYLE_MinFeeMulti,
            MARKER_MinFeeSingle,
            "Fee/ Single",
        ),
        (
            COLOUR_MaxProbSingle,
            LINESTYLE_MaxProbSingle,
            MARKER_MaxProbSingle,
            "Probability/ Single",
        ),
        (COLOUR_MinFeeMulti, LINESTYLE_MinFeeMulti, MARKER_MinFeeMulti, "Fee/ Multi"),
        (
            COLOUR_MaxProbMulti,
            LINESTYLE_MaxProbMulti,
            MARKER_MaxProbMulti,
            "Probability/ Multi",
        ),
    ]

    # df [percent, strategy, scenario, amt]
    axis_num = 0
    x_ticks = [length for length in range(0, len(amounts))]
    for st in strategies:
        for pt in percentages:
            for sc in range(0, len(scenarios)):
                y_ticks = [df[pt][st][scenarios[sc]][amount] for amount in amounts]
                (colour, linestyle, marker, label) = plot_styles[sc]
                axes[axis_num].plot(
                    x_ticks,
                    y_ticks,
                    color=colour,
                    linestyle=linestyle,
                    label=label,
                    marker=marker,
                    ms=4,
                )
                axes[axis_num].set_xticks(x_ticks, X_TICKS_LABELS, rotation=45)
                axes[axis_num].tick_params("x", labelrotation=45)
            axis_num += 1
    rows = ["{}".format(row) for row in strategies]
    cols = [f"{col:,} % adversaries removed" for col in percentages]
    for ax1, col in zip(ax[0], cols):
        ax1.set_title(col)
    pad = 1
    for ax2, row in zip(ax[:, 0], rows):
        ax2.annotate(
            row,
            xy=(0, 0.5),
            xytext=(-ax2.yaxis.labelpad - pad, 0),
            xycoords=ax2.yaxis.label,
            textcoords="offset points",
            size="medium",
            rotation=45,
            ha="right",
            va="center",
        )
    plt.tight_layout()
    plt.legend(
        bbox_to_anchor=(0.2, 4.1),
        handlelength=2,
        ncol=4,
        frameon=False,
        fontsize=8,
    )
    fig.subplots_adjust(left=0.1, top=0.95, bottom=0.075)
    fig.text(0.5, -0.025, xlabel, ha="center", va="center")
    fig.text(-0.025, 0.4, ylabel, rotation=90)
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Success rate with attacks", output_path))


def scatter_plot(df, ylabel, xlabel, output_path):
    fig, ax = plt.subplots(1, 1, figsize=(12, 8))
    df = df.reset_index()
    x_ticks = []
    x_ticks_labels = []
    for amt in range(0, len(df["amount"])):
        amount = df["amount"][amt]
        x_ticks.append(amt)
        x_ticks_labels.append(amount)
        mpm = df["MaxProbMulti"][amt]
        mps = df["MaxProbSingle"][amt]
        mfm = df["MinFeeMulti"][amt]
        mfs = df["MinFeeSingle"][amt]
        y = [mpm, mps, mfm, mfs]
        plt.scatter(y=y[0], x=amt, color=COLOUR_MaxProbMulti)
        plt.scatter(y=y[1], x=amt, color=COLOUR_MaxProbSingle)
        plt.scatter(y=y[2], x=amt, color=COLOUR_MinFeeMulti)
        plt.scatter(y=y[3], x=amt, color=COLOUR_MinFeeSingle)
        y = sorted(y)
        plt.plot([amt, amt], [y[0], y[1]], "--", color="gray", alpha=0.5)
        plt.plot([amt, amt], [y[1], y[2]], "--", color="gray", alpha=0.5)
        plt.plot([amt, amt], [y[2], y[3]], "--", color="gray", alpha=0.5)
    plt.xticks(x_ticks, X_TICKS_LABELS, rotation=45)
    l1 = mpatches.Patch(color=COLOUR_MaxProbSingle, label="Probability/ Single")
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    plt.legend(
        handles=[l1, l2, l3, l4],
        bbox_to_anchor=(0.75, 1.05),
        handlelength=2,
        ncol=4,
        frameon=False,
        fontsize=8,
    )
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format(ylabel, output_path))


def plot(
    df,
    ylabel,
    xlabel,
    output_path,
):
    fig, ax = plt.subplots(1, 1, figsize=(12, 8))
    df = df.reset_index()
    x_ticks = []
    x_ticks_labels = []
    mpm = list()
    mps = list()
    mfm = list()
    mfs = list()
    for amt in range(0, len(df["amount"])):
        amount = df["amount"][amt]
        x_ticks.append(amt)
        x_ticks_labels.append(amount)
        mpm.append(df["MaxProbMulti"][amt])
        mps.append(df["MaxProbSingle"][amt])
        mfm.append(df["MinFeeMulti"][amt])
        mfs.append(df["MinFeeSingle"][amt])
    ax.plot(
        x_ticks,
        mpm,
        linestyle=LINESTYLE_MaxProbMulti,
        color=COLOUR_MaxProbMulti,
        marker=MARKER_MaxProbSingle,
        ms=4,
        label="Probability/ Multi",
    )
    ax.plot(
        x_ticks,
        mps,
        linestyle=LINESTYLE_MaxProbSingle,
        color=COLOUR_MaxProbSingle,
        marker=MARKER_MaxProbMulti,
        ms=4,
        label="Probability/ Single",
    )
    ax.plot(
        x_ticks,
        mfm,
        linestyle=LINESTYLE_MinFeeMulti,
        color=COLOUR_MinFeeMulti,
        marker=MARKER_MinFeeMulti,
        ms=4,
        label="Fee/ Multi",
    )
    ax.plot(
        x_ticks,
        mfs,
        linestyle=LINESTYLE_MinFeeSingle,
        color=COLOUR_MinFeeSingle,
        marker=MARKER_MinFeeSingle,
        ms=4,
        label="Fee/ Single",
    )
    ax.set_xticks(x_ticks, X_TICKS_LABELS, rotation=45)
    ax.tick_params("x", labelrotation=45)
    plt.legend(
        bbox_to_anchor=(0.75, 1.05),
        handlelength=2,
        ncol=4,
        frameon=False,
        fontsize=8,
    )
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format(ylabel, output_path))
