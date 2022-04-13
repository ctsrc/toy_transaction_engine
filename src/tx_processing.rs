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

/// Client ID is represented by u16 integer as per spec.
pub(crate) struct ClientId(u16);

/// Transaction ID is represented by u32 integer as per spec.
pub(crate) struct TransactionId(u32);

/// Transaction amount is precise to four places past the decimal point in inputs
/// and outputs. Therefore, we represent the amount internally as integer amounts
/// of 1/1000ths of the i/o amount unit.
pub(crate) struct TransactionAmount(u64);

/// Credit to client's account.
pub(crate) struct DepositTransaction(TransactionAmount);
/// Debit to client's account.
pub(crate) struct WithdrawalTransaction(TransactionAmount);

/// Claim that referenced transaction was erroneous and should be reversed.
pub(crate) struct DisputeTransaction(TransactionId);
/// A resolution to a dispute.
pub(crate) struct ResolveTransaction(TransactionId);
/// Final state of a dispute.
pub(crate) struct ChargebackTransaction(TransactionId);

/// All of the possible transaction types.
pub(crate) enum Transaction {
  Deposit(DepositTransaction),
  Withdrawal(WithdrawalTransaction),
  Dispute(DisputeTransaction),
  Resolve(ResolveTransaction),
  Chargeback(ChargebackTransaction),
}
