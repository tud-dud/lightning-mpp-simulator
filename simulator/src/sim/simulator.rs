use crate::{
    core_types::graph::Graph, event::*, payment::Payment, time::Time,
    traversal::pathfinding::PathFinder, Invoice, PaymentId, PaymentParts, RoutingMetric, ID,
};
use log::{debug, error, info};
use rand::SeedableRng;
use std::{
    collections::{BTreeMap, HashMap},
    time::Instant,
};

pub struct Simulation {
    /// Graph describing LN topology
    pub(crate) graph: Graph,
    /// Payment amount to simulate
    amount: usize,
    /// Sim seed
    run: u64,
    /// Number of payments to simulate
    num_pairs: usize,
    /// Fee minimisation or probability maximisation
    pub(crate) routing_metric: RoutingMetric,
    /// Single or multi-path
    pub(crate) payment_parts: PaymentParts,
    /// Queue of events to be simulated
    pub(crate) event_queue: EventQueue,
    /// Assigned to each new payment
    current_payment_id: PaymentId,
    /// Invoices each node has issued; map of <node, <invoice id, invoice>
    outstanding_invoices: BTreeMap<ID, HashMap<usize, Invoice>>,
    total_num_payments: usize,
    pub(crate) num_successful: usize,
    pub(crate) successful_payments: Vec<Payment>,
    pub(crate) num_failed: usize,
    pub(crate) failed_payments: Vec<Payment>,
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
        info!("Initialising simulation...");
        info!(
            "# Payment pairs = {}, Pathfinding weight = {:?}, Single/MMP payments: {:?}",
            num_pairs, routing_metric, payment_parts
        );
        let mut rng = crate::RNG.lock().unwrap();
        *rng = SeedableRng::seed_from_u64(run);
        let event_queue = EventQueue::new();
        let outstanding_invoices: BTreeMap<String, HashMap<usize, Invoice>> = BTreeMap::new();
        let successful_payments = Vec::new();
        Self {
            graph,
            amount,
            run,
            num_pairs,
            routing_metric,
            payment_parts,
            event_queue,
            current_payment_id: 0,
            outstanding_invoices,
            num_successful: 0,
            successful_payments,
            num_failed: 0,
            failed_payments: Vec::new(),
            total_num_payments: 0,
        }
    }

    // 1. Create and queue payments in event queue
    //  - create and add invoices for tracking of payments
    // 2. Process event queue
    // 3. Evaluate and report simulation results
    pub fn run(&mut self) {
        info!(
            "Drawing {} sender-receiver pairs for simulation.",
            self.num_pairs
        );
        let random_pairs_iter = Self::draw_n_pairs_for_simulation(&self.graph, self.num_pairs);
        let mut now = Time::from_secs(0.0); // start simulation at (0)
        for (src, dest) in random_pairs_iter {
            let payment_id = self.next_payment_id();
            let invoice = Invoice::new(payment_id, self.amount, &src, &dest);
            self.add_invoice(invoice);
            let payment = Payment::new(payment_id, src, dest, self.amount);
            let event = EventType::ScheduledPayment { payment };
            self.event_queue.schedule(now, event);
            now += Time::from_secs(crate::SIM_DELAY_IN_SECS);
        }
        self.total_num_payments = self.event_queue.queue_length();
        debug!(
            "Queued {} events for simulation.",
            self.event_queue.queue_length()
        );

        info!("Starting simulation.");
        // this is where the actual simulation happens
        while let Some(event) = self.event_queue.next() {
            match event {
                EventType::ScheduledPayment { mut payment } => {
                    debug!(
                        "Dispatching scheduled payment {} at simulation time = {}.",
                        payment.payment_id,
                        self.event_queue.now()
                    );
                    let _ = match self.payment_parts {
                        PaymentParts::Single => self.send_single_payment(&mut payment),
                        PaymentParts::Split => self.send_mpp_payment(&mut payment),
                    };
                }
                EventType::UpdateFailedPayment { payment } => {
                    self.num_failed += 1;
                    self.failed_payments.push(payment.to_owned());
                }
                EventType::UpdateSuccesfulPayment { payment } => {
                    self.num_successful += 1;
                    self.successful_payments.push(payment.to_owned());
                }
            }
        }
        info!(
            "Completed simulation after {} simulation secs.",
            now.as_secs(),
        );
        info!(
            "# Total payments = {}, # successful {}, # failed = {}.",
            self.total_num_payments, self.num_successful, self.num_failed
        );
    }

    // 1. Split payment into n parts
    //  - observe min amount
    //  2. Find paths for all parts
    //  TODO: Maybe expect a shard
    fn send_mpp_payment(&mut self, mut payment: &mut Payment) -> bool {
        let graph = Box::new(self.graph.clone());
        if graph.get_total_node_balance(&payment.source) < payment.amount_msat {
            // TODO: immediate failure
        }
        let mut path_finder = PathFinder::new(
            payment.source.clone(),
            payment.dest.clone(),
            payment.amount_msat,
            graph,
            self.routing_metric,
            self.payment_parts,
        );

        let start = Instant::now();
        if let Some(candidate_paths) = path_finder.find_path() {
            payment.paths = candidate_paths.clone();
            let duration_in_ms = start.elapsed().as_millis();
            info!("Found paths after {} ms.", duration_in_ms);
            let mut payment_shard = payment.to_shard(payment.amount_msat);
            let success = self.attempt_payment(&mut payment_shard, &candidate_paths);
            if success {
                // TODO
            } else {
                if let Some(split_shard) = payment_shard.split_payment() {
                    let (shard1, shard2) = (split_shard.0, split_shard.1);
                }
            }
        }
        false
    }

    // TODO: pair should be made up of distinct nodes
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

    pub(crate) fn add_invoice(&mut self, invoice: Invoice) {
        // Has this node already issued invoices?
        match self.outstanding_invoices.get_mut(&invoice.destination) {
            Some(node_invoices) => {
                node_invoices.insert(invoice.id, invoice);
            }
            None => {
                let mut node_invoices = HashMap::new();
                node_invoices.insert(invoice.id, invoice.clone());
                self.outstanding_invoices
                    .insert(invoice.destination, node_invoices);
            }
        };
    }

    /// Invoices each node has issued; map of <node, <invoice id, invoice>
    pub(crate) fn get_invoices_for_node(&self, node: &ID) -> Option<&HashMap<usize, Invoice>> {
        match self.outstanding_invoices.get(node) {
            Some(invoices_map) => Some(invoices_map),
            None => None,
        }
    }

    pub(crate) fn remove_invoice(&mut self, invoice: &Invoice) {
        let id = invoice.id;
        match self.outstanding_invoices.get_mut(&invoice.destination) {
            Some(invoices_map) => {
                invoices_map.retain(|k, v| *k != id && v.id != id);
                self.outstanding_invoices.retain(|_, v| !v.is_empty());
            }
            None => error!("Requested invoice with id {} not found.", id),
        };
    }

    fn next_payment_id(&mut self) -> usize {
        let current_id = self.current_payment_id;
        self.current_payment_id += 1;
        current_id
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

    #[test]
    fn add_invoice() {
        let seed = 1;
        let amount = 100;
        let pairs = 2;
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut simulator =
            Simulation::new(seed, graph, amount, pairs, routing_metric, payment_parts);
        let invoice = Invoice::new(
            simulator.next_payment_id(),
            1234,
            &"alice".to_string(),
            &"dina".to_string(),
        );
        simulator.add_invoice(invoice.clone());
        let invoice2 = Invoice::new(
            simulator.next_payment_id(),
            4321,
            &"alice".to_string(),
            &"dina".to_string(),
        );
        simulator.add_invoice(invoice2.clone());
        assert_eq!(simulator.outstanding_invoices.len(), 1);
        let actual = simulator
            .outstanding_invoices
            .get(&"dina".to_owned())
            .unwrap()
            .clone();
        let expected = HashMap::from([(invoice.id, invoice), (invoice2.id, invoice2)]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn get_invoices_for_node() {
        let seed = 1;
        let amount = 100;
        let pairs = 2;
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut simulator =
            Simulation::new(seed, graph, amount, pairs, routing_metric, payment_parts);
        let invoice = Invoice::new(
            simulator.next_payment_id(),
            1234,
            &"alice".to_string(),
            &"dina".to_string(),
        );
        simulator.add_invoice(invoice.clone());
        let invoice2 = Invoice::new(
            simulator.next_payment_id(),
            4321,
            &"alice".to_string(),
            &"chan".to_string(),
        );
        simulator.add_invoice(invoice2.clone());
        let actual = simulator.get_invoices_for_node(&"dina".to_string());
        assert!(actual.is_some());
        let actual = actual.unwrap().clone();
        let expected = HashMap::from([(invoice.id, invoice)]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn delete_invoice() {
        let seed = 1;
        let amount = 100;
        let pairs = 2;
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut simulator =
            Simulation::new(seed, graph, amount, pairs, routing_metric, payment_parts);
        let invoice = Invoice::new(
            simulator.next_payment_id(),
            1234,
            &"alice".to_string(),
            &"dina".to_string(),
        );
        simulator.add_invoice(invoice.clone());
        simulator.remove_invoice(&invoice);
        let actual = simulator.get_invoices_for_node(&"dina".to_string());
        assert!(actual.is_none());
    }
}
