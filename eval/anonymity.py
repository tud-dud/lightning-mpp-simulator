#!/usr/bin/env python3

import matplotlib.pyplot as plt
from constants import *
import os


def plot_anonymity(df_success, df_fail, output_path):
    df, total = df_success[0], df_success[1]
    succ_counts = calculate_probability(df, total)
    df, total = df_fail[0], df_fail[1]
    fail_counts = calculate_probability(df, total, success=False)
    plot(
        succ_counts,
        fail_counts,
        output_path=os.path.join(output_path, "predecessor_guesses.pdf"),
    )


def calculate_probability(df, total_num, success=True):
    counts = {}
    for (scenario, amount, path_len), occurences in df.items():
        total = total_num[scenario, amount]
        if success:
            prob = prob_for_successful(path_len, occurences, total)
        else:
            prob = prob_for_failed(path_len, occurences, total)
        if (scenario, amount) not in counts:
            counts[scenario, amount] = 0
        counts[scenario, amount] += prob
    return counts


def prob_for_successful(path_len, num_occurences, total):
    return (num_occurences / total) * (1 / (path_len - 1))


def prob_for_failed(path_len, num_occurences, total):
    return (num_occurences / total) * (1 / (path_len - 2))


def plot(
    df_succ,
    df_fail,
    output_path,
):
    fig, ax = plt.subplots(
        2, 1, sharex=True, sharey=False, constrained_layout=True, figsize=(12, 10)
    )
    plt.tight_layout()
    axes = ax.flatten()
    mpm = list()
    mps = list()
    mfm = list()
    mfs = list()
    for (scenario, amount), prob in df_succ.items():
        match scenario:
            case "MaxProbSingle":
                mps.append((amount, prob))
            case "MaxProbMulti":
                mpm.append((amount, prob))
            case "MinFeeSingle":
                mfs.append((amount, prob))
            case "MinFeeMulti":
                mfm.append((amount, prob))
    # sort by amount
    mps.sort()
    mpm.sort()
    mfm.sort()
    mfs.sort()
    x_ticks = [i for i in range(0, len(mps))]
    mps = [prob for _, prob in mps]
    mpm = [prob for _, prob in mpm]
    mfs = [prob for _, prob in mfs]
    mfm = [prob for _, prob in mfm]
    axes[0].plot(
        mps,
        color=COLOUR_MaxProbSingle,
        linestyle=LINESTYLE_MaxProbSingle,
        label="Probability/ Single",
        marker=MARKER_MaxProbSingle,
        ms=4,
    )
    axes[0].plot(
        mpm,
        color=COLOUR_MaxProbMulti,
        linestyle=LINESTYLE_MaxProbMulti,
        label="Probability/ Multi",
        marker=MARKER_MaxProbMulti,
        ms=4,
    )
    axes[0].plot(
        mfs,
        color=COLOUR_MinFeeSingle,
        linestyle=LINESTYLE_MinFeeSingle,
        label="Fee/ Single",
        marker=MARKER_MinFeeSingle,
        ms=4,
    )
    axes[0].plot(
        mfm,
        color=COLOUR_MinFeeMulti,
        linestyle=LINESTYLE_MinFeeMulti,
        label="Fee/ Multi",
        marker=MARKER_MinFeeMulti,
        ms=4,
    )
    axes[0].set_ylabel("Probability for successful payments")
    mpm = list()
    mps = list()
    mfm = list()
    mfs = list()
    for (scenario, amount), prob in df_fail.items():
        match scenario:
            case "MaxProbSingle":
                mps.append((amount, prob))
            case "MaxProbMulti":
                mpm.append((amount, prob))
            case "MinFeeSingle":
                mfs.append((amount, prob))
            case "MinFeeMulti":
                mfm.append((amount, prob))
    # sort by amount
    mps.sort()
    mpm.sort()
    mfm.sort()
    mfs.sort()
    mps = [prob for _, prob in mps]
    mpm = [prob for _, prob in mpm]
    mfs = [prob for _, prob in mfs]
    mfm = [prob for _, prob in mfm]
    axes[1].plot(
        x_ticks,
        mps,
        color=COLOUR_MaxProbSingle,
        linestyle=LINESTYLE_MaxProbSingle,
        label="Probability/ Single",
        marker=MARKER_MaxProbSingle,
        ms=4,
    )
    axes[1].plot(
        x_ticks,
        mpm,
        color=COLOUR_MaxProbMulti,
        linestyle=LINESTYLE_MaxProbMulti,
        label="Probability/ Multi",
        marker=MARKER_MaxProbMulti,
        ms=4,
    )
    axes[1].plot(
        x_ticks,
        mfs,
        color=COLOUR_MinFeeSingle,
        linestyle=LINESTYLE_MinFeeSingle,
        label="Fee/ Single",
        marker=MARKER_MinFeeSingle,
        ms=4,
    )
    axes[1].plot(
        x_ticks,
        mfm,
        color=COLOUR_MinFeeMulti,
        linestyle=LINESTYLE_MinFeeMulti,
        label="Fee/ Multi",
        marker=MARKER_MinFeeMulti,
        ms=4,
    )
    axes[1].set_xticks(x_ticks, X_TICKS_LABELS, rotation=45)
    axes[1].set_ylabel("Probability for failed payments")
    plt.legend(
        bbox_to_anchor=(0.75, 2.15),
        handlelength=2,
        ncol=4,
        frameon=False,
        fontsize=8,
    )
    plt.xlabel("Payment amount in sat")
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Anonymity", output_path))
