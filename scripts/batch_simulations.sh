#!/bin/sh
source $HOME/.bashrc

BIN_DIR=$HOME/lightning-simulator

OUTPUT_DIR=simulation_results_2022_12_01
LOGS_DIR=logs_2022_12_01/
NUM_PAIRS=5000
GRAPH_FILE=data/gossip-20210906_1000UTC.json

cd $BIN_DIR
mkdir $LOGS_DIR
mkdir $OUTPUT_DIR

declare -a SEEDS=($(seq 1 2 20))
printf "%s\n" "${SEEDS[@]}"

cargo build --release --bin batch-simulator
for seed in ${!SEEDS[@]}; do
    LOG_FILE="${LOGS_DIR}run_$seed.log"
    echo $LOG_FILE
    if [[ ! -e $LOG_FILE ]]; then
        touch $LOG_FILE
    fi
    echo "Sarting run $seed with $NUM_PAIRS pairs."
    target/release/batch-simulator $GRAPH_FILE -n $NUM_PAIRS --run $seed -o $OUTPUT_DIR > "$LOG_FILE" 2>&1
done
