use rust_decimal::Decimal;

pub type ClientId = u16;

pub type TransactionId = u32;

pub type Amount = Decimal;

/// Represents money flowing towards or from a client account
#[derive(Debug, Clone, PartialEq)]
pub struct MonetaryTransactionRecord {
    pub client: ClientId,
    pub transaction: TransactionId,
    pub amount: Amount,
}

/// References a `MonetaryTransactionRecord` for use in dispute claim handling
#[derive(Debug, Clone, PartialEq)]
pub struct DisputedTransactionRecord {
    pub client: ClientId,
    pub transaction: TransactionId,
}

/// A transaction that can occur in the processor's input
#[derive(Debug, Clone, PartialEq)]
pub enum Transaction {
    Deposit(MonetaryTransactionRecord),
    Withdrawal(MonetaryTransactionRecord),
    Dispute(DisputedTransactionRecord),
    Resolve(DisputedTransactionRecord),
    Chargeback(DisputedTransactionRecord),
}

/// Only a limited set of transactions is disputable
//
/// In the requirements, the business logic for disputes is only defined for deposits.
#[derive(Debug, Clone, PartialEq)]
pub enum DisputableTransaction {
    Deposit(MonetaryTransactionRecord),
}

/// Represents the current funds (available and held) of a client
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub client: ClientId,
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

impl Account {
    /// Compute the total funds of the client (available and held)
    pub fn total(&self) -> Amount {
        self.available + self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rust_decimal_macros::dec;

    #[test]
    fn test_total() {
        let account = Account {
            client: 0,
            available: dec!(1.0),
            held: dec!(2.0),
            locked: false,
        };
        assert_eq!(account.total(), dec!(3.0));
    }
}
