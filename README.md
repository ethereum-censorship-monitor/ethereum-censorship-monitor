# Ethereum Censorship Monitor

The Ethereum Censorship Monitor is a tool that observes the network and looks
for evidence of transaction censorship by validators. The data is made available
both as a [](https://www.ethereum-censorship-monitor.org) visual dashboard and
as a REST API at [](https://api.ethereum-censorship-monitor.org).

## Methodology

The monitor connects to both an Ethereum node. It keeps track of the chain head
and the transaction pool as seen by the node. For each transaction, it records
when it was first seen as well when it left the pool.

The pool is fetched once after each new head. Only transactions that appear in
this snapshot are considered as candidates for inclusion in the next block, as
only those are known to be valid (disregarding the transaction nonce which has
to be checked separately). In the unlikely case of a reorg, all data is reset to
avoid considering invalid transactions.

Querying the pool only once per slot results in very poor time resolution of the
first seen timestamp. To mitigate this, the monitor subscribes to the node's
pending transaction stream which gives real time notifications of observed
transactions, even if only of their hashes. For each of those, it records a
timestamp in order to increase the timing accuracy (though transactions have to
still appear in a pool snapshot in order to be considered for inclusion). In
addition, the data is cross-validated by subscribing to the same data stream
from a set of secondary nodes at different locations in the network. This allows
us to check that transactions are available everywhere, not only on a single
node.

After each observed head, the monitor checks if there are transactions in the
pool the block proposer should have included. This check is delayed until after
the next pool observation. The reason being that transactions observed between
the previous pool snapshot and the block came in via the pending transaction
streams and therefore only their hashes are known. The next pool observation
backfills the transaction data.

The set of transactions to check for inclusion in a block is the set of
transactions that are visible in the pool at the block's proposal time (i.e.,
the start of the corresponding slot), but are not included. For each of those
transactions, the following checks are performed in order:

1. At least one connected node has not observed the transaction (in this case,
   it is plausible that the proposer did not know of the transaction either).
2. The monitor only knows the hash of the transaction (in this case, the
   following criteria can not be checked).
3. The block contains another transaction from the same sender (in this case,
   this transaction may have used up the account balance such that it would have
   been unable to pay the transaction fee).
4. The block is full, i.e., the block's gas usage plus the transaction's gas
   limit would have exceeded the block's gas limit.
5. The transaction's maximum base fee is lower than the block's base fee.
6. The transaction's tip is lower than the minimum tip of all other transactions
   in the block.
7. The transaction nonce does not match the sender's account nonce at the end of
   the block.

If any of the checks succeed, the transaction is considered justifiably not
included and further checks are skipped. If all checks fail, the transaction is
recorded as a "miss" and inserted as such into the database.

## Usage

Build the executable with `cargo build`. Run it with
`monitor run --config config.toml` where `config.toml` is a file looking like
this:

```
log = "info,monitor=debug,sqlx=warn,warp=warn"

execution_http_url = ""
main_execution_ws_url = ""
secondary_execution_ws_urls = []
consensus_http_url = ""

sync_check_enabled = true

db_enabled = true
db_connection = ""

metrics_enabled = true
metrics_endpoint = "127.0.0.1:8080"
```
