use crate::{traversal::pathfinding::CandidatePath, PaymentId, ID};

use log::error;

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
    pub(crate) paths: CandidatePath,
    pub(crate) attempts: usize,
    /// Payment amounts that have already succeed, used for MPP payments
    pub(crate) failed_amounts: Vec<usize>,
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
            num_parts: 1,
            paths: CandidatePath::default(),
            attempts: 0,
            failed_amounts: Vec::default(),
        }
    }

    /// All payments are sent as shards, regardless of mpp or single
    pub(crate) fn to_shard(&self, amount: usize) -> PaymentShard {
        PaymentShard::new(self, amount)
    }

    /// Split payment and return two shards
    pub(crate) fn split_payment(
        payment: &Payment,
        amt_to_split: usize,
    ) -> Option<(Payment, Payment)> {
        if amt_to_split < crate::MIN_SHARD_AMOUNT
            || amt_to_split / 2 < crate::MIN_SHARD_AMOUNT
            || amt_to_split < payment.min_shard_amt
            || amt_to_split / 2 < payment.min_shard_amt
        {
            error!(
                "Payment failing as min shard amount has been reached. Min amount {}, amount {}",
                crate::MIN_SHARD_AMOUNT,
                amt_to_split
            );
            None
        } else if amt_to_split > *payment.failed_amounts.iter().max().unwrap_or(&usize::MAX) {
            error!(
                "Aborting splitting as larger payments have already failed. Amount {}",
                amt_to_split
            );
            None
        } else {
            // ceil one, floor the either
            let prev_amt = amt_to_split;
            let shard1_amount = (prev_amt + 2 - 1) / 2;
            let shard2_amount = prev_amt / 2;
            assert_eq!(
                shard1_amount + shard2_amount,
                amt_to_split,
                "Payment division results unequal to payment amount {}, {}",
                shard1_amount + shard2_amount,
                amt_to_split
            );
            let shard1 = Payment {
                amount_msat: shard1_amount,
                ..payment.clone()
            };
            let shard2 = Payment {
                amount_msat: shard2_amount,
                ..payment.clone()
            };
            Some((shard1, shard2))
        }
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

    pub(crate) fn to_payment(&self, num_parts: usize) -> Payment {
        Payment {
            payment_id: self.payment_id,
            source: self.source.clone(),
            dest: self.dest.clone(),
            amount_msat: self.amount,
            succeeded: self.succeeded,
            min_shard_amt: self.min_shard_amt,
            num_parts,
            paths: self.used_path.clone(),
            attempts: self.attempts,
            failed_amounts: Vec::default(),
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
            paths: CandidatePath::default(),
            num_parts: 1,
            attempts: 0,
            failed_amounts: Vec::default(),
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
            paths: CandidatePath::default(),
            num_parts: 1,
            attempts: 1,
            failed_amounts: Vec::default(),
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

    #[test]
    fn successfully_split() {
        let source = "source".to_string();
        let dest = "dest".to_string();
        let amount = 2001;
        let payment = Payment {
            payment_id: 0,
            source: source.clone(),
            dest,
            amount_msat: amount,
            succeeded: false,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            paths: CandidatePath::default(),
            num_parts: 1,
            attempts: 1,
            failed_amounts: Vec::default(),
        };
        let actual = Payment::split_payment(&payment, payment.amount_msat).unwrap();
        let expected = (
            Payment {
                amount_msat: 1001,
                ..payment.clone()
            },
            Payment {
                amount_msat: 1000,
                ..payment.clone()
            },
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn split_should_fail_due_to_amount() {
        let source = "source".to_string();
        let dest = "dest".to_string();
        let amount = crate::MIN_SHARD_AMOUNT + 1;
        let payment = Payment {
            payment_id: 0,
            source: source.clone(),
            dest,
            amount_msat: amount,
            succeeded: false,
            min_shard_amt: crate::MIN_SHARD_AMOUNT,
            paths: CandidatePath::default(),
            num_parts: 1,
            attempts: 1,
            failed_amounts: Vec::default(),
        };
        assert!(Payment::split_payment(&payment, payment.amount_msat).is_none());
    }
}
