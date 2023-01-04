#!/usr/bin/env python3

from re import X
from numpy.random import laplace
import pandas as pd
import matplotlib.pyplot as plt
from seaborn.external.husl import xyz_to_luv
from constants import *
import matplotlib.patches as mpatches
import os
import numpy as np

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
        xlabel="Amount in sats",
        ylabel="Success rate",
        output_path=os.path.join(output_path, "success_rate_line.pdf"),
    )
    scatter_plot(
        df,
        xlabel="Amount in sats",
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
        markersize=3,
        label="Probability/ Multi",
    )
    ax.plot(
        x_ticks,
        mps,
        linestyle=LINESTYLE_MaxProbSingle,
        color=COLOUR_MaxProbSingle,
        marker="o",
        markersize=3,
        label="Probability/ Single",
    )
    ax.plot(
        x_ticks,
        mfm,
        linestyle=LINESTYLE_MinFeeMulti,
        color=COLOUR_MinFeeMulti,
        markersize=3,
        label="Fee/ Multi",
    )
    ax.plot(
        x_ticks,
        mfs,
        linestyle=LINESTYLE_MinFeeSingle,
        color=COLOUR_MinFeeSingle,
        marker="o",
        markersize=3,
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
