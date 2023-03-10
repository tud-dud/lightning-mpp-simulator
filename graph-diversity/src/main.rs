use clap::Parser;
use env_logger::Env;
use log::{error, info};
use std::path::PathBuf;

mod total_diversity;
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
    verbose: bool,
}

fn main() {
    let args = Cli::parse();
    let log_level = args.log_level;
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", log_level)
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    let output_dir = if let Some(output_dir) = args.output_dir {
        output_dir
    } else {
        PathBuf::from("results")
    };
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
    info!(
        "Graph metrics will be written to {:#?}/ directory.",
        output_dir
    );
    let _total_diversity = total_graph_diversity(&graph, k, routing_metric);
}
