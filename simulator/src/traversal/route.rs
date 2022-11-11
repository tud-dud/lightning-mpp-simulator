use crate::{traversal::path_finder::Hop, Edge, Node};

#[derive(Debug, Clone)]
pub(crate) struct Route {
    /// Route a payment: one hop for single payments and multiple for multipath
    pub(crate) hops: Vec<Hop>,
}

impl Route {
    /// Computes the weight of an edge as done in [LND](https://github.com/lightningnetwork/lnd/blob/290b78e700021e238f7e6bdce6acc80de8d0a64f/routing/pathfind.go#L263)
    /// Used when searching for the shortest path between two nodes.
    /// assumes src != dest
    fn get_edge_fee(edge: Edge, amount: usize) -> usize {
        let risk_factor = 15;
        let millionths = 1000000;
        let billionths = 1000000000;
        let base_fee = edge.fee_base_msat;
        let prop_fee = amount * edge.fee_proportional_millionths / millionths;
        let time_lock_penalty = amount * edge.cltv_expiry_delta * risk_factor / billionths;
        base_fee + prop_fee + time_lock_penalty
    }

    /// Returns the success probabilty (amt/ cap) of given amount
    fn get_edge_probabilty(edge: Edge, amount: usize) -> usize {
        amount / edge.htlc_maximum_msat
    }
}

#[cfg(test)]
mod tests {}
