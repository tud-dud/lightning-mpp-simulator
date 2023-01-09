use crate::{
    core_types::graph::Graph, event::*, payment::Payment, sim::SimResult, time::Time, Adversaries,
    Invoice, PaymentId, PaymentParts, RoutingMetric, WeightPartsCombi, ID,
};
use log::{debug, error, info};
use rand::{seq::IteratorRandom, SeedableRng};
use std::collections::{BTreeMap, HashMap};

pub struct Simulation {
    /// Graph describing LN topology
    pub(crate) graph: Graph,
    /// Payment amount to simulate
    pub(crate) amount: usize,
    /// Sim seed
    run: u64,
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
    /// If fraction of adversaries in the simulation is not passed, we simulate 0 to 90% of
    /// adversaries
    pub(crate) fraction_of_adversaries: Option<usize>,
    pub(crate) adversaries: Vec<Adversaries>,
}

impl Simulation {
    pub fn new(
        run: u64,
        graph: Graph,
        amount: usize,
        routing_metric: RoutingMetric,
        payment_parts: PaymentParts,
        fraction_of_adversaries: Option<usize>,
    ) -> Self {
        info!("Initialising simulation...");
        let mut rng = crate::RNG.lock().unwrap();
        *rng = SeedableRng::seed_from_u64(run);
        let event_queue = EventQueue::new();
        let outstanding_invoices: BTreeMap<String, HashMap<usize, Invoice>> = BTreeMap::new();
        let successful_payments = Vec::new();
        Self {
            graph,
            amount,
            run,
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
            fraction_of_adversaries,
            adversaries: vec![],
        }
    }

    pub fn new_batch_simulator(
        run: u64,
        graph: Graph,
        amount: usize,
        weight_parts: WeightPartsCombi,
        fraction_of_adversaries: Option<usize>,
    ) -> Self {
        let (routing_metric, payment_parts) = match weight_parts {
            WeightPartsCombi::MinFeeSingle => (RoutingMetric::MinFee, PaymentParts::Single),
            WeightPartsCombi::MinFeeMulti => (RoutingMetric::MinFee, PaymentParts::Split),
            WeightPartsCombi::MaxProbSingle => (RoutingMetric::MaxProb, PaymentParts::Single),
            WeightPartsCombi::MaxProbMulti => (RoutingMetric::MaxProb, PaymentParts::Split),
        };
        Self::new(
            run,
            graph,
            amount,
            routing_metric,
            payment_parts,
            fraction_of_adversaries,
        )
    }

    pub fn run(&mut self, payment_pairs: impl Iterator<Item = (ID, ID)> + Clone) -> SimResult {
        info!(
            "# Payment pairs = {}, Pathfinding weight = {:?}, Single/MMP payments: {:?}",
            payment_pairs.size_hint().0,
            self.routing_metric,
            self.payment_parts
        );
        let mut now = Time::from_secs(0.0); // start simulation at (0)
        for (src, dest) in payment_pairs {
            let payment_id = self.next_payment_id();
            let invoice = Invoice::new(payment_id, self.amount, &src, &dest);
            self.add_invoice(invoice);
            let payment = Payment::new(payment_id, src, dest, self.amount);
            let event = PaymentEvent::Scheduled { payment };
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
                PaymentEvent::Scheduled { mut payment } => {
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
                PaymentEvent::UpdateFailed { payment } => {
                    self.num_failed += 1;
                    self.failed_payments.push(payment.to_owned());
                }
                PaymentEvent::UpdateSuccesful { payment } => {
                    self.num_successful += 1;
                    self.successful_payments.push(payment.to_owned());
                }
            }
        }
        assert_eq!(
            self.num_successful + self.num_failed,
            self.total_num_payments,
            "Something went wrong. Expected a different number simulation events."
        );
        info!(
            "Completed simulation after {} simulation secs.",
            now.as_secs(),
        );
        info!(
            "# Total payments = {}, # successful {}, # failed = {}.",
            self.total_num_payments, self.num_successful, self.num_failed
        );
        self.eval_adversaries();
        SimResult {
            run: self.run,
            amount: self.amount,
            total_num: self.total_num_payments,
            num_succesful: self.num_successful,
            num_failed: self.num_failed,
            successful_payments: self.successful_payments.clone(),
            failed_payments: self.failed_payments.clone(),
            adversaries: self.adversaries.to_owned(),
        }
    }

    pub fn draw_n_pairs_for_simulation(
        graph: &Graph,
        n: usize,
    ) -> (impl Iterator<Item = (ID, ID)> + Clone) {
        info!("Drawing {} sender-receiver pairs for simulation.", n,);
        let g = graph.clone();
        g.get_random_pairs_of_nodes(n)
    }

    pub fn draw_adversaries(nodes: &[ID], percentage: usize) -> (impl Iterator<Item = ID> + Clone) {
        // safely round upwards
        let num_adv =
            (nodes.len() * percentage) / 100 + (nodes.len() * percentage % 100 != 0) as usize;
        let mut rng = crate::RNG.lock().unwrap();
        nodes
            .iter()
            .cloned()
            .choose_multiple(&mut *rng, num_adv)
            .into_iter()
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

    #[allow(unused)]
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

    fn eval_adversaries(&mut self) {
        let fraction_of_adversaries = if let Some(percent) = self.fraction_of_adversaries {
            vec![percent]
        } else {
            (1..10).map(|v| v * 10).collect() // in percent
        };
        let mut all_payments = self.failed_payments.clone();
        all_payments.extend(self.successful_payments.clone());
        for percent in fraction_of_adversaries {
            let adv: Vec<ID> =
                Simulation::draw_adversaries(&self.graph.get_node_ids(), percent).collect();
            // TODO: Runtime
            for node in self.graph.nodes.iter_mut() {
                if adv.contains(&node.id) {
                    node.is_adversary = true;
                }
            }
            let mut adversary_hits = 0;
            let mut adversary_hits_successful = 0;
            for payment in &all_payments {
                let used_paths = payment.used_paths.to_owned();
                for path in used_paths {
                    // does the path contain any adversaries?
                    // ignore source and dest nodes for now
                    for n in 1..path.path.hops.len() - 1 {
                        let node = path.path.hops[n].0.clone();
                        if self.graph.node_is_an_adversary(&node) {
                            adversary_hits += 1;
                            if payment.succeeded {
                                adversary_hits_successful += 1;
                            }
                        }
                    }
                }
            }
            self.adversaries.push(Adversaries {
                percentage: percent,
                hits: adversary_hits,
                hits_successful: adversary_hits_successful,
            });
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::Path;

    #[test]
    fn init_simulator() {
        let seed = 0;
        let amount = 100;
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let actual = Simulation::new(seed, graph, amount, routing_metric, payment_parts, Some(0));
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
    fn get_adversaries() {
        let path_to_file = Path::new("../test_data/trivial_connected.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let nodes = &graph.get_node_ids();
        let percentage = 100;
        let actual = Simulation::draw_adversaries(nodes, percentage);
        assert_eq!(actual.size_hint(), (nodes.len(), Some(nodes.len())));
        let percentage = 50;
        let actual = Simulation::draw_adversaries(nodes, percentage);
        assert_eq!(actual.size_hint(), (2, Some(2)));
    }

    #[test]
    fn add_invoice() {
        let seed = 1;
        let amount = 100;
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut simulator =
            Simulation::new(seed, graph, amount, routing_metric, payment_parts, Some(0));
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
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut simulator =
            Simulation::new(seed, graph, amount, routing_metric, payment_parts, Some(0));
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
        let path_to_file = Path::new("../test_data/trivial.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(path_to_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut simulator =
            Simulation::new(seed, graph, amount, routing_metric, payment_parts, Some(0));
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

    #[test]
    fn adversary_hits() {
        let fraction_of_adversaries = 100; // all three nodes are adversaries
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let amount_msat = 1000;
        // alice -> bob -> chan
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: true,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        assert!(simulator.send_single_payment(payment));
        simulator.eval_adversaries();
        //assert_eq!(simulator.adversary_hits, 1);
        //assert_eq!(simulator.adversary_hits_succesful, 1);
    }
}
