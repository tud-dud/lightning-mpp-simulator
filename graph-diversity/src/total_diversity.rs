use crate::io::GraphDiversity;
use itertools::Itertools;
#[cfg(not(test))]
use log::{debug, info};
use rayon::prelude::*;
use simlib::{
    graph::Graph, traversal::pathfinding::PathFinder, CandidatePath, Path, RoutingMetric, ID,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::{println as info, println as debug};

/// The TGD [0, 1] is the average of the EPD values of all node pairs within that graph.
/// a star topology will always have a TGD of 0, while a ring topology will have a TGD of 0.6 given
/// a lambda of 1.
/// an EPD of 1 would indicate an infinite diversity) and also reflect the decreasing marginal
/// utility provided by additional paths in real networks.
/// We only calculate the EPDs for the payment pairs we used because of the runtime
pub(crate) fn total_graph_diversity(
    graph: &Graph,
    k: usize,
    routing_metric: RoutingMetric,
    lambdas: &[f32],
    amount: usize,
) -> Vec<GraphDiversity> {
    let mut graph_diversity = vec![];
    let ids = graph.get_node_ids();
    let pairs: Vec<(String, String)> = ids.into_iter().tuple_combinations().collect();
    let count = pairs.len() as f32;
    info!("Computing graph diversity using {} pairs.", count);
    let outstanding = Arc::new(Mutex::new(pairs.len()));
    let div_scores = Arc::new(Mutex::new(Vec::new()));
    pairs.par_iter().for_each(|comb| {
        info!("{} computations to go.", outstanding.lock().unwrap());
        let (src, dest) = (comb.0.clone(), comb.1.clone());
        let diversities =
            effective_path_diversity(&src, &dest, graph, k, routing_metric, lambdas, amount);
        div_scores.lock().unwrap().push(diversities);
        *outstanding.lock().unwrap() -= 1;
    });
    let mut scores: HashMap<(usize, usize), f32> = HashMap::new();
    if let Ok(arc) = Arc::try_unwrap(div_scores) {
        if let Ok(mutex) = arc.into_inner() {
            for d in mutex {
                for (k, v) in d {
                    if let Some(x) = scores.get_mut(&k) {
                        *x += v;
                    } else {
                        scores.insert(k, v);
                    }
                }
            }
        }
    }
    for (k, v) in scores {
        let diversity = v / count;
        let gd = GraphDiversity {
            lambda: lambdas[k.1],
            diversity,
            k: k.0,
        };
        graph_diversity.push(gd);
    }
    info!("Completed graph diversity using {} pairs.", count);
    graph_diversity
}

/// Effective path diversity (EPD) is an aggregation of path diversities for a selected set of
/// paths between a given node-pair
fn effective_path_diversity(
    source: &ID,
    dest: &ID,
    graph: &Graph,
    max_num_paths: usize,
    routing_metric: RoutingMetric,
    lambdas: &[f32],
    amount: usize,
) -> HashMap<(usize, usize), f32> {
    // returns a map of <(k, lambda[pos]), f32>
    let mut k_div = HashMap::with_capacity(max_num_paths);
    debug!("Calculating diversity between {source} and {dest}.");
    let k = if max_num_paths == 20 {
        vec![4, 5, 6, 10]
    } else {
        vec![max_num_paths]
    };
    let k_shortest_paths =
        get_shortest_paths(source, dest, graph, max_num_paths, routing_metric, amount);
    let mut aggregated_div_src_dest = 0.0;
    for num in k {
        if num > k_shortest_paths.len() {
            continue;
        }
        for idx in 0..num {
            let mut div_min_path_i = f32::MAX;
            let mut alternate_paths = k_shortest_paths[0..num].to_vec();
            // the base path should be the only item returned by drain
            if let Some(base_path) = alternate_paths.drain(idx..idx + 1).last() {
                for path in alternate_paths {
                    let div = simlib::sim::Simulation::calculate_path_diversity(&base_path, &path);
                    div_min_path_i = f32::min(div_min_path_i, div);
                }
            }
            aggregated_div_src_dest += div_min_path_i;
        }
        debug!("Completed diversity calculation between {source} and {dest}.");
        for (id, lambda) in lambdas.iter().enumerate() {
            let div = 1.0 - std::f32::consts::E.powf(-lambda * aggregated_div_src_dest);
            k_div.insert((num, id), div);
        }
    }
    k_div
}

fn get_shortest_paths(
    source: &ID,
    dest: &ID,
    graph: &Graph,
    k: usize,
    routing_metric: RoutingMetric,
    amount: usize,
) -> Vec<Vec<(ID, String)>> {
    let mut shortest_paths = vec![];
    let mut graph_copy = graph.clone();
    let mut path_finder = PathFinder::new(
        source.clone(),
        dest.clone(),
        0,
        &graph_copy,
        routing_metric,
        simlib::PaymentParts::Single,
    );
    graph_copy.set_edges(PathFinder::remove_inadequate_edges(&graph_copy, amount));
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn shortest_paths() {
        let k = 3;
        let routing_metric = RoutingMetric::MinFee;
        let path = std::path::Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(
            &network_parser::Graph::from_json_file(&path, network_parser::GraphSource::Lnresearch)
                .unwrap(),
        );
        let amount = 10;
        let pairs = vec![
            ("034".to_owned(), "025".to_owned()),
            ("025".to_owned(), "036".to_owned()),
            ("036".to_owned(), "034".to_owned()),
        ];
        for pair in pairs {
            // only two loopless paths each available
            let k_shortest_paths =
                get_shortest_paths(&pair.0, &pair.1, &graph, k, routing_metric, amount);
            assert_eq!(k_shortest_paths.len(), 2);
        }
    }

    #[test]
    fn calculate_effective_diversity() {
        let lambdas = [0.5];
        let path = std::path::Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(
            &network_parser::Graph::from_json_file(&path, network_parser::GraphSource::Lnresearch)
                .unwrap(),
        );
        let k = 2;
        let routing_metric = RoutingMetric::MinFee;
        let amount = 10;
        let source = String::from("034");
        let dest = String::from("036");
        let k_shortest_paths =
            get_shortest_paths(&source, &dest, &graph, k, routing_metric, amount);
        // since we only have two paths, each diversity is the min and the sum is equal to k_sd
        let mut agg_diversity = simlib::Simulation::calculate_path_diversity(
            &k_shortest_paths[0],
            &k_shortest_paths[1],
        );
        agg_diversity += simlib::Simulation::calculate_path_diversity(
            &k_shortest_paths[1],
            &k_shortest_paths[0],
        );
        let expected = 1.0 - std::f32::consts::E.powf(-lambdas[0] * agg_diversity);
        let epd =
            effective_path_diversity(&source, &dest, &graph, k, routing_metric, &lambdas, amount);
        let actual = epd.get(&(k, 0)).unwrap();
        assert_eq!(*actual, expected);
    }

    #[test]
    fn calculate_graph_diversity() {
        let path = std::path::Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(
            &network_parser::Graph::from_json_file(&path, network_parser::GraphSource::Lnresearch)
                .unwrap(),
        );
        let k = 2;
        let lambdas = [0.5];
        let amount = 10;
        let routing_metric = RoutingMetric::MinFee;
        let ids = graph.get_node_ids();
        let mut diversity = 0.0;
        let pairs = vec![
            ("034".to_owned(), "025".to_owned()),
            ("025".to_owned(), "036".to_owned()),
            ("036".to_owned(), "034".to_owned()),
        ];
        for pair in &pairs {
            let div = effective_path_diversity(
                &pair.0,
                &pair.1,
                &graph,
                k,
                routing_metric,
                &lambdas,
                amount,
            );
            diversity += div.get(&(k, 0)).unwrap();
        }
        diversity /= ids.len() as f32;
        let expected = vec![GraphDiversity {
            lambda: lambdas[0],
            diversity,
            k,
        }];
        let actual = total_graph_diversity(&graph, k, routing_metric, &lambdas, amount);
        assert_eq!(actual, expected);
    }
}
