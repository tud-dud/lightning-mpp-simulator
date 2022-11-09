use std::fs::File;
use std::io::prelude::Read;
use std::io::BufReader;

/// Basic usage example.
/// Should be executed in the workspace's top level directory because of the path.

fn main() {
    let file_path = "./data/gossip-20220823.json";
    let file = File::open(file_path).expect("Failed to open file");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    let _res = buf_reader
        .read_to_string(&mut contents)
        .expect("Could not read from file");
    let graph = network_parser::from_json_str(&contents);
    match graph {
        Ok(graph) => {
            println!("Number of nodes {}", graph.nodes.len());
            println!("Total number of edges {}", graph.edge_count());
        }
        Err(e) => println!("{:?}", e),
    };
}
