mod adversaries;
mod deanonymisation;
mod distance;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct AnonymitySet {
    /// Possible senders
    sender: usize,
    /// Possible recipients
    recipient: usize,
}
