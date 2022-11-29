use crate::{payment::Payment, WeightPartsCombi};
use serde::Serialize;

pub struct Report {
    pub run: u64,
    pub amount: usize,
    pub total_num: usize,
    pub successful_payments: Vec<Payment>,
    pub failed_payments: Vec<Payment>,
    pub scenario: WeightPartsCombi,
}

pub mod report;
pub use report::*;
