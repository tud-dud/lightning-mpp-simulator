use crate::ID;

pub mod attempt;
pub mod payment;

/// the recipient generates an invoice on their node, which will contain basic information,
/// such as amount, destination and validity
#[derive(Debug, Clone)]
pub(crate) struct Invoice {
    /// Unique invoice id (represents the hash)
    pub(crate) id: usize,
    /// Amount that is due
    #[allow(unused)]
    pub(crate) amount: usize,
    /// payment source
    pub(crate) source: ID,
    /// payment recipient and issuer of invoice
    pub(crate) destination: ID,
}

impl Invoice {
    pub(crate) fn new(id: usize, amount: usize, source: &ID, destination: &ID) -> Self {
        Self {
            id,
            amount,
            source: source.clone(),
            destination: destination.clone(),
        }
    }
}

impl Eq for Invoice {}
impl PartialEq for Invoice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_new_invoice() {
        let id = 0;
        let source = "source".to_string();
        let destination = "dest".to_string();
        let amount = 10000;
        let actual = Invoice::new(id, amount, &source, &destination);
        let expected = Invoice {
            id,
            source,
            destination,
            amount,
        };
        assert_eq!(actual, expected);
    }
}
