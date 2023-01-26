mod adversaries;
mod deanonymisation;
mod distance;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Adversaries {
    pub(crate) selection_strategy: crate::AdversarySelection,
    pub(crate) percentage: usize,
    /// Number of times an adversary was included a payment path
    pub hits: usize,
    /// Number of times an adversary was included a successful payment path
    pub hits_successful: usize,
    pub anonymits_sets: Vec<AnonymitySet>, // one for each payment
}

/// All the distances in the simulated payments' paths
#[derive(Debug, Serialize, Clone)]
pub struct PathDistances(pub Vec<usize>);

#[derive(Debug, Serialize, Clone)]
pub struct AnonymitySet {
    /// Possible senders
    sender: usize,
    /// Possible recipients
    recipient: usize,
}
