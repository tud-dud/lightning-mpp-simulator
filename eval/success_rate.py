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


def plot_success_rate(json_data, output_path):
    print("Evaluating success rate data.")
    d = []
    for json in json_data:
        for j in json:
            run = j["run"]
            scenario = j["scenario"]
            for x in j["reports"]:
                amount = x["amount"]
                total = x["totalNum"]
                successful = x["numSuccesful"]
                success_rate = successful / total
                d.append(
                    {
                        "run": run,
                        "amount": amount,
                        "scenario": scenario,
                        "total": total,
                        "success_rate": success_rate,
                    }
                )
    df = pd.DataFrame(d)
    df = df.groupby(["amount", "scenario"])["success_rate"].median().unstack()
    plot(
        df,
        xlabel="Amount in msats",
        ylabel="Success rate",
        output_path=os.path.join(output_path, "success_rate_line.pdf"),
    )
    scatter_plot(
        df,
        xlabel="Amount in msats",
        ylabel="Success rate",
        output_path=os.path.join(output_path, "success_rate_scatter.pdf"),
    )


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
    plt.xticks(x_ticks, x_ticks_labels, rotation=45)
    l1 = mpatches.Patch(color=COLOUR_MaxProbSingle, label="Probability/ Single")
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    ax.legend(
        # loc="upper center",
        handles=[l1, l2, l3, l4],
        fontsize=8,
        frameon=False,
    )
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.savefig(output_path)
    print("{} written to {}".format(ylabel, output_path))


def plot(
    df,
    ylabel,
    xlabel,
    output_path,
    kind="line",
    colours=[
        COLOUR_MaxProbMulti,
        COLOUR_MaxProbSingle,
        COLOUR_MinFeeMulti,
        COLOUR_MinFeeSingle,
    ],
    linestyles=[
        LINESTYLE_MaxProbMulti,
        LINESTYLE_MaxProbSingle,
        LINESTYLE_MinFeeMulti,
        LINESTYLE_MinFeeSingle,
    ],
    logy=False,
):
    plt.style.use("default")
    plt.minorticks_off()
    cycler = plt.cycler(linestyle=linestyles, color=colours)
    ax = plt.gca()
    ax.set_prop_cycle(cycler)
    ax = df.plot(
        kind=kind,
        logy=logy,
        stacked=False,
        ax=ax,
    )
    ax.legend(
        loc="upper center",
        bbox_to_anchor=(0.5, 1.1),
        ncol=2,
        fontsize=8,
        frameon=False,
    )
    x_ticks = list(range(0, len(df)))
    x_ticks_labels = [
        100,
        500,
        1000,
        5000,
        10000,
        50000,
        100000,
        1000000,
        5000000,
        10000000,
    ]
    # plt.xticks(x_ticks, x_ticks_labels, rotation=45)
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.savefig(output_path)
    print("{} written to {}".format(ylabel, output_path))
