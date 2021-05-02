use anyhow::Result;

use crate::types::{
    Account, DisputableTransaction, DisputedTransactionRecord, MonetaryTransactionRecord,
    Transaction,
};
use crate::{
    account_store::{AccountStore, HashMapAccountStore},
    transaction_store::{HashMapTransactionStore, TransactionStore, UndisputeOutcome},
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

    /// Handle a single "deposit" transaction
    /// The client's available funds will go up and the transaction will be stored for later use
    fn handle_deposit(&mut self, record: MonetaryTransactionRecord) -> Result<()> {
        let transaction_result = self
            .transaction_store
            .add_transaction(DisputableTransaction::Deposit(record.clone()));

        // Note: We do not need to propagate information to the `transaction_store` back about
        //       whether the `account_store` accepts the balance change.
        //       If flagging the transaction in this case was required, the code would go here.
        transaction_result.and_then(|_| {
            self.account_store
                .add_to_balance(record.client, record.amount)
        })
    }

    /// Handle a single "withdrawal" transaction
    /// The client's available funds will go down if sufficient for the transaction
    fn handle_withdrawal(&mut self, record: MonetaryTransactionRecord) -> Result<()> {
        self.account_store
            .add_to_balance(record.client, -record.amount)
    }

    /// Handle a single "dispute" transaction
    /// If the disputed transaction exists, and belongs to the client, the amount from the
    /// transaction is held back for further handling.
    /// As only "deposit" transactions are stored, only those can be disputed successfully.
    fn handle_dispute(&mut self, record: DisputedTransactionRecord) -> Result<()> {
        let transaction_result = self.transaction_store.dispute_transaction(&record);

        transaction_result.and_then(|transaction| {
            let DisputableTransaction::Deposit(data) = transaction;
            self.account_store.hold_amount(data.client, data.amount)
        })
    }

    /// Handle a single "resolve" transaction
    /// If the referenced transaction exists, belongs to the client, and was disputed, the held back
    /// amount from the transaction is released into the client's available funds.
    fn handle_resolve(&mut self, record: DisputedTransactionRecord) -> Result<()> {
        let transaction_result = self
            .transaction_store
            .undispute_transaction(&record, UndisputeOutcome::Resolve);

        transaction_result.and_then(|transaction| {
            let DisputableTransaction::Deposit(data) = transaction;
            self.account_store
                .release_held_amount(data.client, data.amount)
        })
    }

    /// Handle a single "chargeback" transaction
    /// If the referenced transaction exists, belongs to the client, and was disputed, the held back
    /// amount from the transaction removed from the client's account and the account is frozen.
    fn handle_chargeback(&mut self, record: DisputedTransactionRecord) -> Result<()> {
        let transaction_result = self
            .transaction_store
            .undispute_transaction(&record, UndisputeOutcome::Chargeback);

        // The following call includes the "freeze"
        transaction_result.and_then(|transaction| {
            let DisputableTransaction::Deposit(data) = transaction;
            self.account_store
                .charge_back_amount(data.client, data.amount)
        })
    }

    /// Handle all given transactions
    /// This method is infallible, all bogus transactions are ignored, errors will be logged.
    pub fn handle_transactions(&mut self, transactions: impl Iterator<Item = Result<Transaction>>) {
        for transaction in transactions {
            let result = transaction.and_then(|transaction| match transaction {
                Transaction::Deposit(record) => self.handle_deposit(record),
                Transaction::Withdrawal(record) => self.handle_withdrawal(record),
                Transaction::Dispute(record) => self.handle_dispute(record),
                Transaction::Resolve(record) => self.handle_resolve(record),
                Transaction::Chargeback(record) => self.handle_chargeback(record),
            });
            if let Err(error) = result {
                warn!("{}", error);
            }
        }
    }
}

impl Default for TransactionHandler {
    fn default() -> Self {
        Self::new()
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

        handler.handle_transactions(transactions.into_iter().map(|t| Ok(t)));

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

        handler.handle_transactions(transactions.into_iter().map(|t| Ok(t)));

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

    #[test]
    fn deposit_then_withdraw_more_than_available() {
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
                amount: dec!(3.0),
            }),
        ];

        handler.handle_transactions(transactions.into_iter().map(|t| Ok(t)));

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
    fn deposit_dispute() {
        let mut handler = TransactionHandler::new();

        let transactions = vec![
            Transaction::Deposit(MonetaryTransactionRecord {
                client: 0,
                transaction: 0,
                amount: dec!(2.0),
            }),
            Transaction::Deposit(MonetaryTransactionRecord {
                client: 0,
                transaction: 1,
                amount: dec!(3.0),
            }),
            Transaction::Dispute(DisputedTransactionRecord {
                client: 0,
                transaction: 1,
            }),
        ];

        handler.handle_transactions(transactions.into_iter().map(|t| Ok(t)));

        let accounts: Vec<_> = handler.into_iter().collect();
        assert_eq!(
            accounts,
            vec![Account {
                client: 0,
                available: dec!(2.0),
                held: dec!(3.0),
                locked: false,
            }]
        );
    }

    #[test]
    fn deposit_dispute_resolve() {
        let mut handler = TransactionHandler::new();

        let transactions = vec![
            Transaction::Deposit(MonetaryTransactionRecord {
                client: 0,
                transaction: 0,
                amount: dec!(2.0),
            }),
            Transaction::Dispute(DisputedTransactionRecord {
                client: 0,
                transaction: 0,
            }),
            Transaction::Resolve(DisputedTransactionRecord {
                client: 0,
                transaction: 0,
            }),
        ];

        handler.handle_transactions(transactions.into_iter().map(|t| Ok(t)));

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
    fn deposit_dispute_charge_back() {
        let mut handler = TransactionHandler::new();

        let transactions = vec![
            Transaction::Deposit(MonetaryTransactionRecord {
                client: 0,
                transaction: 0,
                amount: dec!(2.0),
            }),
            Transaction::Dispute(DisputedTransactionRecord {
                client: 0,
                transaction: 0,
            }),
            Transaction::Chargeback(DisputedTransactionRecord {
                client: 0,
                transaction: 0,
            }),
        ];

        handler.handle_transactions(transactions.into_iter().map(|t| Ok(t)));

        let accounts: Vec<_> = handler.into_iter().collect();
        assert_eq!(
            accounts,
            vec![Account {
                client: 0,
                available: Amount::ZERO,
                held: Amount::ZERO,
                locked: true,
            }]
        );
    }
}
