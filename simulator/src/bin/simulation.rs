use std::path::PathBuf;

use clap::Parser;
use env_logger::Env;
use lightning_simulator::{core_types::graph, sim::Simulation};
use log::error;

#[derive(clap::Parser)]
#[command(name = "lightning-simulator", version, about)]
struct Cli {
    /// Path to JSON ile describing topology
    graph_file: PathBuf,
    /// The payment anount to be simulated in msats
    #[arg(long, short)]
    amount: usize,
    /// Set the seed for the simulation
    #[arg(long, short, default_value_t = 19)]
    run: u64,
    /// Number of src/dest pairs to use in the simulation
    #[arg(long = "pairs", short = 'n', default_value_t = 1000)]
    num_pairs: usize,
    /// Split the payment and route independently. Default is not to split and send as a single
    /// payment
    #[arg(long = "split", short = 's')]
    split_payments: bool,
    /// Routing finding heuristic to use
    #[arg(long = "path-metric", short = 'p')]
    edge_weight: lightning_simulator::RoutingMetric,
    #[arg(long = "log", short = 'l', default_value = "info")]
    log_level: String,
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
    let paymeny_amt = args.amount;
    let number_of_sim_pairs = args.num_pairs;
    let routing_metric = args.edge_weight;
    let split_payments = if args.split_payments {
        lightning_simulator::PaymentParts::Split
    } else {
        lightning_simulator::PaymentParts::Single
    };
    let graph = match g {
        Ok(graph) => graph::Graph::to_sim_graph(&graph),
        Err(e) => {
            error!("Error in graph file {}. Exiting.", e);
            std::process::exit(-1)
        }
    };
    let mut simulator = Simulation::new(
        seed,
        graph,
        paymeny_amt,
        number_of_sim_pairs,
        routing_metric,
        split_payments,
    );
    simulator.run();
}
