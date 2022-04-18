# Toy Transaction Engine

This is an implementation of a toy transaction engine according to a specification
that was handed to me by someone for implementation.

The spec itself is not public, but I was told to make the repository public.

## Code Organization Overview

* Command-line utility resides in [`src/main.rs`](src/main.rs).
* CSV input parsing happens in [`transaction_engine_util/src/csv_input.rs`](transaction_engine_util/src/csv_input.rs).
* Transaction processing happens in [`transaction_engine/src/lib.rs`](transaction_engine/src/lib.rs).
* For CSV output, there is a single struct in [`transaction_engine_util/src/csv_output.rs`](transaction_engine_util/src/csv_output.rs)
  which is used in the command-line utilitity when it serializes CSV output with the [csv](https://crates.io/crates/csv) crate.

## Documentation

The code in this repository is annotated with doc strings. To generate documentation
and view it in your browser, run the following command:

```zsh
cargo doc --open
```

A current copy of the generated docs is deployed with GitHub Actions after each
push to the main branch of the toy transaction engine repository, and is available
at https://ctsrc.github.io/toy_transaction_engine/toy_transaction_engine/index.html

Besides the doc string annotations in the source, there are some relevant details
in this readme as well. In particular, the following sections of this readme
provides important information and context about the assumptions and the
implementation of the code:

* [Assumptions](#assumptions)
* [Implementation](#implementation)
  - [Handling of cases](#handling-of-cases)
    * [Withdrawals](#withdrawals)
    * [Deposits](#deposits)
    * [Disputes](#disputes)
    * [Resolves](#resolves)
    * [Chargebacks](#chargebacks)
  - [Correctness](#correctness)
    * [State of transactions](#state-of-transactions)
    * [Multithreading](#multithreading)

## Running Tests

A number of tests are included with this program and its parts.

To run the tests for all parts of the code in this repository, run:

```zsh
cargo test --workspace
```

## Command-line Usage Example

The toy transaction engine takes a single argument, which is the path to a CSV file
containing transactions. The program writes its output in CSV format to `stdout`.

In order to build and run the program you need to have the Rust toolchain
installed. Install the Rust toolchain from https://rustup.rs/ and ensure
that `cargo` is in your `$PATH`.

An example of using the program is given below, using a sample CSV input
file that is included in the repository root. This sample data is from
the spec, and I was told that it was ok to include sample data from
the spec as long as I documented the fact that I have done so.

```zsh
cargo run -- transactions.csv > accounts.csv
```

The resulting output redirected to `accounts.csv` from the command above
will look like the following once the implementation of the program
is complete.

```csv
client,available,held,total,locked
2,2.0000,0.0000,2.0000,false
1,1.5000,0.0000,1.5000,false
```

Note that as per the spec, the rows of data in the output is
not guaranteed to be in any particular order.

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
goes down one of two possible paths; transaction -> dispute -> resolve, or
transaction -> dispute -> chargeback. The chargeback, which is the final state
of a dispute, says that funds that were held have now been withdrawn.
Meanwhile, if the transaction under dispute was itself a withdrawal, and with
assumption 3 in mind, we are not able to "unwithdraw" money that has been withdrawn.

## Implementation

This section of the readme gives a high level overview of the implementation.
For more details about the implementation please see also the comments
throughout the source code itself.

Note also that where applicable, comments in the source are doc comments, so that
you can also run `cargo doc --open --bin toy_transaction_engine_command`
in the repo root and browse the generated documentation to read such comments.

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

(As with disputes, user id must match, and is handled because we include the user id
in the key that we look up dispute by.)

#### Chargebacks

When a transaction gets chargeback, the user shall not be able to dispute the same transaction
again as the amount has been sent back to their third party bank account. Therefore, we forget
about the dispute (and therefore also the transaction itself) after chargeback has been processed.

If a current dispute cannot be found when processing a chargeback
then we return an error indicating this.

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

#### State of transactions

State of past transactions is handled by maintaining two collections of transactions;
deposit transactions and dispute transactions. When a valid deposit transaction is
received, we remember the transaction for as long as we do not yet see a valid
dispute for the transaction.

When a transaction is disputed, we remove the transaction from our collection
of deposit transactions and put an entry for the transaction in our collection
of disputed transactions.

When a dispute is resolved, we remove the transaction from our collection of
disputed transactions and put an entry for it back in the collection of
deposit transactions, in case the transaction is disputed again in the future.

When a dispute is charged back, we remove the transaction from our collection of
disputed transactions, and we then forget about the transaction as it cannot
be disputed again in the future after it has been charged back.

#### Multithreading

After finishing the single threaded version of the program I will have
a look at splitting up the work across the logical CPU cores of the host
system. In the multi-threaded version of the program, we will read the CSV
input on a single thread and we will use the client id when deciding which
thread to send the transaction data for processing to. As such there will
not be any synchronization needed between the transaction processing threads.
