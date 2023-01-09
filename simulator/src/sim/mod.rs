use crate::{payment::Payment, Adversaries};
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
    pub successful_payments: Vec<Payment>,
    pub failed_payments: Vec<Payment>,
    pub adversaries: Vec<Adversaries>,
}
