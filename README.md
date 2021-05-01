# Rust Coding Test

This is a simple transaction processing engine.
It takes a list of records from a `.csv` file and produces `.csv` formatted results on stdout.

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
backed variant of point *3.*.

Similar considerations need to be taken for the account information (`u16` ID, so up to 65,536
accounts).
My solution will also store the account information directly in RAM.

In total, this design decision requires the host system to provide enough RAM for 4,294,967,296
transactions and 65,536 accounts for the worst-case scenario.
How this translates to a size in GB depends on the exact representation of the data in memory, so
this will need to be clarified later.

### Resolve or Chargeback for undisputed transactions

The requirements say that the application **can** ignore "resolve" transactions for undisputed
transactions. I will also extend this to "chargeback" transactions and read this as a **must**.

If this was truly to be meant to be undefined behavior, an obvious optimization would be to not
track *any* information about the "dispute" state which would mean that the whole transaction
data structure could be immutable.
While this would make the whole task easier, I feel that this would cause quite a lot of trouble in
an actual banking application.
I will, under similar reasoning, not allow a second "dispute" for transaction that have already
completed "chargeback"

## Design Decisions

### Parallelism

This implementation must focus on readability over performance. However, pipelining I/O and actual
transaction processing seems to be a very natural optimization and I will try to consider this for
my attempt at the problem.
