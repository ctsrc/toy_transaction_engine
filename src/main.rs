//! # Toy Transaction Engine
//!
//! This is a toy transaction engine that demonstrates basic reading of transaction data
//! from a CSV file and processing the transactions listed in the input according to
//! a specification document.
//!
//! Be sure to check out the [main readme](https://github.com/ctsrc/toy_transaction_engine#readme)
//! file in the [repository](https://github.com/ctsrc/toy_transaction_engine) as well.
//! Said readme has some high-level details about the implementation, and it also details
//! some assumptions.
//!
//! Input of CSV data is handled in the [transaction_engine_util::csv_input] module.
//!
//! The transaction processing itself happens in the [transaction_engine] package.
//!
//! CSV output happens mainly here in the command-line utility, using a
//! helper struct from [transaction_engine_util::csv_output] for formatting
//! the output according to spec.
//!
//! CSV input and output errors, including errors during parsing of CSV input,
//! are considered fatal and will result in termination of the program.
//!
//! When processing transactions, syntactically valid transactions
//! that specify invalid operations are reported as errors but are
//! not considered fatal and processing of the remaining transactions
//! will continue. These types of errors are reported to `stderr`
//! by the command-line utility.

use clap::Parser;

use transaction_engine_util::csv_input::{CSVInputParser, Transaction};
use transaction_engine::{TransactionProcessor, Accounts};
use transaction_engine_util::csv_output::AccountOutputCSVRecord;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
  csv_input_file: String,
}

fn main () -> anyhow::Result<()>
{
  let args = Args::parse();
  let csv_parser: CSVInputParser<_> = args.csv_input_file.try_into()?;
  let mut transaction_processor = TransactionProcessor::new();
  for tx_result in csv_parser {
    // XXX: We consider failures in CSV parsing to be fatal.
    let (client_id, transaction_id, tx) = tx_result?;
    // XXX: Transactions themselves are allowed to error as per spec.
    //      Errors in transactions themselves are logged to stderr
    //      and processing continues.
    match tx {
      Transaction::Deposit(amount) => {
        let tx_result = transaction_processor.deposit(client_id, transaction_id, amount);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of deposit tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Withdrawal(amount) => {
        let tx_result = transaction_processor.withdraw(client_id, transaction_id, amount);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of withdrawal tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Dispute => {
        let tx_result = transaction_processor.dispute(client_id, transaction_id);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of dispute for tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Resolve => {
        let tx_result = transaction_processor.resolve(client_id, transaction_id);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of resolve for tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Chargeback => {
        let tx_result = transaction_processor.chargeback(client_id, transaction_id);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of chargeback for tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
    }
  }
  let final_account_data: Accounts = transaction_processor.into();
  let mut wtr = csv::Writer::from_writer(std::io::stdout());
  for (client_id, account) in final_account_data {
    wtr.serialize(AccountOutputCSVRecord {
      client: client_id.into(),
      available: account.get_available().to_string(),
      held: account.get_held().to_string(),
      total: account.get_total().to_string(),
      locked: account.is_frozen(),
    })?;
  }
  wtr.flush()?;
  Ok(())
}
