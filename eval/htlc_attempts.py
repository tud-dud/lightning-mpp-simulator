#!/usr/bin/env python3

import matplotlib.pyplot as plt
from constants import *
import matplotlib.patches as mpatches
import seaborn as sns

"""
the total number(runs, scenario, amount) of HTLC attempts performed by the simulation
"""


def plot_htlc_attempts(
    df,
    output_path,
    colours=[
        COLOUR_MaxProbSingle,
        COLOUR_MaxProbMulti,
        COLOUR_MinFeeSingle,
        COLOUR_MinFeeMulti,
    ],
):
    print("Evaluating htlc attempts.")
    df_abs = (
        df.groupby(["amount", "scenario"])["total_htlc_attempts"]
        .sum()
        .unstack()
        .reset_index()
    )
    fig, axes = plt.subplots(ncols=1, nrows=2, sharex=True, figsize=(12, 8))
    x_ticks = []
    x_ticks_labels = []
    for amt in range(0, len(df_abs["amount"])):
        amount = df_abs["amount"][amt]
        x_ticks.append(amt)
        x_ticks_labels.append(amount)
        mpm = df_abs["MaxProbMulti"][amt]
        mps = df_abs["MaxProbSingle"][amt]
        mfm = df_abs["MinFeeMulti"][amt]
        mfs = df_abs["MinFeeSingle"][amt]
        y = [mpm, mps, mfm, mfs]
        axes[0].scatter(y=y[0], x=amt, color=COLOUR_MaxProbMulti)
        axes[0].scatter(y=y[1], x=amt, color=COLOUR_MaxProbSingle)
        axes[0].scatter(y=y[2], x=amt, color=COLOUR_MinFeeMulti)
        axes[0].scatter(y=y[3], x=amt, color=COLOUR_MinFeeSingle)
        y = sorted(y)
        axes[0].plot([amt, amt], [y[0], y[1]], "--", color="gray", alpha=0.5)
        axes[0].plot([amt, amt], [y[1], y[2]], "--", color="gray", alpha=0.5)
        axes[0].plot([amt, amt], [y[2], y[3]], "--", color="gray", alpha=0.5)
    axes[0].set_xticks(x_ticks, x_ticks_labels, rotation=45)
    axes[0].set_yscale("log")
    axes[0].set_ylabel("Total number of HTLC attempts")

    # relative: #successful of total attempts
    df_success = (
        df.groupby(["amount", "scenario"])["successful_htlc_attempts"]
        .sum()
        .unstack()
        .reset_index()
    )
    df_total = (
        df.groupby(["amount", "scenario"])["total_htlc_attempts"]
        .sum()
        .unstack()
        .reset_index()
    )
    x_ticks = []
    x_ticks_labels = []
    for amt in range(0, len(df_success["amount"])):
        amount = df_success["amount"][amt]
        x_ticks.append(amt)
        x_ticks_labels.append(amount)
        mpm = (df_success["MaxProbMulti"][amt] / df_total["MaxProbMulti"][amt]) * 100
        mps = (df_success["MaxProbSingle"][amt] / df_total["MaxProbSingle"][amt]) * 100
        mfm = (df_success["MinFeeMulti"][amt] / df_total["MinFeeMulti"][amt]) * 100
        mfs = (df_success["MinFeeSingle"][amt] / df_total["MinFeeSingle"][amt]) * 100
        y = [mpm, mps, mfm, mfs]
        axes[1].scatter(y=y[0], x=amt, color=COLOUR_MaxProbMulti)
        axes[1].scatter(y=y[1], x=amt, color=COLOUR_MaxProbSingle)
        axes[1].scatter(y=y[2], x=amt, color=COLOUR_MinFeeMulti)
        axes[1].scatter(y=y[3], x=amt, color=COLOUR_MinFeeSingle)
        y = sorted(y)
        axes[1].plot([amt, amt], [y[0], y[1]], "--", color="gray", alpha=0.5)
        axes[1].plot([amt, amt], [y[1], y[2]], "--", color="gray", alpha=0.5)
        axes[1].plot([amt, amt], [y[2], y[3]], "--", color="gray", alpha=0.5)
    axes[1].set_xticks(x_ticks, x_ticks_labels, rotation=45)
    axes[1].tick_params("x", labelrotation=45)
    # axes[1].set_yscale("log")
    axes[1].set_ylabel("Percentage of successful HTLC attempts")

    l1 = mpatches.Patch(color=COLOUR_MaxProbSingle, label="Probability/ Single")
    l2 = mpatches.Patch(color=COLOUR_MaxProbMulti, label="Probability/ Multi")
    l3 = mpatches.Patch(color=COLOUR_MinFeeSingle, label="Fee/ Single")
    l4 = mpatches.Patch(color=COLOUR_MinFeeMulti, label="Fee/ Multi")
    plt.legend(
        handles=[l1, l2, l3, l4],
        bbox_to_anchor=(0.75, 2.3),
        ncol=4,
        fontsize=8,
        frameon=False,
    )
    plt.xlabel("Payment amount in msat")
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("HTLC attempts", output_path))
