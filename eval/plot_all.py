#!/usr/bin/env python3

import pandas as pd
from argparse import ArgumentParser
import json
from pathlib import Path
from collections import namedtuple

from constants import *
from adversaries import plot_adversary_hits
from success_rate import *
from fees import *
from htlc_attempts import *
from path_length import *
from anonymity import plot_anonymity
from splits import plot_parts


def read_json_files(input_path):
    input_files = []
    json_files = []
    files = Path(input_path).glob("run*.json")
    for filename in files:
        input_files.append(filename)
        with open(filename, "r") as file:
            run_data = json.load(file)
            json_files.append(run_data)
    print("Read {} files".format(len(json_files)))
    return json_files


"""
Expects a JSON file for each run
Returns box plots with the transaction fees per amount and pathfinfing combination
"""


def get_transactions_data(json_data):
    transactions_df = []
    htlc_attempts_df = []
    path_len_df = []
    failed_paths_df = []
    parts_df = []
    PathInf = namedtuple("PathInf", ["scenario", "amt", "length"])
    success_anonymity_df = {}
    fail_anonymity_df = {}
    # number of successful with more than 2 hops per scenario
    # maybe {'scenario, amt": total #successful}
    TotalNum = namedtuple("TotalNum", ["scenario", "amt"])
    num_successful_paths = {}
    num_failed_paths = {}
    for json in json_data:
        for j in json:
            run = j["run"]
            scenario = j["scenario"]
            for r in j["reports"]:
                htlc_attempts = 0
                successful_htlc_attempts = 0
                amount = r["amount"]
                for payment in r["payments"]:
                    htlc_attempts += payment["htlcAttempts"]
                    if payment["succeeded"] is True:
                        successful_htlc_attempts += payment["htlcAttempts"]
                    total_fees = 0
                    total_time = 0
                    if payment["succeeded"] is True:
                        for path in payment["usedPaths"]:
                            total_fees += path["totalFees"]
                            total_time += path["totalTime"]
                            path_len = path["pathLen"]
                            path_len_df.append(
                                {
                                    "run": run,
                                    "amount": amount,
                                    "scenario": scenario,
                                    "path_len": path_len,
                                }
                            )
                            if path_len > 2 and path_len < 21:
                                total_num = TotalNum(scenario=scenario, amt=amount)
                                if total_num not in num_successful_paths:
                                    num_successful_paths[total_num] = 0
                                num_successful_paths[total_num] += 1
                                # add entry of type <(scenario, amt, len)>
                                path_inf = PathInf(
                                    scenario=scenario, amt=amount, length=path_len
                                )
                                if path_inf not in success_anonymity_df:
                                    success_anonymity_df[path_inf] = 0
                                success_anonymity_df[path_inf] += 1
                        transactions_df.append(
                            {
                                "run": run,
                                "amount": amount,
                                "scenario": scenario,
                                "total_fees": total_fees,
                                "relative_fees": total_fees / amount,
                                "total_time": total_time,
                            }
                        )
                        if scenario in ["MaxProbMulti", "MinFeeMulti"]:
                            parts = payment["numParts"]
                            parts_df.append(
                                {
                                    "run": run,
                                    "amount": amount,
                                    "scenario": scenario,
                                    "num_parts": parts,
                                }
                            )
                    else:
                        path_len_df.append(
                            {
                                "run": run,
                                "amount": amount,
                                "scenario": scenario,
                                "path_len": float("nan"),
                            }
                        )
                        transactions_df.append(
                            {
                                "run": run,
                                "amount": amount,
                                "scenario": scenario,
                                "total_fees": float("nan"),
                                "relative_fees": float("nan"),
                                "total_time": float("nan"),
                            }
                        )
                        for path in payment["failedPaths"]:
                            path_len = path["pathLen"]
                            if path_len > 3 and path_len < 21:
                                total_num = TotalNum(scenario=scenario, amt=amount)
                                if total_num not in num_failed_paths:
                                    num_failed_paths[total_num] = 0
                                num_failed_paths[total_num] += 1
                                # add entry of type <(scenario, amt, len)>
                                path_inf = PathInf(
                                    scenario=scenario, amt=amount, length=path_len
                                )
                                if path_inf not in fail_anonymity_df:
                                    fail_anonymity_df[path_inf] = 0
                                fail_anonymity_df[path_inf] += 1
                    for path in payment["failedPaths"]:
                        path_len = path["pathLen"]
                        failed_paths_df.append(
                            {
                                "run": run,
                                "amount": amount,
                                "scenario": scenario,
                                "path_len": path_len,
                            }
                        )
                htlc_attempts_df.append(
                    {
                        "run": run,
                        "amount": amount,
                        "scenario": scenario,
                        "total_htlc_attempts": htlc_attempts,
                        "successful_htlc_attempts": successful_htlc_attempts,
                    }
                )
    transactions_df = pd.DataFrame(transactions_df)
    htlc_attempts_df = pd.DataFrame(htlc_attempts_df)
    path_len_df = pd.DataFrame(path_len_df)
    failed_paths_df = pd.DataFrame(failed_paths_df)
    parts_df = pd.DataFrame(parts_df)
    return (
        transactions_df,
        htlc_attempts_df,
        path_len_df,
        failed_paths_df,
        (success_anonymity_df, num_successful_paths),
        (fail_anonymity_df, num_failed_paths),
        parts_df,
    )


if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument(
        "-i",
        "--input",
        dest="input_dir",
        help="Path to the directory containing the JSON files.",
        required=True,
    )
    parser.add_argument(
        "-o",
        "--output",
        dest="output_dir",
        help="Path to the directory where result files should be stored.",
        default="plots",
    )
    args = parser.parse_args()
    input_path = args.input_dir
    output_path = args.output_dir
    if output_path is None:
        output_path = "./plots"
    Path(output_path).mkdir(parents=True, exist_ok=True)
    print("Reading from ", input_path)
    data_files = read_json_files(input_path)
    print("Generating plots.")
    (
        transactions_df,
        htlc_attempts_df,
        paths_df,
        failed_paths_df,
        success_anonymity_df,
        fail_anonymity_df,
        parts_df,
    ) = get_transactions_data(data_files)
    plot_success_rate(data_files, output_path)
    plot_fees(
        transactions_df,
        xlabel="Amount in sats",
        ylabel="Fees in sats",
        output_path=os.path.join(output_path, "transaction_fees.pdf"),
    )
    plot_fee_distributions(
        transactions_df, output_path=os.path.join(output_path, "fee_dist.pdf")
    )
    plot_htlc_attempts(
        htlc_attempts_df, output_path=os.path.join(output_path, "htlc_attempts.pdf")
    )
    plot_all_paths(paths_df, failed_paths_df, output_path)
    plot_adversary_hits(data_files, output_path)
    plot_anonymity(success_anonymity_df, fail_anonymity_df, output_path)
    plot_parts(parts_df, output_path=os.path.join(output_path, "splits.pdf"))
    print("Successfully generated plots.")
