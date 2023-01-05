use crate::payment::Payment;
use serde::Serialize;

mod simulator;
pub use simulator::*;

#[derive(Debug, Serialize, Clone)]
pub struct SimResult {
    pub run: u64,
    pub amount: usize,
    pub total_num: usize,
    pub num_succesful: usize,
    pub num_failed: usize,
    pub(crate) percentage_adversaries: usize,
    /// Number of times an adversary was included a payment path
    pub adversary_hits: usize,
    /// Number of times an adversary was included a successful payment path
    pub adversary_hits_succesful: usize,
    pub successful_payments: Vec<Payment>,
    pub failed_payments: Vec<Payment>,
}
