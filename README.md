# Rust Coding Test

This is a simple transaction processing engine.
It takes a list of records from a `.csv` file and produces `.csv` formatted results on stdout.

## Usage

```
$ cargo run -- input.csv > output.csv
```

Since the [pretty-env-logger](https://crates.io/crates/pretty_env_logger) crate is used for logging,
you can use environment variables to change the log level:

```
$ RUST_LOG=warn cargo run ...
```

## Assumptions

### Available RAM

Per requirement, the transaction are indexed over a `u32` ID. In total this would allow for up to
4,294,967,296 individual transactions (exhausting the ID space).
To allow rolling back transactions at a later point in time, all past transactions need to be
retrievable by ID.
There are multiple possible options to allow this, among them

1. reading the `.csv` file multiple times to look up transactions by ID,
2. storing the `.csv` file in a different format (RAM or disk) to allow faster lookup (e.g. a DB),
3. keeping track of all "past" transactions on the fly

   - by storing them in a simple data structure in RAM,
   - or by storing them in a persistent data structure on disk (e.g. a DB).

Due to the scope of this task as a test and due to the fact that the program can run in a single
pass by design, I don't think that storing data persistently using a DB is required here.
Looking up transactions in the `.csv` file would be horribly slow, so I will go with the memory
backed variant of point _3._.

Similar considerations need to be taken for the account information (`u16` ID, so up to 65,536
accounts).
My solution will also store the account information directly in RAM.

In total, this design decision requires the host system to provide enough RAM for 4,294,967,296
transactions and 65,536 accounts for the worst-case scenario.

### Only Deposits Can be Disputed

The requirements are unfortunately a bit unclear about what kind of transactions can be disputed.
Obviously only "deposit" and "withdrawal" are possible candidates, since they are the only
transactions that have their own transaction ID.

Because a dispute decreases the available amount, it intuitively only makes sense to dispute
deposits, since they would be essentially get reverted by a "dispute" followed by a "chargeback".
Disputing a withdrawal would, on the contrary, double the effect of a transaction which does not
seem desired.

However, there is a mathematically correct solution for this. Disputing a "withdrawal" would use
calculations with **negative** amounts, effectively increasing the amount of available funds and
potentially decreasing the held amounts to below 0.
That would be equivalent to the bank loaning money to their client during the dispute period to make
up for potentially fraudulent withdrawals by a third party.
In some ways, this even makes sense from a business perspective, but since negative amounts and
especially negative balances are not mentioned anywhere in the requirements, I have decided that
only "deposit" transactions can be disputed.

### Resolve or Chargeback for Undisputed Transactions

The requirements say that the application **can** ignore "resolve" transactions for undisputed
transactions. I will also extend this to "chargeback" transactions and read this as a **must**.

If this was truly to be meant to be undefined behavior, an obvious optimization would be to not
track _any_ information about the "dispute" state which would mean that the whole transaction
data structure could be immutable.
While this would make the whole task easier, I feel that this would cause quite a lot of trouble in
an actual banking application.
I will, under similar reasoning, not allow a second "dispute" for transaction that have already
completed "chargeback"

In total, this means that only the transactions depicted in the following image will successfully
change the state of a transaction in the `TransactionStore`.

![Transaction Store Overview](doc/transaction_store.png)

### Client ID Mismatch

Transactions of the kind "dispute", "resolve", and "chargeback" will be ignored if the client ID
does not match the client ID of the referenced transaction.

### Client Creation

Invalid transactions will not lead to the creation of client account data. That means in practice
that a client account will only be created if there is a deposit with this client ID.

### Insufficient Funds for Disputes

If the current available amount of an account is not sufficient to completely satisfy a "Dispute"
transaction, the maximum possible amount will be held. The same principle will be applied to
"Resolve" and "Chargeback". If the amount held is smaller than the original transactions
value, the maximum possible amount will be released or charged back.

### Handling Locked/Frozen Accounts

While not explicitly stated in the requirements, all deposits/withdrawals to locked/frozen accounts
will be refused.

## Design Decisions

### Performance

This implementation must focus on readability over performance. I have designed all modules to
accept iterators to avoid loading the whole input/output text at once.
A logical optimization step could be to pipeline I/O and actual transaction processing if the
current performance does not prove sufficient.
Having interfaces that already work on iterators will help with that.

Before starting any optimization at all, I would create a benchmark suite (usable through
`cargo bench`) and profile e.g. using [flamegraph](https://github.com/flamegraph-rs/flamegraph) for
evaluation.

The largest input file I tested contained 2^26 "deposit" transactions split over 2^16 clients
(about 1.8 GB of `.csv` data) which took around 110 seconds to process on my machine.

### Money Representation

I have decided to go with the `rust_decimal` crate for simplicity since it requires no extra effort
for serialization/deserialization with `serde`. An easy optimization in RAM and CPU usage would be
to use a simple fixed point format (i.e. an `i64` representing 1/10,000 of the unit currency).

This crate is also one of the few possible causes of `panic!()` in the implementation. Attempting
to parse too large numbers will result in an integer overflow!

### Correctness & Robustness

I use Rust's type system wherever possible to ensure that failures cannot happen by design. As an
example, I eliminate the `Option<Amount>` from the data structures within the parser, so that the
type used itself clearly communicates whether a transaction has an amount or not.

I do not use any source of `panic()!` in my own code (apart from `unwrap()` and `unwrap_err()` in
unit tests).
A possible source of panic is, as listed above, the `rust_decimal` crate.

### Testing

All modules have been developed against unit tests which are part of the module files.
I also have run the final version against a separate integration test suite that is not fully
automated. This test suite is not part of this repository because it relies on third-party data and
code.

Something that I considered, but did not do to keep the project smaller and the dependency list
short is adding property based testing using the [proptest](https://crates.io/crates/proptest).
A good application would be e.g. to check that regardless of the input, the available, held, and
total balance of an account must remain positive.

## Input Validation

All IDs must be in their allowed range (`u32`/`u16`) and all transaction amounts must be positive.
Violation will not lead to a crash but it will cause undefined results.
Checking and filtering for valid inputs using these criteria is definitely something to consider
for further improvements.
