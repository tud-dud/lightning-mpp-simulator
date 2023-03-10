use itertools::Itertools;
use log::info;
use simlib::{
    graph::Graph, traversal::pathfinding::PathFinder, CandidatePath, Path, RoutingMetric, ID,
};

const LAMBDA: f32 = 0.5;

/// The TGD is he average of the EPD values of all node pairs within that graph
/// a star topology will always have a TGD of 0, while a ring topology will have a TGD of 0.6 given a lambda of 1
pub(crate) fn total_graph_diversity(graph: &Graph, k: usize, routing_metric: RoutingMetric) -> f32 {
    let ids = graph.get_node_ids();
    println!("{}", ids.len());
    let pairs = ids.iter().combinations(2);
    let count = pairs.clone().count() as f32;
    info!("Computing graph diversity using {} pairs.", count);
    let mut graph_div = 0.0;
    for comb in pairs {
        let (src, dest) = (comb[0].clone(), comb[1].clone());
        graph_div += effective_path_diversity(&src, &dest, graph, k, routing_metric);
    }
    info!("Completed graph diversity computation.");
    graph_div / count
}

/// Effective path diversity (EPD) is an aggregation of path diversities for a selected set of
/// paths between a given node-pair
fn effective_path_diversity(
    source: &ID,
    dest: &ID,
    graph: &Graph,
    k: usize,
    routing_metric: RoutingMetric,
) -> f32 {
    let aggregated_diversities = 1.0;

    let epd = 1.0 - std::f32::consts::E.powf(-LAMBDA * aggregated_diversities);
    let k_shortest_paths = get_shortest_paths(source, dest, graph, k, routing_metric);
    for idx in 0..k_shortest_paths.len() {
        let alternate_paths = k_shortest_paths.clone();
        let base_path = k_shortest_paths.clone().remove(idx);
        for path in alternate_paths {
            let _div = path_diversity(&base_path, &path);
        }
    }
    epd
}

fn path_diversity(base_path: &[(ID, String)], alternate_path: &[(ID, String)]) -> f32 {
    let base_path = simlib::Simulation::get_intermediate_node_and_edges(base_path);
    let alternate_path = simlib::Simulation::get_intermediate_node_and_edges(alternate_path);
    1.0 - (base_path.intersection(&alternate_path).count() as f32 / base_path.len() as f32)
}

fn get_shortest_paths(
    source: &ID,
    dest: &ID,
    graph: &Graph,
    k: usize,
    routing_metric: simlib::RoutingMetric,
) -> Vec<Vec<(ID, String)>> {
    let mut shortest_paths = vec![];
    let graph_copy = graph.clone();
    let mut path_finder = PathFinder::new(
        source.clone(),
        dest.clone(),
        0,
        &graph_copy,
        routing_metric,
        simlib::PaymentParts::Single,
    );
    let k_shortest_paths = path_finder.k_shortest_paths_from(source, k);
    for p in k_shortest_paths {
        let mut path = Path::new(source.clone(), dest.clone());
        path.hops =
            p.0.into_iter()
                .map(|h| (h, usize::default(), usize::default(), String::default()))
                .collect();
        let mut candidate_path = CandidatePath::new_with_path(path);
        path_finder.get_aggregated_path_cost(&mut candidate_path, false);
        // add the edges
        let links: Vec<(ID, String)> = candidate_path
            .path
            .hops
            .iter()
            .map(|h| (h.0.clone(), h.3.clone()))
            .collect();
        shortest_paths.push(links);
    }
    shortest_paths
}
