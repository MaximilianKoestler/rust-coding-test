use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::types::{Account, Amount, ClientId};

/// Store account information to settle transactions
pub trait AccountStore {
    /// Process a balance change, positive amounts mean deposits, negative mean withdrawals
    fn add_to_balance(&mut self, client: ClientId, amount: Amount) -> Result<()>;

    /// Hold the given (positive) amount due to a dispute (resolved by later transactions)
    fn hold_amount(&mut self, client: ClientId, amount: Amount) -> Result<()>;

    /// Release the given (positive) amount into the available funds
    fn release_held_amount(&mut self, client: ClientId, amount: Amount) -> Result<()>;

    /// Withdraw the given (positive) amount from the held funds and lock the account
    fn charge_back_amount(&mut self, client: ClientId, amount: Amount) -> Result<()>;
}

#[derive(Debug, PartialEq)]
struct AccountData {
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

/// A simple RAM-backed account store using a standard Rust `HashMap`
pub struct HashMapAccountStore {
    data_store: HashMap<ClientId, AccountData>,
}

impl HashMapAccountStore {
    fn new() -> Self {
        Self {
            data_store: HashMap::new(),
        }
    }
}

impl<'a> IntoIterator for &'a mut HashMapAccountStore {
    type Item = Account;

    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.data_store.iter_mut().map(|(client, data)| Account {
            client: *client,
            available: data.available,
            held: data.held,
            locked: data.locked,
        }))
    }
}

impl AccountStore for HashMapAccountStore {
    fn add_to_balance(&mut self, client: ClientId, amount: Amount) -> Result<()> {
        if let Some(data) = self.data_store.get_mut(&client) {
            if data.locked {
                return Err(anyhow!(
                    "Cannot change balance of locked account (client = {})",
                    client
                ));
            }

            let new_amount = data.available + amount;
            if new_amount.is_sign_negative() {
                return Err(anyhow!(
                    "Transaction would cause negative balance (client = {})",
                    client
                ));
            }
            data.available = new_amount;
        } else {
            if amount.is_sign_negative() {
                return Err(anyhow!(
                    "Account creation would start with negative balance (client = {})",
                    client
                ));
            }

            self.data_store.insert(
                client,
                AccountData {
                    available: amount,
                    held: Amount::ZERO,
                    locked: false,
                },
            );
        }
        Ok(())
    }

    fn hold_amount(&mut self, client: ClientId, amount: Amount) -> Result<()> {
        if amount.is_sign_negative() {
            return Err(anyhow!("Cannot hold negative amount (client = {})", client));
        }

        if let Some(data) = self.data_store.get_mut(&client) {
            let amount_to_be_held = data.available.min(amount);
            data.available -= amount_to_be_held;
            data.held += amount_to_be_held;
        } else {
            return Err(anyhow!("Client does not exist (client = {})", client));
        }
        Ok(())
    }

    fn release_held_amount(&mut self, client: ClientId, amount: Amount) -> Result<()> {
        if amount.is_sign_negative() {
            return Err(anyhow!(
                "Cannot release negative amount (client = {})",
                client
            ));
        }

        if let Some(data) = self.data_store.get_mut(&client) {
            let amount_to_be_released = data.held.min(amount);
            data.available += amount_to_be_released;
            data.held -= amount_to_be_released;
        } else {
            return Err(anyhow!("Client does not exist (client = {})", client));
        }
        Ok(())
    }

    fn charge_back_amount(&mut self, client: ClientId, amount: Amount) -> Result<()> {
        if amount.is_sign_negative() {
            return Err(anyhow!(
                "Cannot charge back negative amount (client = {})",
                client
            ));
        }

        if let Some(data) = self.data_store.get_mut(&client) {
            let amount_to_be_charged = data.held.min(amount);
            data.held -= amount_to_be_charged;
            data.locked = true;
        } else {
            return Err(anyhow!("Client does not exist (client = {})", client));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn empty_store() {
        let mut store = HashMapAccountStore::new();
        assert_eq!(store.into_iter().count(), 0);
    }

    #[test]
    fn single_positive_balance_change() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(1.0)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: dec!(1.0),
                held: Amount::ZERO,
                locked: false,
            }]
        );
    }

    #[test]
    fn single_negative_balance_change() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(-1.0)).unwrap_err();
        assert_eq!(store.into_iter().count(), 0);
    }

    #[test]
    fn hold_release_charge_from_non_existing_account() {
        let mut store = HashMapAccountStore::new();

        store.hold_amount(0, dec!(1.0)).unwrap_err();
        store.release_held_amount(0, dec!(1.0)).unwrap_err();
        store.charge_back_amount(0, dec!(1.0)).unwrap_err();
        assert_eq!(store.into_iter().count(), 0);
    }

    #[test]
    fn hold_release_negative_amount() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(-1.0)).unwrap_err();
        store.release_held_amount(0, dec!(-1.0)).unwrap_err();
        store.charge_back_amount(0, dec!(-1.0)).unwrap_err();
    }

    #[test]
    fn hold_partial_amount() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(1.0)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: dec!(1.0),
                held: dec!(1.0),
                locked: false,
            }]
        );
    }

    #[test]
    fn hold_more_than_available() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(5.0)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: Amount::ZERO,
                held: dec!(2.0),
                locked: false,
            }]
        );
    }

    #[test]
    fn release_partial_amount() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(1.0)).unwrap();
        store.release_held_amount(0, dec!(0.5)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: dec!(1.5),
                held: dec!(0.5),
                locked: false,
            }]
        );
    }

    #[test]
    fn release_more_then_held() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(1.0)).unwrap();
        store.release_held_amount(0, dec!(5.0)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: dec!(2.0),
                held: Amount::ZERO,
                locked: false,
            }]
        );
    }

    #[test]
    fn charge_back_partial_amount() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(1.0)).unwrap();
        store.charge_back_amount(0, dec!(0.5)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: dec!(1.0),
                held: dec!(0.5),
                locked: true,
            }]
        );
    }

    #[test]
    fn charge_back_more_then_held() {
        let mut store = HashMapAccountStore::new();

        store.add_to_balance(0, dec!(2.0)).unwrap();
        store.hold_amount(0, dec!(1.0)).unwrap();
        store.charge_back_amount(0, dec!(5.0)).unwrap();

        let entries: Vec<_> = store.into_iter().collect();
        assert_eq!(
            entries,
            vec![Account {
                client: 0,
                available: dec!(1.0),
                held: Amount::ZERO,
                locked: true,
            }]
        );
    }
}
