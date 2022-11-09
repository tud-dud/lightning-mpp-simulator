use std::fs::File;
use std::io::prelude::Read;
use std::io::BufReader;

use lightning_simulator::graph::*;

use env_logger::Env;
use log::info;

/// Basic usage example.
/// Should be executed in the workspace's top level directory because of the path.

fn main() {
    let log_level = "debug";
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", log_level)
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    let file_path = "./data/gossip-20210906_1000UTC.json";
    info!("Parsing graph from {}.", file_path);
    let file = File::open(file_path).expect("Failed to open file");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    let _res = buf_reader
        .read_to_string(&mut contents)
        .expect("Could not read from file");
    let graph = network_parser::from_json_str(&contents);
    info!("Got graph");
    match graph {
        Ok(graph) => {
            let graph = Graph::to_sim_graph(&graph);
            info!("to petgraph");
            let greatest_scc = graph.reduce_to_greatest_scc();
            info!("Greatest SCC with {} nodes.", greatest_scc.node_count());
            info!("Greatest SCC with {} edges ", greatest_scc.edge_count());
        }
        Err(e) => println!("{:?}", e),
    };
}
