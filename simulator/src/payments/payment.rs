use crate::{traversal::pathfinding::CandidatePath, PaymentId, ID};

pub(crate) enum Message {
    /// Offer an HTLC to another node
    UpdateAddHtlc {},
    /// Error
    UpdateFailHtlc,
    RevokeAndAck,
    CommitmentSigned,
}

#[derive(Debug, Clone, Default)]
pub struct Payment {
    /// Unique payment identifier
    pub(crate) payment_id: PaymentId,
    pub(crate) source: ID,
    pub(crate) dest: ID,
    /// Total amount issued by this payment (fees + amount)
    pub(crate) amount_msat: usize,
    /// True when the payment fails completely and we give up
    pub(crate) failed: bool,
    pub(crate) current_hop: ID,
    pub(crate) min_shard_amt: usize,
    /// Number of parts this payment has been split into
    pub(crate) num_parts: usize,
    /// Value of highest successful shard
    pub(crate) highest_successful: usize,
    /// Paths payment can take
    /// unstable, might change
    pub(crate) paths: Vec<CandidatePath>,
}

#[derive(Debug, Clone)]
pub struct PaymentShard {
    /// The original payment this shard belongs to
    pub(crate) payment_id: PaymentId,
    pub(crate) source: ID,
    pub(crate) dest: ID,
    pub(crate) amount: usize,
    pub(crate) failed: bool,
    pub(crate) succeeded: bool,
    /// Paths payment can take
    /// unstable, might change
    pub(crate) paths: Vec<CandidatePath>,
    pub(crate) min_shard_amt: usize,
}

impl Payment {
    pub(crate) fn new(payment_id: PaymentId, source: ID, dest: ID, amount_msat: usize) -> Self {
        Self {
            payment_id,
            source: source.clone(),
            dest,
            amount_msat,
            failed: false,
            current_hop: source,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            num_parts: 0,
            highest_successful: 0,
            paths: Vec::default(),
        }
    }

    /// All payments are sent as shards, regardless of mpp or single
    pub(crate) fn to_shard(&self, amount: usize) -> PaymentShard {
        PaymentShard::new(self, amount)
    }
}

impl PaymentShard {
    pub(super) fn new(payment: &Payment, amount: usize) -> Self {
        Self {
            payment_id: payment.payment_id,
            source: payment.source.clone(),
            dest: payment.dest.clone(),
            amount,
            failed: payment.failed,
            paths: payment.paths.clone(),
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            succeeded: false,
        }
    }

    /// Split payment and return two shards
    /// Continue here
    pub(crate) fn split_payment(&self) -> Option<(PaymentShard, PaymentShard)> {
        if self.amount < self.min_shard_amt {
            None
        } else if (self.amount / 2) < self.min_shard_amt {
            None
        // enough balance at sender
        // TODO
        //} else if {
        //None
        } else {
            None
        }
    }
}

impl Eq for PaymentShard {}
impl PartialEq for PaymentShard {
    fn eq(&self, other: &Self) -> bool {
        self.payment_id == other.payment_id
    }
}

impl Eq for Payment {}
impl PartialEq for Payment {
    fn eq(&self, other: &Self) -> bool {
        self.payment_id == other.payment_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_new_payment() {
        let id = 1;
        let source = "source".to_string();
        let dest = "dest".to_string();
        let amount = 10000;
        let actual = Payment::new(id, source.clone(), dest.clone(), amount);
        let expected = Payment {
            payment_id: id,
            source: source.clone(),
            dest,
            amount_msat: amount,
            failed: false,
            current_hop: source,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            paths: Vec::default(),
            num_parts: 0,
            highest_successful: amount,
        };
        assert_eq!(actual, expected);
        assert_eq!(actual.failed, expected.failed);
        assert_eq!(actual.min_shard_amt, expected.min_shard_amt);
    }
}
