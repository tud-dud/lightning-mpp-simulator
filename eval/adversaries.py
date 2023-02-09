#!/usr/bin/env python3

import pandas as pd
import matplotlib.pyplot as plt
from constants import *
import os
import seaborn as sns
import numpy as np
from collections import namedtuple, Counter


def plot_adversary_hits(json_data, output_path):
    print("Evaluating adversary data.")
    (hits_df, total_counts_df) = prepare_data(json_data)
    abs_hits_df = (
        hits_df.groupby(["amount", "scenario", "strategy", "percent"])[
            ["strategy", "total", "total_hits"]
        ]
        .sum()
        .unstack()
    )
    rel_hits_df = (
        hits_df.groupby(["amount", "scenario", "strategy", "percent"])[
            ["strategy", "total_succ", "successful_hits"]
        ]
        .sum()
        .unstack()
    )
    plot(
        abs_hits_df,
        output_path=os.path.join(output_path, "adversary_hits_all_payments.pdf"),
    )
    plot(
        rel_hits_df,
        output_path=os.path.join(output_path, "adversary_hits_successful_payments.pdf"),
        key="successful_hits",
        total_key="total_succ",
    )


def plot(
    df,
    output_path,
    key="total_hits",
    total_key="total",
    amounts=[10000, 50000, 100000, 5000000],
):
    fig, ax = plt.subplots(
        4, 3, sharex=False, sharey=False, constrained_layout=True, figsize=(12, 10)
    )
    axes = ax.flatten()
    mpm = dict()
    mps = dict()
    mfm = dict()
    mfs = dict()
    vals = []
    strategies = [
        "Random",
        "HighBetweenness",
        "HighDegree",
    ]
    for amt in amounts:
        for scenario in [
            "MaxProbMulti",
            "MaxProbSingle",
            "MinFeeMulti",
            "MinFeeSingle",
        ]:
            for strategy in strategies:
                vals = [
                    (
                        percent,
                        (
                            df[key][percent][amt][scenario][strategy]
                            / df[total_key][percent][amt][scenario][strategy]
                        )
                        * 100,
                    )
                    for percent in range(1, 5, 1)
                    # for percent in range(1, 11, 1)
                ]
                match scenario:
                    case "MaxProbSingle":
                        if strategy not in mps:
                            mps[strategy] = []
                        mps[strategy].append(vals)
                    case "MaxProbMulti":
                        if strategy not in mpm:
                            mpm[strategy] = []
                        mpm[strategy].append(vals)
                    case "MinFeeSingle":
                        if strategy not in mfs:
                            mfs[strategy] = []
                        mfs[strategy].append(vals)
                    case "MinFeeMulti":
                        if strategy not in mfm:
                            mfm[strategy] = []
                        mfm[strategy].append(vals)

    # strategy, amount, i
    axis_num = 0
    for i in range(0, len(amounts)):
        for s in range(0, len(strategies)):
            axes[axis_num].plot(
                *zip(*mps[strategies[s]][i]),
                color=COLOUR_MaxProbSingle,
                linestyle=LINESTYLE_MaxProbSingle,
                label="Probability/ Single",
                marker=MARKER_MaxProbSingle,
                ms=4,
            )
            axes[axis_num].plot(
                *zip(*mpm[strategies[s]][i]),
                color=COLOUR_MaxProbMulti,
                linestyle=LINESTYLE_MaxProbMulti,
                label="Probability/ Multi",
                marker=MARKER_MaxProbMulti,
                ms=4,
            )
            axes[axis_num].plot(
                *zip(*mfs[strategies[s]][i]),
                color=COLOUR_MinFeeSingle,
                linestyle=LINESTYLE_MinFeeSingle,
                label="Fee/ Single",
                marker=MARKER_MinFeeSingle,
                ms=4,
            )
            axes[axis_num].plot(
                *zip(*mfm[strategies[s]][i]),
                color=COLOUR_MinFeeMulti,
                linestyle=LINESTYLE_MinFeeMulti,
                label="Fee/ Multi",
                marker=MARKER_MinFeeMulti,
                ms=4,
            )
            # axes[axis_num].set_yticks(np.arange(0, 110, 20))
            axes[axis_num].minorticks_on()
            axes[axis_num].xaxis.set_tick_params(which="minor", bottom=False)
            axis_num += 1
    cols = [
        "{} adversaries".format(col)
        for col in ["Random", "High betweenness", "High degree"]
    ]
    rows = [f"{amounts[row]:,} sat" for row in range(0, len(amounts))]
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
        bbox_to_anchor=(0.2, 5),
        handlelength=2,
        ncol=4,
        frameon=False,
        fontsize=8,
    )
    fig.subplots_adjust(left=0.1, top=0.95, bottom=0.075)
    ylabel = "% of payments including an adversary"
    fig.text(0.5, 0.01, "% of adversarial nodes.", ha="center", va="center")
    fig.text(-0.025, 0.4, ylabel, rotation=90)
    plt.savefig(output_path, bbox_inches="tight")
    print("{} written to {}".format("Adversary hits", output_path))


def prepare_data(json_data):
    d = []
    counts_df = []
    successful_counts_df = []
    # NumAttacks = namedtuple("NumAttacks", ["scenario", "amt", "strategy", "percent"])
    num_attacks = {}
    total_num_payments = {}
    total_num_successful_payments = {}
    for json in json_data:
        for j in json:
            run = j["run"]
            scenario = j["scenario"]
            for x in j["reports"]:
                amount = x["amount"]
                total_num = x["totalNum"]
                if (amount, scenario) not in total_num_payments:
                    total_num_payments[(amount, scenario)] = 0
                total_num_payments[(amount, scenario)] += total_num
                successful_num = x["numSuccesful"]
                if (amount, scenario) not in total_num_successful_payments:
                    total_num_successful_payments[(amount, scenario)] = 0
                total_num_successful_payments[(amount, scenario)] += successful_num
                for a in x["adversaries"]:
                    strategy = a["selection_strategy"]
                    for s in a["statistics"]:
                        percent = s["percentage"]
                        hits = s["hits"]
                        successful_hits = s["hits_successful"]
                        success_rate = 0
                        if successful_num != 0:
                            # change to nan
                            success_rate = successful_hits / successful_num
                        # num_att = NumAttacks(scenario=scenario, amt=amount, strategy=strategy, percent=percent)
                        """
                        num_att = (scenario, amount, strategy, percent)
                        if num_att not in num_attacks:
                            num_attacks[num_att] = s["attacked_all"]
                        else:
                            counter1 = Counter(num_attacks[num_att])
                            counter2 = Counter(s["attacked_all"])
                            counter1.update(counter2)
                            num_attacks[num_att] = dict(counter1)
                            #num_attacks[num_att] = dict(Counter(num_attacks[num_att])+Counter(s["attacked_all"]))
                        num_att = (scenario, amount, strategy, percent)
                        """
                        for k, v in s["attacked_all"].items():
                            counts_df.append(
                                {
                                    "run": run,
                                    "scenario": scenario,
                                    "amount": amount,
                                    "strategy": strategy,
                                    "percent": percent,
                                    "times": k,
                                    "num_payments": v,
                                }
                            )
                        d.append(
                            {
                                "run": run,
                                "amount": amount,
                                "scenario": scenario,
                                "strategy": strategy,
                                "percent": percent,
                                "total": total_num,
                                "total_succ": successful_num,
                                "total_hits": hits,
                                "successful_hits": successful_hits,
                            }
                        )
    adv_hits_df = pd.DataFrame(d)
    num_attacks_df = pd.DataFrame(counts_df)
    return (adv_hits_df, (num_attacks_df, total_num_payments))
