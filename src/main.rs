#![forbid(unsafe_code)]

use pretty_env_logger;
use anyhow::{anyhow, Result};

use rust_coding_test::{
    csv_parser::iter_transactions, csv_writer::write_accounts,
    transaction_handler::TransactionHandler,
};

fn main() -> Result<()> {
    pretty_env_logger::init();

    let path = std::env::args()
        .nth(1) // skip executable name
        .ok_or_else(|| anyhow!("Missing input file"))?;

    let file = std::fs::File::open(path)?;
    let transactions = iter_transactions(file);

    let mut handler = TransactionHandler::new();
    handler.handle_transactions(transactions);

    let mut stdout = Box::new(std::io::stdout());
    write_accounts(&mut stdout, handler.into_iter())?;

    Ok(())
}
