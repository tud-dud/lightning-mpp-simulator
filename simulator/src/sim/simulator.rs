use std::time::Instant;

use crate::{
    core_types::graph::Graph, event::*, payment::Payment, time::Time, PaymentParts, RoutingMetric,
    ID,
};
use log::{debug, info};
use rand::SeedableRng;

pub struct Simulation {
    /// Graph describing LN topology
    graph: Graph,
    /// Payment amount to simulate
    amount: usize,
    /// Sim seed
    run: u64,
    /// Number of payments to simulate
    num_pairs: usize,
    /// Fee minimisation or probability maximisation
    routing_metric: RoutingMetric,
    /// Single or multi-path
    payment_parts: PaymentParts,
    /// Queue of events to be simulated
    event_queue: EventQueue,
}

impl Simulation {
    pub fn new(
        run: u64,
        graph: Graph,
        amount: usize,
        num_pairs: usize,
        routing_metric: RoutingMetric,
        payment_parts: PaymentParts,
    ) -> Self {
        info!("Initialising simulation.");
        let mut rng = crate::RNG.lock().unwrap();
        *rng = SeedableRng::seed_from_u64(run);
        let event_queue = EventQueue::new();
        Self {
            graph,
            amount,
            run,
            num_pairs,
            routing_metric,
            payment_parts,
            event_queue,
        }
    }

    // 1. Create and queue payments in event queue
    // 2. Process event queue
    // 3. Evaluate and report simulation results
    pub fn run(&mut self) {
        let random_pairs_iter = Self::draw_n_pairs_for_simulation(&self.graph, self.num_pairs);
        info!("Starting simulation.");
        let mut now = Time::from_secs(0.0); // start simulation at (0)
        for (payment_id, (src, dest)) in random_pairs_iter.enumerate() {
            let payment = Payment::new(payment_id, src, dest, self.amount);
            let event = EventType::ScheduledPayment { payment };
            self.event_queue.schedule(now, event);
            now += Time::from_secs(crate::SIM_DELAY_IN_SECS);
        }
        debug!(
            "Queued {} events for simulation.",
            self.event_queue.queue_length()
        );

        // this is where the actual simulation happens
        while let Some(event) = self.event_queue.next() {
            match event {
                EventType::ScheduledPayment { payment } => {
                    debug!(
                        "Dispatching scheduled payment {} at simulation time = {}.",
                        payment.payment_id,
                        self.event_queue.now()
                    );
                    self.send_payment()
                }
            }
        }
    }

    // TODO
    fn send_payment(&self) {
        let start = Instant::now();

        // call after finding path
        let duration = start.elapsed().as_millis();
    }

    fn draw_n_pairs_for_simulation(
        graph: &Graph,
        n: usize,
    ) -> (impl Iterator<Item = (ID, ID)> + Clone) {
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
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let actual = Simulation::new(seed, graph, amount, pairs, routing_metric, payment_parts);
        assert_eq!(actual.amount, amount);
        assert_eq!(actual.run, seed);
    }

    #[test]
    fn get_n_random_node_pairs() {
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let n = 2;
        let actual = Simulation::draw_n_pairs_for_simulation(&graph, n);
        assert_eq!(actual.size_hint(), (n, Some(n)));
    }
}
