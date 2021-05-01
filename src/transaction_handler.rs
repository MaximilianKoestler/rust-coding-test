use anyhow::{anyhow, Result};

use crate::types::{
    Account, DisputableTransaction, DisputedTransactionRecord, MonetaryTransactionRecord,
    Transaction,
};
use crate::{
    account_store::{AccountStore, HashMapAccountStore},
    transaction_store::{HashMapTransactionStore, TransactionStore},
};

/// Can process a series of transactions while keeping track of the system's state
pub struct TransactionHandler {
    account_store: HashMapAccountStore,
    transaction_store: HashMapTransactionStore,
}

impl<'a> IntoIterator for &'a mut TransactionHandler {
    type Item = Account;

    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.account_store.into_iter()
    }
}

impl TransactionHandler {
    pub fn new() -> Self {
        Self {
            account_store: HashMapAccountStore::new(),
            transaction_store: HashMapTransactionStore::new(),
        }
    }

    fn handle_deposit(&mut self, record: MonetaryTransactionRecord) -> Result<()> {
        let transaction_result = self
            .transaction_store
            .add_transaction(DisputableTransaction::Deposit(record.clone()));

        transaction_result.and_then(|_| {
            self.account_store
                .add_to_balance(record.client, record.amount)
        })
    }

    fn handle_withdrawal(&mut self, record: MonetaryTransactionRecord) -> Result<()> {
        self.account_store
            .add_to_balance(record.client, -record.amount)
    }

    fn handle_dispute(&mut self, record: DisputedTransactionRecord) -> Result<()> {
        todo!()
    }

    fn handle_resolve(&mut self, record: DisputedTransactionRecord) -> Result<()> {
        todo!()
    }

    fn handle_chargeback(&mut self, record: DisputedTransactionRecord) -> Result<()> {
        todo!()
    }

    /// Handle all given transactions
    /// This method is infallible, all bogus transactions are ignored, errors will be logged.
    pub fn handle_transaction(&mut self, transactions: impl Iterator<Item = Transaction>) {
        for transaction in transactions {
            let result = match transaction {
                Transaction::Deposit(record) => self.handle_deposit(record),
                Transaction::Withdrawal(record) => self.handle_withdrawal(record),
                Transaction::Dispute(record) => self.handle_dispute(record),
                Transaction::Resolve(record) => self.handle_resolve(record),
                Transaction::Chargeback(record) => self.handle_chargeback(record),
            };
            let _ = result; // TODO: add logging
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use rust_decimal_macros::dec;

    #[test]
    fn single_deposit() {
        let mut handler = TransactionHandler::new();

        let transactions = vec![Transaction::Deposit(MonetaryTransactionRecord {
            client: 0,
            transaction: 0,
            amount: dec!(2),
        })];

        handler.handle_transaction(transactions.into_iter());

        let accounts: Vec<_> = handler.into_iter().collect();
        assert_eq!(
            accounts,
            vec![Account {
                client: 0,
                available: dec!(2.0),
                held: Amount::ZERO,
                locked: false,
            }]
        );
    }

    #[test]
    fn deposit_then_withdraw() {
        let mut handler = TransactionHandler::new();

        let transactions = vec![
            Transaction::Deposit(MonetaryTransactionRecord {
                client: 0,
                transaction: 0,
                amount: dec!(2.0),
            }),
            Transaction::Withdrawal(MonetaryTransactionRecord {
                client: 0,
                transaction: 0,
                amount: dec!(1.0),
            }),
        ];

        handler.handle_transaction(transactions.into_iter());

        let accounts: Vec<_> = handler.into_iter().collect();
        assert_eq!(
            accounts,
            vec![Account {
                client: 0,
                available: dec!(1.0),
                held: Amount::ZERO,
                locked: false,
            }]
        );
    }
}