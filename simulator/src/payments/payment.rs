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
    pub(crate) succeeded: bool,
    pub(crate) min_shard_amt: usize,
    /// Number of parts this payment has been split into
    pub(crate) num_parts: usize,
    /// Paths payment can take
    /// unstable, might change
    pub(crate) paths: Vec<CandidatePath>,
    pub(crate) attempts: usize,
}

#[derive(Debug, Clone)]
pub struct PaymentShard {
    /// The original payment this shard belongs to
    pub(crate) payment_id: PaymentId,
    pub(crate) source: ID,
    pub(crate) dest: ID,
    pub(crate) amount: usize,
    pub(crate) succeeded: bool,
    /// Path the payment took. COntains fee and weight information
    pub(crate) used_path: CandidatePath,
    pub(crate) min_shard_amt: usize,
    pub(crate) attempts: usize,
}

impl Payment {
    pub(crate) fn new(payment_id: PaymentId, source: ID, dest: ID, amount_msat: usize) -> Self {
        Self {
            payment_id,
            source,
            dest,
            amount_msat,
            succeeded: false,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            num_parts: 0,
            paths: Vec::default(),
            attempts: 0,
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
            used_path: CandidatePath::default(),
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            succeeded: payment.succeeded,
            attempts: payment.attempts,
        }
    }

    pub(super) fn to_payment(&self, num_parts: usize) -> Payment {
        Payment {
            payment_id: self.payment_id,
            source: self.source.clone(),
            dest: self.dest.clone(),
            amount_msat: self.amount,
            succeeded: self.succeeded,
            min_shard_amt: self.min_shard_amt,
            num_parts,
            paths: vec![self.used_path.clone()],
            attempts: self.attempts,
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
            succeeded: false,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            paths: Vec::default(),
            num_parts: 0,
            attempts: 0,
        };
        assert_eq!(actual, expected);
        assert_eq!(actual.succeeded, expected.succeeded);
        assert_eq!(actual.min_shard_amt, expected.min_shard_amt);
        assert_eq!(actual.attempts, expected.attempts);
    }

    #[test]
    fn payment_shard_conversion() {
        let id = 1;
        let source = "source".to_string();
        let dest = "dest".to_string();
        let amount = 10000;
        let num_parts = 1;
        let payment = Payment {
            payment_id: id,
            source: source.clone(),
            dest,
            amount_msat: amount,
            succeeded: true,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            paths: vec![CandidatePath::default()],
            num_parts: 0,
            attempts: 1,
        };
        let shard = payment.to_shard(amount);
        assert_eq!(shard.payment_id, id);
        assert_eq!(shard.amount, amount);
        assert_eq!(shard.succeeded, payment.succeeded);
        let actual = shard.to_payment(num_parts);
        assert_eq!(actual.payment_id, payment.payment_id);
        assert_eq!(actual.succeeded, payment.succeeded);
        assert_eq!(actual.attempts, payment.attempts);
    }
}
