#!/bin/sh
source $HOME/.bashrc

BIN_DIR=./

OUTPUT_DIR=test_simulation_results
LOGS_DIR=test_logs_2022_12_08/
NUM_PAIRS=20
GRAPH_FILE=data/gossip-20210906_1000UTC.json

cd $BIN_DIR
mkdir $LOGS_DIR
mkdir $OUTPUT_DIR


cargo build --release --bin batch-simulator
for seed in 7 8; do
    LOG_FILE="${LOGS_DIR}run_$seed.log"
    echo $LOG_FILE
    if [[ ! -e $LOG_FILE ]]; then
        touch $LOG_FILE
    fi
    echo "Sarting run $seed with $NUM_PAIRS pairs."
    target/release/batch-simulator $GRAPH_FILE -n $NUM_PAIRS --run $seed -o $OUTPUT_DIR --log trace > "$LOG_FILE" 2>&1
done
