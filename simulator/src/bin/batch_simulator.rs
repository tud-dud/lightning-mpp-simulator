use lightning_simulator::SimResult;
use lightning_simulator::{
    core_types::graph::Graph,
    io::{Output, Results},
    sim::Simulation,
    WeightPartsCombi,
};

use std::path::PathBuf;
use std::{error::Error, time::Instant};

use clap::Parser;
use env_logger::Env;
use log::{error, info};

#[derive(clap::Parser)]
#[command(name = "batch-simulator", version, about)]
struct Cli {
    /// Path to JSON ile describing topology
    graph_file: PathBuf,
    /// Set the seed for the simulation
    #[arg(long, short, default_value_t = 19)]
    run: u64,
    /// Number of src/dest pairs to use in the simulation
    #[arg(long = "pairs", short = 'n', default_value_t = 5000)]
    num_pairs: usize,
    #[arg(long = "log", short = 'l', default_value = "info")]
    log_level: String,
    /// Path to directory in which the results will be stored
    #[arg(long = "out", short = 'o')]
    output_dir: Option<PathBuf>,
    #[arg(long)]
    verbose: bool,
}

fn main() {
    let args = Cli::parse();
    let log_level = args.log_level;
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", log_level)
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    let g = network_parser::from_json_file(std::path::Path::new(&args.graph_file));
    let seed = args.run;
    let number_of_sim_pairs = args.num_pairs;
    let graph = match g {
        Ok(graph) => Graph::to_sim_graph(&graph),
        Err(e) => {
            error!("Error in graph file {}. Exiting.", e);
            std::process::exit(-1)
        }
    };
    let now: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    let timestamp = format!("{}", now.format("%Y_%m_%d_%T"));
    let output_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from(timestamp)
    };
    info!(
        "Simulation results will be written to {:#?}/ directory.",
        output_dir
    );

    let amounts = vec![
        100, 500, 1000, 5000, 10000, 50000, 100000, 5000000, 1000000, 5000000, 10000000,
    ];
    let amounts = vec![100, 500];
    let weight_parts = vec![
        WeightPartsCombi::MinFeeSingle,
        WeightPartsCombi::MaxProbSingle,
        WeightPartsCombi::MinFeeMulti,
        WeightPartsCombi::MaxProbMulti,
    ];
    let pairs = Simulation::draw_n_pairs_for_simulation(&graph, number_of_sim_pairs);
    let mut results = Vec::with_capacity(4);
    for combi in weight_parts {
        let mut sim_results: Vec<SimResult> = Vec::with_capacity(amounts.len());
        for amount in &amounts {
            let start = Instant::now();
            let sim = init_sim(seed, graph.clone(), *amount, combi, number_of_sim_pairs);
            info!(
                "Starting {:?} simulation of {} pairs of {} msats.",
                combi, number_of_sim_pairs, amount
            );
            let sim_result = simulate(sim, pairs.clone());
            let duration_in_ms = start.elapsed().as_millis();
            info!(
                "Simulation {:?} of amount {} completed after {} ms.",
                combi, amount, duration_in_ms
            );
            sim_results.push(sim_result);
        }
        results.push(Output::to_results_type(&sim_results, combi, seed));
    }
    report_to_file(&results, output_dir, seed).expect("Writing to report failed.");
}

fn init_sim(
    seed: u64,
    graph: Graph,
    amount: usize,
    weight_parts: WeightPartsCombi,
    num_pairs: usize,
) -> Simulation {
    Simulation::new_batch_simulator(seed, graph, amount, weight_parts, num_pairs)
}

fn simulate(
    mut sim: Simulation,
    payment_pairs: impl Iterator<Item = (std::string::String, std::string::String)> + Clone,
) -> SimResult {
    sim.run(payment_pairs)
}

fn report_to_file(
    results: &[Results],
    output_dir: PathBuf,
    run: u64,
) -> Result<(), Box<dyn Error>> {
    Output::write(results.to_owned(), output_dir, run)?;
    Ok(())
}
