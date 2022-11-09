use crate::{traversal::Route, Node, PaymentId};

#[derive(Debug, Clone)]
pub struct PaymentShard {
    payment_id: PaymentId,
    source: Node,
    dest: Node,
    amt: usize,
    path: Route,
}

impl Eq for PaymentShard {}
impl PartialEq for PaymentShard {
    fn eq(&self, other: &Self) -> bool {
        self.payment_id == other.payment_id
    }
}
