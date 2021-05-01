use anyhow::Result;
use csv;
use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::types::Account;

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Account", 5)?;
        state.serialize_field("client", &self.client)?;
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;
        state.serialize_field("total", &self.total())?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}

/// Write all accounts to the provided destination (in CSV format)
pub fn write_accounts(
    destination: &mut dyn std::io::Write,
    accounts: impl Iterator<Item = Account>,
) -> Result<()> {
    let mut writer = csv::WriterBuilder::new().from_writer(destination);

    for account in accounts {
        if let Err(err) = writer.serialize(account) {
            return Err(err.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use rust_decimal_macros::dec;

    #[test]
    fn empty_list() {
        let mut buffer = vec![];
        let accounts = vec![];

        write_accounts(&mut buffer, accounts.into_iter()).unwrap();
        let data = String::from_utf8(buffer).unwrap();
        assert_eq!(&data, "");
    }

    #[test]
    fn single_account() {
        let mut buffer = vec![];
        let accounts = vec![Account {
            client: 0,
            available: dec!(1.0),
            held: dec!(2.0),
            locked: true,
        }];

        write_accounts(&mut buffer, accounts.into_iter()).unwrap();
        let data = String::from_utf8(buffer).unwrap();
        assert_eq!(
            &data,
            r#"client,available,held,total,locked
0,1.0,2.0,3.0,true
"#
        );
    }
}
