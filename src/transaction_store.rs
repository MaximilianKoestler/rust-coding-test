use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::types::{
    Amount, ClientId, DisputableTransaction, DisputedTransactionRecord, MonetaryTransactionRecord,
    TransactionId,
};

/// Select how a disputed transaction should be handled
pub enum UndisputeOutcome {
    /// After resolving the transaction will be exactly as before the dispute
    Resolve,

    /// The transaction will be completely undone and locked. It cannot be disputed again.
    Chargeback,
}

/// Store transactions for later possibility to dispute
pub trait TransactionStore {
    /// Add a transaction to the store
    /// No transaction with the same ID may have been added before.
    fn add_transaction(&mut self, transaction: DisputableTransaction) -> Result<()>;

    /// Dispute a transaction
    /// The transaction must have been added and it may not have gone through chargeback.
    fn dispute_transaction(
        &mut self,
        transaction: &DisputedTransactionRecord,
    ) -> Result<DisputableTransaction>;

    /// Undispute a transaction (resolve or chargeback)
    /// The transaction must have been successfully disputed before.
    fn undispute_transaction(
        &mut self,
        transaction: &DisputedTransactionRecord,
        outcome: UndisputeOutcome,
    ) -> Result<DisputableTransaction>;
}

#[derive(Debug, PartialEq)]
enum DisputeState {
    NotDisputed,
    Disputed,
    ChargebackOcurred,
}

#[derive(Debug, PartialEq)]
struct DisputableTransactionData {
    client: ClientId,
    amount: Amount,
    state: DisputeState,
}

/// A simple RAM-backed transaction store using a standard Rust `HashMap`
pub struct HashMapTransactionStore {
    data_store: HashMap<TransactionId, DisputableTransactionData>,
}

impl HashMapTransactionStore {
    pub fn new() -> Self {
        Self {
            data_store: HashMap::new(),
        }
    }
}

impl Default for HashMapTransactionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionStore for HashMapTransactionStore {
    fn add_transaction(&mut self, transaction: DisputableTransaction) -> Result<()> {
        match transaction {
            DisputableTransaction::Deposit(record) => {
                if self.data_store.contains_key(&record.transaction) {
                    return Err(anyhow!(
                        "Transaction already present (tx = {})",
                        record.transaction
                    ));
                }

                self.data_store.insert(
                    record.transaction,
                    DisputableTransactionData {
                        client: record.client,
                        amount: record.amount,
                        state: DisputeState::NotDisputed,
                    },
                );
            }
        }
        Ok(())
    }

    fn dispute_transaction(
        &mut self,
        transaction: &DisputedTransactionRecord,
    ) -> Result<DisputableTransaction> {
        if let Some(data) = self.data_store.get_mut(&transaction.transaction) {
            if data.client != transaction.client {
                return Err(anyhow!(
                    "Mismatching client for dispute (tx = {})",
                    transaction.transaction
                ));
            }

            if data.state != DisputeState::NotDisputed {
                return Err(anyhow!(
                    "Transaction already disputed (tx = {})",
                    transaction.transaction
                ));
            }

            data.state = DisputeState::Disputed;

            Ok(DisputableTransaction::Deposit(MonetaryTransactionRecord {
                client: data.client,
                transaction: transaction.transaction,
                amount: data.amount,
            }))
        } else {
            Err(anyhow!(
                "Transaction not found for dispute (tx = {})",
                transaction.transaction
            ))
        }
    }

    fn undispute_transaction(
        &mut self,
        transaction: &DisputedTransactionRecord,
        outcome: UndisputeOutcome,
    ) -> Result<DisputableTransaction> {
        if let Some(data) = self.data_store.get_mut(&transaction.transaction) {
            if data.client != transaction.client {
                return Err(anyhow!(
                    "Mismatching client for undispute (tx = {})",
                    transaction.transaction
                ));
            }

            if data.state != DisputeState::Disputed {
                return Err(anyhow!(
                    "Transaction not yet disputed (tx = {})",
                    transaction.transaction
                ));
            }

            match outcome {
                UndisputeOutcome::Resolve => data.state = DisputeState::NotDisputed,
                UndisputeOutcome::Chargeback => data.state = DisputeState::ChargebackOcurred,
            }

            Ok(DisputableTransaction::Deposit(MonetaryTransactionRecord {
                client: data.client,
                transaction: transaction.transaction,
                amount: data.amount,
            }))
        } else {
            Err(anyhow!(
                "Transaction not found for undispute (tx = {})",
                transaction.transaction
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::*;

    use rust_decimal_macros::dec;

    #[test]
    fn add_dispute_resolve() {
        let mut store = HashMapTransactionStore::new();

        let deposit = DisputableTransaction::Deposit(MonetaryTransactionRecord {
            client: 0,
            transaction: 0,
            amount: dec!(1.0),
        });
        store.add_transaction(deposit).unwrap();

        let dispute = DisputedTransactionRecord {
            client: 0,
            transaction: 0,
        };
        let DisputableTransaction::Deposit(record) = store.dispute_transaction(&dispute).unwrap();
        assert_eq!(record.amount, dec!(1.0));

        let DisputableTransaction::Deposit(record) = store
            .undispute_transaction(&dispute, UndisputeOutcome::Resolve)
            .unwrap();
        assert_eq!(record.amount, dec!(1.0));

        // after resolve, the transaction can be disputed again
        store.dispute_transaction(&dispute).unwrap();
    }

    #[test]
    fn add_dispute_chargeback() {
        let mut store = HashMapTransactionStore::new();

        let deposit = DisputableTransaction::Deposit(MonetaryTransactionRecord {
            client: 0,
            transaction: 0,
            amount: dec!(1.0),
        });
        store.add_transaction(deposit).unwrap();

        let dispute = DisputedTransactionRecord {
            client: 0,
            transaction: 0,
        };
        let DisputableTransaction::Deposit(record) = store.dispute_transaction(&dispute).unwrap();
        assert_eq!(record.amount, dec!(1.0));

        let DisputableTransaction::Deposit(record) = store
            .undispute_transaction(&dispute, UndisputeOutcome::Chargeback)
            .unwrap();
        assert_eq!(record.amount, dec!(1.0));

        // after chargeback, the transaction cannot be disputed again
        store.dispute_transaction(&dispute).unwrap_err();
    }

    #[test]
    fn add_twice() {
        let mut store = HashMapTransactionStore::new();

        let deposit = DisputableTransaction::Deposit(MonetaryTransactionRecord {
            client: 0,
            transaction: 0,
            amount: dec!(1.0),
        });
        store.add_transaction(deposit.clone()).unwrap();
        store.add_transaction(deposit).unwrap_err();
    }

    #[test]
    fn dispute_without_add() {
        let mut store = HashMapTransactionStore::new();

        let dispute = DisputedTransactionRecord {
            client: 0,
            transaction: 0,
        };
        store.dispute_transaction(&dispute).unwrap_err();
    }

    #[test]
    fn undispute_without_add() {
        let mut store = HashMapTransactionStore::new();

        let dispute = DisputedTransactionRecord {
            client: 0,
            transaction: 0,
        };
        store
            .undispute_transaction(&dispute, UndisputeOutcome::Resolve)
            .unwrap_err();
    }

    #[test]
    fn undispute_without_dispute() {
        let mut store = HashMapTransactionStore::new();

        let deposit = DisputableTransaction::Deposit(MonetaryTransactionRecord {
            client: 0,
            transaction: 0,
            amount: dec!(1.0),
        });
        store.add_transaction(deposit.clone()).unwrap();

        let dispute = DisputedTransactionRecord {
            client: 0,
            transaction: 0,
        };
        store
            .undispute_transaction(&dispute, UndisputeOutcome::Resolve)
            .unwrap_err();
    }
}
