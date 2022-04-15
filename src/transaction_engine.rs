//! This module forms the core of the transaction processing
//! for the toy transaction engine command.
//!
//! In a production system we could imagine that this portion of the code
//! was running across a fleet of servers, with each server handling transactions
//! for individual shards of the users. This would work quite straightforward
//! for the current toy spec as the types of transactions we are concerned about
//! are per individual user and never between one user and the other. In theory
//! then, if our toy code was to be deployed to a fleet of servers, we could
//! shard on client id.
//!
//! Beyond the toy spec though, something that would become a concern over time
//! is balancing users across the fleet of servers. Certainly there is a lot
//! that would need to be considered. I'm eager to talk about that as well,
//! as I have some thoughts about it and much to learn about it.
//!
//! Going further beyond the toy spec, there would certainly need to be
//! all kinds of interaction between the transaction processing and other
//! systems, and things would quickly become much more complex.
//!
//! ## Examples of using this module
//!
//! The following shows some basic examples of processing transactions
//! using this module.
//!
//! These examples run as doc tests with `cargo test`.
//!
//! In these examples, unwrap is used in order to keep the code short
//! and also because we want the doc tests to fail except where the
//! point is to show that invalid transactions are not accepted.
//!
//! ### Deposits and withdrawals
//!
//! ```
//! use transaction_engine::{TransactionProcessor, ClientId, TransactionId, Accounts};
//!
//! let mut transaction_processor = TransactionProcessor::new();
//! let client_a = ClientId::from(1u16);
//! let amount_1 = "1.5000000000001".try_into().unwrap();
//! let tx_1 = TransactionId::from(1u32);
//!
//! let tx_2 = TransactionId::from(2u32);
//! // XXX: Precision of fractional part is four digits as per spec.
//! //      Fifth digit and onwards will be "chopped off", without rounding.
//! //
//! //      In other words, the following are all equivalent:
//! //
//! //let amount_2 = "0.25".try_into().unwrap();
//! //let amount_2 = "0.2500".try_into().unwrap();
//! //let amount_2 = "0.250099999999999999999999".try_into().unwrap();
//! let amount_2 = "0.25009".try_into().unwrap();
//!
//! transaction_processor.deposit(&client_a, &tx_1, amount_1);
//! transaction_processor.withdraw(&client_a, &tx_2, amount_2);
//!
//! let accounts: Accounts = transaction_processor.into();
//!
//! assert!(accounts.len() == 1);
//! ```

use std::collections::HashMap;

use derive_more::{Add, Display, From};
use serde::Deserialize;
use thiserror::Error;

/// Client ID is represented by u16 integer as per spec.
#[derive(Deserialize, Debug, Display, From)]
pub struct ClientId(u16);

/// Transaction ID is represented by u32 integer as per spec.
#[derive(Deserialize, Debug, Display, From)]
pub struct TransactionId(u32);

/// Transaction amount is precise to four places past the decimal point in inputs
/// and outputs. Therefore, we represent the amount internally as integer fractional
/// amounts of 1/10,000ths (one ten thousands) of the i/o amount unit.
#[derive(Debug, Add, From)]
pub struct FractionalAmount(u64);

/// Turns a string like `"321.54689498498549"` into a [FractionalAmount]
/// with 4 digits of precision for fractional portion as per spec.
///
/// In the case of a string like `"321.54689498498549"`, the value of
/// the [FractionalAmount] will be `3215468`.
///
/// The full string is read in order to ensure that no non-digits
/// are present in the input.
impl TryInto<FractionalAmount> for &str {
  type Error = FractionalAmountParseError;
  fn try_into (self) -> Result<FractionalAmount, Self::Error>
  {
    let mut splitter = self.splitn(2, ".");
    // XXX: The unwrap below is fine because even with an empty string,
    //      the first call to next() will return Some(&str).
    let decimal_portion = splitter.next().unwrap();
    let decimal_portion_amount = decimal_portion.parse::<u64>()
      .map_err(|e| FractionalAmountParseError::DecimalPortionParseIntError(e))?;
    let mut fractional_portion_amount = 0;
    if let Some(fractional_portion) = splitter.next() {
      let mut magnitude = 1_000;
      for digit in fractional_portion.chars() {
        let digit = digit.to_digit(10).ok_or(FractionalAmountParseError::NonDigitInFractionalPortion)? as u64;
        if magnitude > 1 {
          fractional_portion_amount += digit * magnitude;
          magnitude /= 10;
        } else if magnitude == 1 {
          fractional_portion_amount += digit;
          magnitude = 0;
        }
        // XXX: After we finish reading the up to 4 digits that we care about
        //      in terms of precision, we do not break out of the loop. The reason for this
        //      is that we still want to ensure that all remaining characters are digits.
      }
    };
    Ok(FractionalAmount(decimal_portion_amount * 10_000 + fractional_portion_amount))
  }
}

/// Errors returned for [TryInto::try_into]::<[FractionalAmount]> on &[str].
#[derive(Error, Debug)]
pub enum FractionalAmountParseError {
  #[error("Failed to parse decimal portion of amount")]
  DecimalPortionParseIntError(#[from] std::num::ParseIntError),
  #[error("Non-digit in fractional portion of amount")]
  NonDigitInFractionalPortion,
}

/// Contains the account data for a single user.
#[derive(Debug)]
pub struct Account {
  available_amount: FractionalAmount,
  held_amount: FractionalAmount,
}

/// Contains the accounts of all users for which we have processed valid transactions.
pub type Accounts = HashMap<ClientId, Account>;

/// Processes transactions and provides final balances for accounts for which
/// transactions have been processed.
pub struct TransactionProcessor {
  accounts: Accounts,
  /// Contains deposit transactions we have seen and which we are holding onto until,
  /// if ever, they get disputed.
  ///
  /// When a transaction is disputed by the user, we remove the transaction from the
  /// collection of deposit transactions and put an entry for the transaction
  /// in the collection of disputed transactions instead.
  ///
  /// A transaction is put back into the collection of deposit transactions
  /// if the transaction is subsequently resolved after having been disputed.
  /// This is done in case there are any future disputes about the transaction.
  ///
  /// Meanwhile, if the transaction changes state from disputed to charged back,
  /// then the transaction will not be put back into the deposit transactions
  /// collection, as
  deposit_transactions: HashMap<(ClientId, TransactionId), FractionalAmount>,
  /// Contains dispute transactions we have seen and which we are holding onto until,
  /// if ever, they either get resolved or charged back.
  ///
  /// See also the description on the deposit transactions field for details about
  /// what happens to a transaction after it has been disputed and then it has
  /// either been resolved or charged back.
  dispute_transactions: HashMap<(ClientId, TransactionId), FractionalAmount>,
}

/// Processes deposit, withdraw, dispute, resolve and chargeback transactions.
impl TransactionProcessor {
  pub fn new () -> Self
  {
    Self {
      accounts: Default::default(),
      deposit_transactions: Default::default(),
      dispute_transactions: Default::default()
    }
  }
  /// Credit to client's account.
  pub fn deposit (&mut self, client_id: &ClientId, transaction_id: &TransactionId, amount: FractionalAmount)
  {
  }
  /// Debit to client's account.
  pub fn withdraw (&mut self, client_id: &ClientId, transaction_id: &TransactionId, amount: FractionalAmount) -> Result<(), TransactionWithdrawError>
  {
    Ok(())
  }
  /// Claim that referenced transaction was erroneous and should be reversed.
  pub fn dispute (&mut self, client_id: &ClientId, transaction_id: &TransactionId) -> Result<(), TransactionDisputeError>
  {
    Ok(())
  }
  /// A resolution to a dispute.
  pub fn resolve (&mut self, client_id: &ClientId, transaction_id: &TransactionId) -> Result<(), TransactionResolveError>
  {
    Ok(())
  }
  /// Final state of a dispute.
  pub fn chargeback (&mut self, client_id: &ClientId, transaction_id: &TransactionId) -> Result<(), TransactionChargebackError>
  {
    Ok(())
  }
}

impl Into<Accounts> for TransactionProcessor {
  /// Consumes self and returns final account data for all accounts
  /// for which valid transactions have been processed.
  fn into (self) -> Accounts {
    self.accounts
  }
}

/// Errors returned by [TransactionProcessor::withdraw].
#[derive(Error, Debug)]
pub enum TransactionWithdrawError {
}

/// Errors returned by [TransactionProcessor::dispute].
#[derive(Error, Debug)]
pub enum TransactionDisputeError {
}

/// Errors returned by [TransactionProcessor::resolve].
#[derive(Error, Debug)]
pub enum TransactionResolveError {
}

/// Errors returned by [TransactionProcessor::chargeback].
#[derive(Error, Debug)]
pub enum TransactionChargebackError {
}
