use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::types::{
    Amount, ClientId, DisputedTransactionRecord, MonetaryTransactionRecord, Transaction,
    TransactionId,
};

/// The different transaction type identifiers as in the input CSV
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawTransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// A single row of the input CSV, the amount is missing for certain transaction types
#[derive(Debug, Deserialize)]
pub struct RawTransaction {
    #[serde(rename = "type")]
    transaction_type: RawTransactionType,

    #[serde(rename = "client")]
    client: ClientId,

    #[serde(rename = "tx")]
    transaction: TransactionId,

    #[serde(rename = "amount")]
    amount: Option<Amount>,
}

/// Turn a `RawTransaction` into a `Transaction` that can be handled in a nicer way (no optional!)
///
/// Should `Dispute`, `Resolve`, or `Chargeback` records include an `amount`, the `amount` will be
/// silently discarded but the record will be kept.
fn raw_to_transaction(raw: RawTransaction) -> Result<Transaction> {
    let RawTransaction {
        transaction_type,
        client,
        transaction,
        amount,
    } = raw;

    match transaction_type {
        RawTransactionType::Deposit => {
            if let Some(amount) = amount {
                Ok(Transaction::Deposit(MonetaryTransactionRecord {
                    client,
                    transaction,
                    amount,
                }))
            } else {
                Err(anyhow!("No 'amount' for deposit (tx = {})", transaction))
            }
        }
        RawTransactionType::Withdrawal => {
            if let Some(amount) = amount {
                Ok(Transaction::Withdrawal(MonetaryTransactionRecord {
                    client,
                    transaction,
                    amount,
                }))
            } else {
                Err(anyhow!("No 'amount' for withdrawal (tx = {})", transaction))
            }
        }
        RawTransactionType::Dispute => Ok(Transaction::Dispute(DisputedTransactionRecord {
            client,
            transaction,
        })),
        RawTransactionType::Resolve => Ok(Transaction::Resolve(DisputedTransactionRecord {
            client,
            transaction,
        })),
        RawTransactionType::Chargeback => Ok(Transaction::Chargeback(DisputedTransactionRecord {
            client,
            transaction,
        })),
    }
}

/// For each line of the input (skipping the header), read a line by line `Transaction` record.
pub fn iter_transactions(reader: impl std::io::Read) -> impl Iterator<Item = Result<Transaction>> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader)
        .into_deserialize()
        .map(|raw| raw.map_err(Into::into).and_then(raw_to_transaction))
}

#[cfg(test)]
mod tests {
    use super::*;

    use rust_decimal_macros::dec;

    #[test]
    fn empty_file() {
        let buffer = br#""#;
        let count = iter_transactions(&buffer[..]).count();
        assert_eq!(count, 0);
    }

    #[test]
    fn single_line() {
        let buffer = br#"
type, client, tx, amount
deposit, 0, 1, 2
"#;
        let entries: Vec<_> = iter_transactions(&buffer[..]).map(|r| r.unwrap()).collect();
        assert_eq!(
            entries,
            vec![Transaction::Deposit(MonetaryTransactionRecord {
                client: 0,
                transaction: 1,
                amount: dec!(2)
            })]
        );
    }

    #[test]
    fn all_different_transactions() {
        let buffer = br#"
type, client, tx, amount
deposit, 0, 1, 2.5
withdrawal, 3, 4, -5.1
dispute, 6, 7,
resolve, 8, 9,
chargeback, 10, 11,
"#;
        let entries: Vec<_> = iter_transactions(&buffer[..]).map(|r| r.unwrap()).collect();
        assert_eq!(
            entries,
            vec![
                Transaction::Deposit(MonetaryTransactionRecord {
                    client: 0,
                    transaction: 1,
                    amount: dec!(2.5)
                }),
                Transaction::Withdrawal(MonetaryTransactionRecord {
                    client: 3,
                    transaction: 4,
                    amount: dec!(-5.1)
                }),
                Transaction::Dispute(DisputedTransactionRecord {
                    client: 6,
                    transaction: 7,
                }),
                Transaction::Resolve(DisputedTransactionRecord {
                    client: 8,
                    transaction: 9,
                }),
                Transaction::Chargeback(DisputedTransactionRecord {
                    client: 10,
                    transaction: 11,
                })
            ]
        );
    }

    #[test]
    fn all_mixed_in_errors() {
        let buffer = br#"
type, client, tx, amount
deposit, 0, 1, 2
withdrawal, 3, 4
dispute, 6, 7, 4
dance, 8, 9,
chargeback, 10, 11,
"#;
        let entries: Vec<_> = iter_transactions(&buffer[..]).collect();
        assert!(entries[0].is_ok()); // all good
        assert!(entries[1].is_err()); // no amount
        assert!(entries[2].is_ok()); // no amount needed but ok
        assert!(entries[3].is_err()); // unsupported type
        assert!(entries[4].is_ok()); // all good
    }
}
