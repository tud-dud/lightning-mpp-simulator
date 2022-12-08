#!/usr/bin/env python3

import pandas as pd
from argparse import ArgumentParser
import os
import json
from pathlib import Path

from constants import *
from success_rate import *
from fees import *
from htlc_attempts import *
from path_length import *


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
                        for path in payment["paths"]:
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
    return transactions_df, htlc_attempts_df, path_len_df


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
    (transactions_df, htlc_attempts_df, path_len_df) = get_transactions_data(data_files)
    plot_success_rate(data_files, output_path)
    plot_fees(
        transactions_df,
        xlabel="Amount in msats",
        ylabel="Fees in msats",
        output_path=os.path.join(output_path, "transaction_fees.pdf"),
    )
    plot_fee_distributions(
        transactions_df, output_path=os.path.join(output_path, "fee_dist.pdf")
    )
    plot_htlc_attempts(
        htlc_attempts_df, output_path=os.path.join(output_path, "htlc_attempts.pdf")
    )
    plot_path_len(path_len_df, output_path=os.path.join(output_path, "path_length.pdf"))
    print("Successfully generated plots.")
