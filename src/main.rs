#![forbid(unsafe_code)]

use anyhow::{anyhow, Result};
use pretty_env_logger;

use rust_coding_test::{
    csv_parser::iter_transactions, csv_writer::write_accounts,
    transaction_handler::TransactionHandler,
};

/// Read records in CSV format from the `source`, process all transactions and write the account
/// data to `destination` (also in CSV format)
fn process_transactions(
    source: impl std::io::Read,
    destination: &mut dyn std::io::Write,
) -> Result<()> {
    let transactions = iter_transactions(source);

    let mut handler = TransactionHandler::new();
    handler.handle_transactions(transactions);

    write_accounts(destination, handler.into_iter())
}

fn main() -> Result<()> {
    pretty_env_logger::init();

    let path = std::env::args()
        .nth(1) // skip executable name
        .ok_or_else(|| anyhow!("Missing input file"))?;

    let file = std::fs::File::open(path)?;
    let mut stdout = Box::new(std::io::stdout());
    process_transactions(file, &mut stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modified_example_from_requirements() {
        // Note: This test makes assumption about exact output format and is therefore very brittle.
        //       Its purpose is primarily to detect changes in the overall output format.

        let source = br#"
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
"#;
        let mut destination = vec![];

        process_transactions(&source[..], &mut destination).unwrap();

        let result = String::from_utf8(destination).unwrap();
        assert_eq!(
            &result,
            r#"client,available,held,total,locked
1,1.5,0,1.5,false
"#
        );
    }
}
