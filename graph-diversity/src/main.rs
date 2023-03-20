use clap::Parser;
use env_logger::Env;
use log::{error, info};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod io;
mod total_diversity;
use io::*;
use total_diversity::*;

#[derive(clap::Parser)]
#[command(name = "graph-diversity", version, about)]
struct Cli {
    /// Path to JSON ile describing topology
    graph_file: PathBuf,
    /// Route finding heuristic to use
    #[arg(long = "path-metric", short = 'p')]
    edge_weight: simlib::RoutingMetric,
    #[arg(long = "log", short = 'l', default_value = "info")]
    log_level: String,
    /// Path to directory in which the results will be stored
    #[arg(long = "out", short = 'o')]
    output_dir: Option<PathBuf>,
    /// Number of shortest paths to compute between each source, dest tuple
    #[arg(long = "num_paths", short = 'k', default_value_t = 20)]
    num_paths: usize,
    /// an experimentally determined constant that scales the utility of this added diversity
    /// lambda > 1 indicates lower marginal utility for additional paths, while a low value
    /// indicates a higher marginal utility for additional path
    #[arg(long = "lambda", short = 'l')]
    lambda: Option<f32>,
    /// The payment volume (in sat) we are trying to route
    #[arg(long = "amount", short = 'a')]
    amount: Option<usize>,
    /// Set the seed for the simulation
    #[arg(long, short, default_value_t = 19)]
    _run: u64,
    verbose: bool,
}

fn main() {
    let args = Cli::parse();
    let log_level = args.log_level;
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", log_level)
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    let routing_metric = args.edge_weight;
    let k = args.num_paths;
    let g = network_parser::from_json_file(std::path::Path::new(&args.graph_file));
    let graph = match g {
        Ok(graph) => simlib::core_types::graph::Graph::to_sim_graph(&graph),
        Err(e) => {
            error!("Error in graph file {}. Exiting.", e);
            std::process::exit(-1)
        }
    };
    let lambdas = if let Some(lambda) = args.lambda {
        vec![lambda]
    } else {
        vec![0.0, 0.2, 0.5, 0.7, 1.0]
    };
    let amounts = if let Some(amount) = args.amount {
        vec![amount]
    } else {
        vec![
        100, 500, 1000, 5000, 10000, 50000, 100000, 500000, 1000000, 5000000, 10000000,]
    };
    let output_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from("diversity-results")
    };
    info!(
        "Graph metrics will be written to {:#?}/ directory.",
        output_dir
    );
    let mut results = Vec::with_capacity(amounts.len());
    for amount in amounts.iter() {
        let div_results = Arc::new(Mutex::new(Vec::with_capacity(lambdas.len())));
        let amount = simlib::to_millisatoshi(*amount);
        lambdas.par_iter().for_each(|lambda| {
            let total_diversity = total_graph_diversity(&graph, k, routing_metric, *lambda, amount);
            div_results.lock().unwrap().push(total_diversity);
        });
        let combi_div_results = if let Ok(d) = div_results.lock() {
            d.clone()
        } else {
            vec![]
        };
        results.push(io::Results {
            amount,
            routing_metric,
            diversity: combi_div_results.into_iter().flatten().collect(),
        });
    }
    Output::write(
        &Output(results),
        format!("{:?}", routing_metric),
        output_dir,
    )
    .unwrap();
}
