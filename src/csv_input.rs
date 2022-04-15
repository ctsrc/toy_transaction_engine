//! Data structures and logic used for input of CSV data.

use serde::Deserialize;
use thiserror::Error;

use crate::transaction_engine::{ClientId, TransactionId, FractionalAmount};

/// Transaction record as it appears in CSV inputs.
///
/// We use this struct in the initial stage of transaction processing
/// where we are reading a transaction record from CSV data.
///
/// The data in this struct is further transformed into structs for the specific
/// type of transaction that it describes before processing of the transaction itself
/// takes place.
#[derive(Deserialize)]
pub(crate) struct TransactionCSVRecord<'a> {
  #[serde(rename = "type")]
  transaction_type: TransactionType,
  #[serde(rename = "client")]
  client_id: ClientId,
  #[serde(rename = "tx")]
  transaction_id: TransactionId,
  /// The amount, where applicable, for the transaction.
  ///
  /// At the stage of reading the CSV record data we are not yet parsing
  /// the amounts into our [FractionalAmount] type.
  ///
  /// We borrow the string for this field from the CSV reader, as opposed
  /// to using owned String, as the latter would cause additional allocation
  /// for data that we only need for a short amount of time anyways.
  amount: Option<&'a str>,
}

/// The different transaction types that a [TransactionCSVRecord] entry can have.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum TransactionType {
  Deposit,
  Withdrawal,
  Dispute,
  Resolve,
  Chargeback,
}

/// Parses data from CSV file into corresponding [Transaction] variants.
///
/// The purpose of our implementation of the CSV parsing is that we leverage
/// the csv crate and serde for doing the deserialization for us as one would
/// normally, while we also are able to further validate the data a little bit
/// and convert the records into distinct [Transaction] variants, without getting
/// nitty and gritty with serde.
///
/// It is possible that deserializing the rows directly into transaction types
/// could have noticeably better performance, but the spec said that some inefficiencies
/// were allowed in the toy implementation if they made the code cleaner, and
/// in this instance I feel that this way of doing it is easier to read and involves
/// less code than implementing serde deserialization for the individual
/// types of row data directly.
pub(crate) struct CSVInputParser {
  rdr: csv::Reader<std::fs::File>,
  headers: csv::StringRecord,
}

impl TryInto<CSVInputParser> for String {
  type Error = csv::Error;
  fn try_into (self) -> Result<CSVInputParser, Self::Error>
  {
    let mut rdr = csv::ReaderBuilder::new()
      .trim(csv::Trim::All)
      .from_path(self)?;
    let headers = rdr.headers()?.clone();
    Ok(CSVInputParser {
      rdr,
      headers,
    })
  }
}

impl CSVInputParser {
  /// Parses a raw CSV record into a transaction.
  pub(crate) fn parse_raw_record(&self, raw_record: csv::StringRecord) -> Result<(ClientId, TransactionId, Transaction), CSVInputParserError> {
    let record = raw_record.deserialize::<TransactionCSVRecord>(Some(&self.headers)).map_err(|e| CSVInputParserError::Csv(e))?;
    let transaction = match record.transaction_type {
      TransactionType::Deposit => {
        let amount = record.amount
          .ok_or(CSVInputParserError::DepositMustSpecifyAmount)
          .and_then(|a| a.try_into().map_err(|e| CSVInputParserError::AmountParseError(e)))?;
        Transaction::Deposit(amount)
      },
      TransactionType::Withdrawal => {
        let amount = record.amount
          .ok_or(CSVInputParserError::WithdrawalMustSpecifyAmount)
          .and_then(|a| a.try_into().map_err(|e| CSVInputParserError::AmountParseError(e)))?;
        Transaction::Withdrawal(amount)
      },
      TransactionType::Dispute => {
        if record.amount.is_some() {
          return Err(CSVInputParserError::DisputeCannotSpecifyAmount);
        }
        Transaction::Dispute
      },
      TransactionType::Resolve => {
        if record.amount.is_some() {
          return Err(CSVInputParserError::ResolveCannotSpecifyAmount);
        }
        Transaction::Resolve
      },
      TransactionType::Chargeback => {
        if record.amount.is_some() {
          return Err(CSVInputParserError::ChargebackCannotSpecifyAmount);
        }
        Transaction::Chargeback
      },
    };
    Ok((record.client_id, record.transaction_id, transaction))
  }
}

impl Iterator for CSVInputParser {
  type Item = Result<(ClientId, TransactionId, Transaction), CSVInputParserError>;
  fn next (&mut self) -> Option<Self::Item>
  {
    let mut raw_record = csv::StringRecord::new();
    let rec_read = self.rdr.read_record(&mut raw_record).map_err(|e| CSVInputParserError::Csv(e));
    match rec_read {
      Ok(did_read) => {
        if did_read {
          Some(self.parse_raw_record(raw_record))
        } else {
          None
        }
      },
      Err(e) => Some(Err(e)),
    }
  }
}

/// Transaction type and, in the case of deposits and withdrawals, the amount for the transaction.
#[derive(Debug)]
pub(crate) enum Transaction {
  Deposit(FractionalAmount),
  Withdrawal(FractionalAmount),
  Dispute,
  Resolve,
  Chargeback,
}

/// Errors returned by [CSVInputParser::parse_raw_record] and
/// also forwarded by [CSVInputParser::next] inside of the [Option]
/// returned by the latter.
#[derive(Error, Debug)]
pub(crate) enum CSVInputParserError {
  #[error("CSV error")]
  Csv(#[from] csv::Error),
  #[error("Deposit must specify amount")]
  DepositMustSpecifyAmount,
  #[error("Failed to parse amount")]
  AmountParseError(#[from] crate::transaction_engine::FractionalAmountParseError),
  #[error("Withdrawal must specify amount")]
  WithdrawalMustSpecifyAmount,
  #[error("Dispute cannot specify amount")]
  DisputeCannotSpecifyAmount,
  #[error("Resolve cannot specify amount")]
  ResolveCannotSpecifyAmount,
  #[error("Chargeback cannot specify amount")]
  ChargebackCannotSpecifyAmount,
}
