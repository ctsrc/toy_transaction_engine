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
//! The code in the main file for the command line application deals mainly with
//! input and output of CSV data, as well as setting things up for transaction processing
//! and handing data over for transaction processing.
//!
//! The transaction processing itself happens in the [crate::tx_processing] module.
//!
//! - Erik N., Wednesday April 13th 2022

/// Data structures and logic used for input of CSV data.
///
/// While it's not strictly necessary to organize this part of the code into
/// a separate module from the main function of the command line program,
/// I find it useful to do so in this case because the generated HTML documentation
/// output by `cargo doc` is then also more structured for anyone reading it.
///
/// In particular, input (and output) of CSV data, is important to the functioning
/// of the command line program itself, but it is very much secondary to the
/// actual transaction processing.
mod csv_input {
    use crate::tx_processing::{ClientId, TransactionId};

    /// Transaction record as it appears in CSV inputs.
    ///
    /// We use this struct in the initial stage of transaction processing
    /// where we are reading a transaction record from CSV data.
    ///
    /// The data in this struct is further transformed into structs for the specific
    /// type of transaction that it describes before processing of the transaction itself
    /// takes place.
    struct TransactionCSVRecord<'a> {
        transaction_type: TransactionType,
        client_id: ClientId,
        tx_id: TransactionId,
        /// The amount, where applicable, for the transaction.
        ///
        /// At the stage of reading the CSV record data we are not yet parsing
        /// the amounts into our [crate::tx_processing::TransactionAmount] type.
        ///
        /// We borrow the string for this field from the CSV reader, as opposed
        /// to using owned String, as the latter would cause additional allocation
        /// for data that we only need for a short amount of time anyways.
        amount: Option<&'a str>,
    }

    /// The different transaction types that a [TransactionCSVRecord] entry can have.
    enum TransactionType {
        Deposit,
        Withdrawal,
        Dispute,
        Resolve,
        Chargeback,
    }
}

pub mod tx_processing;

fn main() {
    println!("Hello, world!");
}
