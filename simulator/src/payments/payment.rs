use crate::{PaymentId, ID};

#[derive(Debug, Clone, Default)]
pub struct Payment {
    pub(crate) payment_id: PaymentId,
    pub(crate) source: ID,
    pub(crate) dest: ID,
    pub(crate) amount_msat: usize,
    pub(crate) failed: bool,
    //current_hop: Hop,
    pub(crate) current_hop: ID,
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
            min_shard_amt: 10000000,
        }
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
            min_shard_amt: 10000000,
        };
        assert_eq!(actual, expected);
        assert_eq!(actual.failed, expected.failed);
        assert_eq!(actual.min_shard_amt, expected.min_shard_amt);
    }
}
