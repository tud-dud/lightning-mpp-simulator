use std::path::PathBuf;

use clap::Parser;
use env_logger::Env;
use log::{error, info};
use simlib::{core_types::graph, sim::Simulation, AdversarySelection};

#[derive(clap::Parser)]
#[command(name = "lightning-simulator", version, about)]
struct Cli {
    /// Path to JSON ile describing topology
    graph_file: PathBuf,
    /// The payment anount to be simulated in sat
    #[arg(long, short)]
    amount: usize,
    /// Set the seed for the simulation
    #[arg(long, short, default_value_t = 19)]
    run: u64,
    /// Number of src/dest pairs to use in the simulation
    #[arg(long = "pairs", short = 'n', default_value_t = 1000)]
    num_pairs: usize,
    /// Percentage of adversarial nodes
    #[arg(long = "adversaries", short = 'm')]
    num_adv: Option<usize>,
    /// Split the payment and route independently. Default is not to split and send as a single
    /// payment
    #[arg(long = "split", short = 's')]
    split_payments: bool,
    /// Route finding heuristic to use
    #[arg(long = "path-metric", short = 'p')]
    edge_weight: simlib::RoutingMetric,
    #[arg(long = "log", short = 'l', default_value = "info")]
    log_level: String,
    /// Path to directory in which the results will be stored
    #[arg(long = "out", short = 'o')]
    output_dir: Option<PathBuf>,
    /// Path to file containing betweenness scores
    #[arg(short = 'b', long = "betweenness")]
    betweenness_file: Option<PathBuf>,
    /// Path to file containing betweenness scores
    #[arg(short = 'd', long = "degree")]
    degree_file: Option<PathBuf>,
    /// Select adversaries using random sampling
    #[arg(long = "random")]
    random_selection: bool,
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
    let payment_amt = simlib::to_millisatoshi(args.amount);
    let number_of_sim_pairs = args.num_pairs;
    let number_of_adversaries = args.num_adv;
    let routing_metric = args.edge_weight;
    let split_payments = if args.split_payments {
        simlib::PaymentParts::Split
    } else {
        simlib::PaymentParts::Single
    };
    let graph = match g {
        Ok(graph) => graph::Graph::to_sim_graph(&graph),
        Err(e) => {
            error!("Error in graph file {}. Exiting.", e);
            std::process::exit(-1)
        }
    };
    let output_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from("results")
    };
    info!(
        "Simulation results will be written to {:#?}/ directory.",
        output_dir
    );
    let mut adversary_selection = match args.betweenness_file {
        Some(file) => vec![AdversarySelection::HighBetweenness(file)],
        None => vec![],
    };
    if let Some(file) = args.degree_file {
        adversary_selection.push(AdversarySelection::HighDegree(file));
    };
    if args.random_selection {
        adversary_selection.push(AdversarySelection::Random);
    };

    let mut simulator = Simulation::new(
        seed,
        graph.clone(),
        payment_amt,
        routing_metric,
        split_payments,
        number_of_adversaries,
        &adversary_selection,
    );
    let pairs = Simulation::draw_n_pairs_for_simulation(&graph, number_of_sim_pairs);
    _ = simulator.run(pairs);
}
