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
