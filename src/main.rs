//! # Toy Transaction Engine
//!
//! This is a toy transaction engine that demonstrates basic reading of transaction data
//! from a CSV file and processing the transactions listed in the input according to
//! a specification document.
//!
//! The comments in the source code for this toy transaction engine are meant to detail
//! my thinking and reasoning around the requirements, as well as to explain why I'm making
//! some of the choices I am making in implementing it. As such there is quite a bit
//! of details in the comments that extend beyond what I would usually include if this
//! had been code for a production code base.
//!
//! Be sure to check out the main readme file in the repository as well. Said readme has
//! some high-level details about the implementation.
//!
//! Input of CSV data is handled in the [csv_input] module.
//!
//! The transaction processing itself happens in the [tx_processing] module.
//!
//! The code in the main file connects these modules together.
//!
//! - Erik N., Wednesday April 13th 2022

use clap::Parser;
use crate::csv_input::Transaction;

pub mod tx_processing;
pub mod csv_input;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
  csv_input_file: String,
}

fn main () -> anyhow::Result<()>
{
  let args = Args::parse();
  let csv_parser: csv_input::CSVInputParser = args.csv_input_file.try_into()?;
  let mut transaction_processor = tx_processing::TransactionProcessor::new();
  for tx_result in csv_parser {
    eprintln!("{:?}", tx_result);
    // XXX: We consider failures in CSV parsing to be fatal.
    let (client_id, transaction_id, tx) = tx_result?;
    // XXX: Transactions themselves are allowed to error as per spec.
    //      Errors in transactions themselves are logged to stderr
    //      and processing continues.
    match tx {
      Transaction::Deposit(amount) => transaction_processor.deposit(&client_id, &transaction_id, amount),
      Transaction::Withdrawal(amount) => {
        let tx_result = transaction_processor.withdraw(&client_id, &transaction_id, amount);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Dispute => {
        let tx_result = transaction_processor.dispute(&client_id, &transaction_id);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of dispute for tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Resolve => {
        let tx_result = transaction_processor.resolve(&client_id, &transaction_id);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of resolve for tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
      Transaction::Chargeback => {
        let tx_result = transaction_processor.chargeback(&client_id, &transaction_id);
        if let Err(e) = tx_result {
          eprintln!("Error during processing of chargeback for tx {} for client {}: {:?}", transaction_id, client_id, e);
        }
      },
    }
  }
  let final_account_data: tx_processing::Accounts = transaction_processor.into();
  for (client_id, account) in final_account_data {
    println!("{} {:?}", client_id, account);
  }
  Ok(())
}
