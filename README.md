# Toy Transaction Engine

This is an implementation of a toy transaction engine according to a specification
that was handed to me by someone for implementation.

The spec itself is not public, but I was told to make the repository public.

## Assumptions

In addition to the assumptions listed in the spec, I am making some further assumptions:

1. We are not required to keep a record of the individual transactions themselves.
2. We are not required to keep a record of the individual results of the individual transactions.
3. Deposits and withdrawals are transactions between the system and an external
   party such as for example a bank.
4. Users can dispute deposits, but they cannot dispute withdrawals.
5. A frozen account cannot withdraw money.
6. A frozen account is still able to deposit money.
7. A frozen account is also still able to dispute, resolve and chargeback.

Assumption 4 requires some explanation: According to the spec, the dispute process
goes transaction -> dispute -> resolve -> chargeback. The chargeback, which is the
final state of a dispute, says that funds that were held have now been withdrawn.
Meanwhile, if the transaction under dispute was itself a withdrawal, and with
assumption 3 in mind, we are not able to "unwithdraw" money that has been withdrawn.

## Implementation

This section of the readme gives a high level overview of the implementation.
For more details about the implementation please see also the comments
throughout the source code itself.

Note also that where applicable, comments in the source are doc comments, so that
you can also run `cargo doc --open` in the repo root and browse the generated
documentation to read such comments.

### Handling of cases

#### Withdrawals

As a consequence of our assumption that withdrawals cannot be disputed, we can
"forget" the withdrawal transaction as soon as we have processed it.

If a withdrawal attempts to withdraw more than the available amount, then
we return an error indicating that this is not allowed.

If the account is currently frozen then we return an error indicating this.

#### Deposits

We need to remember deposits for a while -- potentially "forever" -- as they could
later get disputed.

#### Disputes

We need to remember disputes until we see either a resolve or a chargeback for
the disputed transaction.

If a transaction is already under dispute then we will return an error indicating as much.

If a transaction cannot be found then we return an error indicating this.

If the client id of the user submitting the dispute does not match the client id
of the user that created the transaction then we consider the dispute to be not valid.
This situation is handled for us when we look for the transaction as we include
the user id in the key that we look up transaction by.

A dispute for a past deposit can result in negative available balance on the account,
if there have been withdrawals or disputes since the time at which the deposit was made.
This is fine and expected.

#### Resolves

When a dispute is resolved, we forget the dispute, but we keep remembering the deposit
in case the same transaction is disputed again by the user.

If a current dispute cannot be found then we return an error indicating this.

(Meanwhile, if a dispute is found then it also means that the transaction still exists,
so we don't need to have an error type for a situation where transaction does not exist.)

(As with disputes, user id must match, and is handled because we include the user id
in the key that we look up dispute by.)

#### Chargebacks

When a transaction gets chargeback, the user shall not be able to dispute the same transaction
again as the amount has been sent back to their third party bank account. Therefore, we forget
about the dispute and the transaction after chargeback has been processed.

If a current dispute cannot be found when processing a chargeback
then we return an error indicating this.

(As with resolves, if a dispute is found then the transaction still exists, and
we don't need to have an error type for a situation where transaction does not exist.)

(As with disputes and resolves, user id must match, and is handled because we include the user id
in the key that we look up dispute by.)

When the chargeback is processed, the total funds can become negative. This is expected.
I guess that is part of the reason why the spec says to freeze the account of the user
after processing a chargeback.

### Correctness

Assuming that the logic of the handling of the cases as listed above is correct,
the implementation itself should also be correct. Transactions are processed
serially in the order that they are read, with the code currently being
single threaded, and we use `std::collections::HashMap` for the deposits
and disputes that we want to remember, as well as for the accounts.

After finishing the single threaded version of the program I will have
a look at splitting up the work across the logical CPU cores of the host
system. In the multi-threaded version of the program, we will read the CSV
input on a single thread and we will use the client id when deciding which
thread to send the transaction data for processing to. As such there will
not be any synchronization needed between the transaction processing threads.
