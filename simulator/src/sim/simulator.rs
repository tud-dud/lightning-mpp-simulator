use crate::core_types::graph::Graph;
use crate::ID;
use log::info;
use rand::SeedableRng;

pub struct Simulation {
    /// Graph describing LN topology
    graph: Graph,
    /// Payment amount to simulate
    amount: usize,
    /// Sim seed
    run: u64,
}

impl Simulation {
    pub fn new(run: u64, graph: Graph, amount: usize, num_pairs: usize) -> Self {
        let mut rng = crate::RNG.lock().unwrap();
        *rng = SeedableRng::seed_from_u64(run);
        let random_pairs_iter = Self::draw_n_pairs_for_simulation(&graph, num_pairs);
        Self { graph, amount, run }
    }

    pub fn run(&mut self) {
        info!("Starting simulation ")
    }
    fn draw_n_pairs_for_simulation(
        graph: &Graph,
        n: usize,
    ) -> (impl Iterator<Item = (ID, ID)> + Clone) {
        let mut node_pairs: Vec<(ID, ID)> = Vec::with_capacity(n);
        let g = graph.clone();
        (0..n)
            .collect::<Vec<_>>()
            .into_iter()
            .map(move |_| g.clone().get_random_pair_of_nodes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn init_simulator() {
        let seed = 1;
        let amount = 100;
        let pairs = 2;
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let actual = Simulation::new(seed, graph, amount, pairs);
        assert_eq!(actual.amount, amount);
        assert_eq!(actual.run, seed);
    }
}
