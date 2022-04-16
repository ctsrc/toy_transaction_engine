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
//!
//! let client_a = ClientId::from(1u16);
//! let amount_1 = "1.5000000000001".try_into().unwrap();
//! let tx_1 = TransactionId::from(1u32);
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
//! transaction_processor.deposit(client_a, tx_1, amount_1).unwrap();
//! transaction_processor.withdraw(client_a, tx_2, amount_2).unwrap();
//!
//! // After processing the above two transactions, we expect to find that
//! // there is record of a single account, belonging to client_a. We expect that
//! // this account is not frozen, that the amount available on the account
//! // should have string representation equal to "1.2500", and that the held
//! // amount on the account should have string representation
//! // equal to "0.0000".
//! let accounts: Accounts = transaction_processor.into();
//! assert_eq!(accounts.len(), 1);
//! let (client_id_ret, acc_a) = accounts.into_iter().next().unwrap();
//! assert_eq!(client_id_ret, client_a);
//! assert!(!acc_a.is_frozen());
//! assert_eq!(acc_a.get_available().to_string(), "1.2500");
//! assert_eq!(acc_a.get_held().to_string(), "0.0000");
//! ```
//!
//! ### Dispute
//!
//! ```
//! use transaction_engine::{TransactionProcessor, ClientId, TransactionId, Accounts};
//!
//! let mut transaction_processor = TransactionProcessor::new();
//!
//! let client_a = ClientId::from(1u16);
//! let amount_1 = "1.5".try_into().unwrap();
//! let tx_1 = TransactionId::from(1u32);
//! let tx_2 = TransactionId::from(2u32);
//! let amount_2 = "0.25".try_into().unwrap();
//!
//! transaction_processor.deposit(client_a, tx_1, amount_1).unwrap();
//! transaction_processor.withdraw(client_a, tx_2, amount_2).unwrap();
//! transaction_processor.dispute(client_a, tx_1).unwrap();
//!
//! let accounts: Accounts = transaction_processor.into();
//! let (_, acc_a) = accounts.into_iter().next().unwrap();
//! assert_eq!(acc_a.get_available().to_string(), "-0.2500");
//! assert_eq!(acc_a.get_held().to_string(), "1.5000");
//! ```

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt::Formatter;

use derive_more::{Add, Display, From, Sub};
use serde::Deserialize;
use thiserror::Error;

/// Client ID is represented by u16 integer as per spec.
#[derive(Deserialize, Debug, Display, From, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ClientId(u16);

/// Transaction ID is represented by u32 integer as per spec.
#[derive(Deserialize, Debug, Display, From, Copy, Clone, Hash, Eq, PartialEq)]
pub struct TransactionId(u32);

/// Transaction amount is precise to four places past the decimal point in inputs
/// and outputs. Therefore, we represent the amount internally as integer fractional
/// amounts of 1/10,000ths (one ten thousands) of the i/o amount unit.
///
/// We use signed integers because even though deposits and withdrawals themselves
/// are not allowed to be negative, the available amount and the total amount on
/// an account can become negative, as explained in the main readme file.
#[derive(Debug, Add, From, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Sub)]
pub struct FractionalAmount(i64);

impl std::fmt::Display for FractionalAmount {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
  {
    write!(f, "{}{}.{:04}", if self.0 >= 0 {""} else {"-"}, (self.0 / 10_000).abs(), (self.0 % 10_000).abs())
  }
}

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
    let decimal_portion_amount = decimal_portion.parse::<i64>()
      .map_err(|e| FractionalAmountParseError::DecimalPortionParseIntError(e))?;
    let mut fractional_portion_amount = 0;
    if let Some(fractional_portion) = splitter.next() {
      let mut magnitude = 1_000;
      for digit in fractional_portion.chars() {
        let digit = digit.to_digit(10).ok_or(FractionalAmountParseError::NonDigitInFractionalPortion)? as i64;
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
#[derive(Debug, Default)]
pub struct Account {
  available_amount: FractionalAmount,
  held_amount: FractionalAmount,
  frozen: bool,
}

impl Account {
  pub fn is_frozen (&self) -> bool {
    self.frozen
  }
  pub fn get_available (&self) -> FractionalAmount {
    self.available_amount
  }
  pub fn get_held (&self) -> FractionalAmount {
    self.held_amount
  }
  pub fn get_total (&self) -> FractionalAmount {
    self.available_amount + self.held_amount
  }
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
  pub fn deposit (&mut self, client_id: ClientId, transaction_id: TransactionId, amount: FractionalAmount) -> Result<(), TransactionDepositError>
  {
    if amount.0 < 0 {
      return Err(TransactionDepositError::CannotDepositANegativeAmount);
    }
    let account = self.accounts.entry(client_id).or_insert_with(|| Default::default());
    account.available_amount = account.available_amount + amount;
    self.deposit_transactions.insert((client_id, transaction_id), amount);
    Ok(())
  }
  /// Debit to client's account.
  pub fn withdraw (&mut self, client_id: ClientId, transaction_id: TransactionId, amount: FractionalAmount) -> Result<(), TransactionWithdrawError>
  {
    if amount.0 < 0 {
      return Err(TransactionWithdrawError::CannotWithdrawANegativeAmount);
    }
    let account = self.accounts.entry(client_id).or_insert_with(|| Default::default());
    if account.available_amount < amount {
      return Err(TransactionWithdrawError::InsufficientAmountAvailableForWithdrawal);
    }
    account.available_amount = account.available_amount - amount;
    Ok(())
  }
  /// Claim that referenced transaction was erroneous and should be reversed.
  pub fn dispute (&mut self, client_id: ClientId, transaction_id: TransactionId) -> Result<(), TransactionDisputeError>
  {
    let k = (client_id, transaction_id);
    let disputed_amount = self.deposit_transactions.remove(&k).ok_or(TransactionDisputeError::ReferencedTransactionNotFoundForSpecifiedClient)?;
    // XXX: The unwrap for the account is fine because we have found the deposit transaction,
    //      and because we create accounts when we process deposits that means that
    //      an account for the client exists for sure :)
    let acc = self.accounts.get_mut(&client_id).unwrap();
    acc.available_amount = acc.available_amount - disputed_amount;
    acc.held_amount = acc.held_amount + disputed_amount;
    self.dispute_transactions.insert(k, disputed_amount);
    Ok(())
  }
  /// A resolution to a dispute.
  pub fn resolve (&mut self, client_id: ClientId, transaction_id: TransactionId) -> Result<(), TransactionResolveError>
  {
    Ok(())
  }
  /// Final state of a dispute.
  pub fn chargeback (&mut self, client_id: ClientId, transaction_id: TransactionId) -> Result<(), TransactionChargebackError>
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
pub enum TransactionDepositError {
  #[error("Cannot deposit a negative amount")]
  CannotDepositANegativeAmount,
}

/// Errors returned by [TransactionProcessor::withdraw].
#[derive(Error, Debug)]
pub enum TransactionWithdrawError {
  #[error("Cannot withdraw a negative amount")]
  CannotWithdrawANegativeAmount,
  #[error("Insufficient amount available for withdrawal")]
  InsufficientAmountAvailableForWithdrawal,
}

/// Errors returned by [TransactionProcessor::dispute].
#[derive(Error, Debug)]
pub enum TransactionDisputeError {
  #[error("Referenced transaction not found for specified client")]
  ReferencedTransactionNotFoundForSpecifiedClient,
}

/// Errors returned by [TransactionProcessor::resolve].
#[derive(Error, Debug)]
pub enum TransactionResolveError {
}

/// Errors returned by [TransactionProcessor::chargeback].
#[derive(Error, Debug)]
pub enum TransactionChargebackError {
}
