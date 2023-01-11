#!/usr/bin/env python3

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from constants import *
import os


def plot_adversary_hits(json_data, output_path):
    print("Evaluating adversary data.")
    df = prepare_data(json_data)
    abs_df = (
        df.groupby(["amount", "scenario", "percent"])[["total", "total_hits"]]
        .sum()
        .unstack()
    )
    rel_df = (
        df.groupby(["amount", "scenario", "percent"])[["total_succ", "successful_hits"]]
        .sum()
        .unstack()
    )
    plot(
        abs_df,
        output_path=os.path.join(output_path, "adversary_hits_all_payments.pdf"),
    )
    plot(
        rel_df,
        output_path=os.path.join(output_path, "adversary_hits_successful_payments.pdf"),
        key="successful_hits",
        total_key="total_succ",
    )


def plot(
    df,
    output_path,
    key="total_hits",
    total_key="total",
):
    fig, ax = plt.subplots(
        2, 2, sharex=False, sharey=False, constrained_layout=True, figsize=(12, 10)
    )
    plt.tight_layout()
    axes = ax.flatten()
    mpm = list()
    mps = list()
    mfm = list()
    mfs = list()
    vals = []
    amounts = [10000, 50000, 100000, 5000000]
    for amt in amounts:
        for scenario in [
            "MaxProbMulti",
            "MaxProbSingle",
            "MinFeeMulti",
            "MinFeeSingle",
        ]:
            vals = [
                (
                    percent,
                    (
                        df[key][percent][amt][scenario]
                        / df[total_key][percent][amt][scenario]
                    )
                    * 100,
                )
                for percent in range(10, 100, 10)
            ]
            match scenario:
                case "MaxProbSingle":
                    mps.append(vals)
                case "MaxProbMulti":
                    mpm.append(vals)
                case "MinFeeSingle":
                    mfs.append(vals)
                case "MinFeeMulti":
                    mfm.append(vals)

    vals = [mpm, mps, mfm, mfs]
    for i in range(0, len(amounts)):
        axes[i].plot(
            *zip(*mps[i]),
            color=COLOUR_MaxProbSingle,
            linestyle=LINESTYLE_MaxProbSingle,
            label="Probability/ Single",
        )
        plt.subplots_adjust(hspace=0.15)
        axes[i].plot(
            *zip(*mpm[i]),
            color=COLOUR_MaxProbMulti,
            linestyle=LINESTYLE_MaxProbMulti,
            label="Probability/ Multi",
        )
        plt.subplots_adjust(hspace=0.15)
        axes[i].plot(
            *zip(*mfs[i]),
            color=COLOUR_MinFeeSingle,
            linestyle=LINESTYLE_MinFeeSingle,
            label="Fee/ Single",
        )
        plt.subplots_adjust(hspace=0.15)
        axes[i].plot(
            *zip(*mfm[i]),
            color=COLOUR_MinFeeMulti,
            linestyle=LINESTYLE_MinFeeMulti,
            label="Fee/ Multi",
        )
        plt.subplots_adjust(hspace=0.15)
        axes[i].set_title("Payment amount = {} sat".format(f"{amounts[i]:,}"))
        axes[i].set_yticks(np.arange(0, 110, 10))
        axes[i].minorticks_on()
        axes[i].xaxis.set_tick_params(which="minor", bottom=False)
        plt.subplots_adjust(hspace=0.1)
    plt.legend(
        bbox_to_anchor=(0.5, 2.2),
        handlelength=2,
        ncol=4,
        frameon=False,
        fontsize=8,
    )
    ylabel = "% of payments including an adversary"
    fig.text(0.5, 0.01, "% of adversarial nodes.", ha="center", va="center")
    fig.text(0, 0.5, ylabel, rotation=90)
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Adversary hits", output_path))


def prepare_data(json_data):
    d = []
    for json in json_data:
        for j in json:
            run = j["run"]
            scenario = j["scenario"]
            for x in j["reports"]:
                amount = x["amount"]
                total_num = x["totalNum"]
                successful_num = x["numSuccesful"]
                for a in x["adversaries"]:
                    percent = a["percentage"]
                    hits = a["hits"]
                    successful_hits = a["hits_successful"]
                    success_rate = 0
                    if successful_num != 0:
                        # change to nan
                        success_rate = successful_hits / successful_num
                    d.append(
                        {
                            "run": run,
                            "amount": amount,
                            "scenario": scenario,
                            "percent": percent,
                            "total": total_num,
                            "total_succ": successful_num,
                            "total_hits": hits,
                            "successful_hits": successful_hits,
                        }
                    )
    adv_hits_df = pd.DataFrame(d)
    return adv_hits_df
