#[cfg(not(test))]
use log::{debug, info};
use simlib::{
    graph::Graph, traversal::pathfinding::PathFinder, CandidatePath, Path, RoutingMetric, ID,
};
#[cfg(test)]
use std::{println as info, println as debug};

use crate::io::Diversity;

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
    lambda: f32,
    amount: usize,
    payment_pairs: impl Iterator<Item = (ID, ID)> + Clone,
) -> Diversity {
    let count = payment_pairs.clone().count() as f32;
    info!("Computing graph diversity using {} pairs.", count);
    let mut outstanding = count as usize;
    let mut graph_div = 0.0;
    for comb in payment_pairs {
        info!("{outstanding} computations to go.");
        let (src, dest) = (comb.0, comb.1);
        graph_div +=
            effective_path_diversity(&src, &dest, graph, k, routing_metric, lambda, amount);
        outstanding -= 1;
    }
    let diversity = graph_div / count;
    Diversity { lambda, diversity }
}

/// Effective path diversity (EPD) is an aggregation of path diversities for a selected set of
/// paths between a given node-pair
fn effective_path_diversity(
    source: &ID,
    dest: &ID,
    graph: &Graph,
    k: usize,
    routing_metric: RoutingMetric,
    lambda: f32,
    amount: usize,
) -> f32 {
    debug!("Calculating diversity between {source} and {dest}.");
    let k_shortest_paths = get_shortest_paths(source, dest, graph, k, routing_metric, amount);
    let mut aggregated_div_src_dest = 0.0;
    for idx in 0..k_shortest_paths.len() {
        let mut div_min_path_i = f32::MAX;
        let mut alternate_paths = k_shortest_paths.clone();
        // the base path should be the only item returned by drain
        if let Some(base_path) = alternate_paths.drain(idx..idx + 1).last() {
            for path in alternate_paths {
                let div = path_diversity(&base_path, &path);
                div_min_path_i = f32::min(div_min_path_i, div);
            }
        }
        aggregated_div_src_dest += div_min_path_i;
    }
    debug!("Completed diversity calculation between {source} and {dest}.");
    1.0 - std::f32::consts::E.powf(-lambda * aggregated_div_src_dest)
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
    use approx::*;

    #[test]
    fn shortest_paths() {
        let k = 3;
        let routing_metric = RoutingMetric::MinFee;
        let path = std::path::Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&path).unwrap());
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
    fn calculate_path_diversity() {
        let base_path = vec![
            ("0".to_string(), "01".to_string()),
            ("1".to_string(), "12".to_string()),
            ("2".to_string(), "21".to_string()),
        ];
        let alternate_path = vec![
            ("0".to_string(), "03".to_string()),
            ("3".to_string(), "31".to_string()),
            ("1".to_string(), "15".to_string()),
            ("5".to_string(), "52".to_string()),
            ("2".to_string(), "25".to_string()),
        ];
        let actual = path_diversity(&base_path, &alternate_path);
        let expected = 0.66;
        assert_abs_diff_eq!(actual, expected, epsilon = 0.01f32);
    }

    #[test]
    fn calculate_effective_diversity() {
        let lambda = 0.5;
        let path = std::path::Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&path).unwrap());
        let k = 3;
        let routing_metric = RoutingMetric::MinFee;
        let amount = 10;
        let source = String::from("034");
        let dest = String::from("036");
        let k_shortest_paths =
            get_shortest_paths(&source, &dest, &graph, k, routing_metric, amount);
        // since we only have two paths, each diversity is the min and the sum is equal to k_sd
        let mut agg_diversity = path_diversity(&k_shortest_paths[0], &k_shortest_paths[1]);
        agg_diversity += path_diversity(&k_shortest_paths[1], &k_shortest_paths[0]);
        let expected = 1.0 - std::f32::consts::E.powf(-lambda * agg_diversity);
        let actual =
            effective_path_diversity(&source, &dest, &graph, k, routing_metric, lambda, amount);
        assert_eq!(actual, expected);
    }

    #[test]
    fn calculate_graph_diversity() {
        let path = std::path::Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&path).unwrap());
        let k = 3;
        let lambda = 0.5;
        let amount = 10;
        let routing_metric = RoutingMetric::MinFee;
        let ids = graph.get_node_ids();
        let mut diversity = 0.0;
        let pairs = vec![
            ("034".to_owned(), "025".to_owned()),
            ("025".to_owned(), "036".to_owned()),
            ("035".to_owned(), "034".to_owned()),
        ];
        for pair in &pairs {
            diversity += effective_path_diversity(
                &pair.0,
                &pair.1,
                &graph,
                k,
                routing_metric,
                lambda,
                amount,
            );
        }
        diversity /= ids.len() as f32;
        let expected = Diversity { lambda, diversity };
        let actual =
            total_graph_diversity(&graph, k, routing_metric, lambda, amount, pairs.into_iter());
        assert_eq!(actual, expected);
    }
}
