//! This module forms the core of the transaction processing.
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

use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;

/// Client ID is represented by u16 integer as per spec.
#[derive(Deserialize, Debug)]
pub(crate) struct ClientId(u16);

/// Transaction ID is represented by u32 integer as per spec.
#[derive(Deserialize, Debug)]
pub(crate) struct TransactionId(u32);

/// Transaction amount is precise to four places past the decimal point in inputs
/// and outputs. Therefore, we represent the amount internally as integer fractional
/// amounts of 1/1000ths of the i/o amount unit.
#[derive(Debug)]
pub(crate) struct FractionalAmount(u64);

/// Contains the account data for a single user.
#[derive(Debug)]
pub(crate) struct Account {
  available_amount: FractionalAmount,
  held_amount: FractionalAmount,
}

/// Contains the accounts of all users for which we have processed valid transactions.
pub(crate) type Accounts = HashMap<ClientId, Account>;

pub(crate) struct TransactionProcessor {
  accounts: Accounts,
  /// Contains deposit transactions we have seen and which we are holding onto until,
  /// if ever, they get disputed.
  ///
  /// When a transaction is disputed by the user, we remove the transaction from the
  /// collection of deposit transactions and put an entry for the transaction
  /// in the collection of disputed transactions, [DisputeTransactions], instead.
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
  /// See also [DepositTransactions] for details about what happens to a transaction
  /// after it has been disputed and then it has either been resolved or charged back.
  dispute_transactions: HashMap<(ClientId, TransactionId), FractionalAmount>,
}

impl TransactionProcessor {
  pub(crate) fn new () -> Self
  {
    Self {
      accounts: Default::default(),
      deposit_transactions: Default::default(),
      dispute_transactions: Default::default()
    }
  }
  /// Credit to client's account.
  pub(crate) fn deposit (&mut self, client_id: ClientId, transaction_id: TransactionId, amount: FractionalAmount)
  {
  }
  /// Debit to client's account.
  pub(crate) fn withdraw (&mut self, client_id: ClientId, transaction_id: TransactionId, amount: FractionalAmount) -> Result<(), TransactionWithdrawError>
  {
    Ok(())
  }
  /// Claim that referenced transaction was erroneous and should be reversed.
  pub(crate) fn dispute (&mut self, client_id: ClientId, transaction_id: TransactionId) -> Result<(), TransactionDisputeError>
  {
    Ok(())
  }
  /// A resolution to a dispute.
  pub(crate) fn resolve (&mut self, client_id: ClientId, transaction_id: TransactionId) -> Result<(), TransactionResolveError>
  {
    Ok(())
  }
  /// Final state of a dispute.
  pub(crate) fn chargeback (&mut self, client_id: ClientId, transaction_id: TransactionId) -> Result<(), TransactionChargebackError>
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

#[derive(Error, Debug)]
pub enum TransactionWithdrawError {
}

#[derive(Error, Debug)]
pub enum TransactionDisputeError {
}

#[derive(Error, Debug)]
pub enum TransactionResolveError {
}

#[derive(Error, Debug)]
pub enum TransactionChargebackError {
}
