# Ethereum Censorship Monitor

The Ethereum Censorship Monitor is a tool that observes the network and looks
for evidence of transaction censorship by validators. The data is made available
both as a [visual dashboard](https://valitraitors.info) and as a REST API at
[](https://api.ethereum-censorship-monitor.org).

## Methodology

The monitor connects to an Ethereum node. It keeps track of the chain head and
the transaction pool as seen by the node. For each transaction, it records when
it was first seen as well when it left the pool. In the rare case that a
transaction appears again after it has already disappeared once, the original
observation is disregarded.

The pool is fetched once after each new observed head. Only transactions that
appear in this snapshot are considered as candidates for inclusion in the next
block, as only those are guaranteed to be valid according to the Ethereum node
(disregarding the transaction nonce which has to be checked separately). In the
unlikely case of a reorg, all data is reset to avoid considering invalid
transactions.

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
2. A configurable time has passed between the block's proposal time and the time
   the last node has seen the transaction. A typical lenient value is `8s`.
3. The monitor only knows the hash of the transaction (in this case, the
   following criteria can not be checked).
4. The block contains another transaction from the same sender (in this case,
   this transaction may have used up the account balance such that it would have
   been unable to pay the transaction fee).
5. The block is full, i.e., the block's gas usage plus the transaction's gas
   limit would have exceeded the block's gas limit.
6. The transaction's maximum base fee is lower than the block's base fee.
7. The transaction's tip is lower than the median of the tips of all other
   transactions in the block.
8. The transaction nonce does not match the sender's account nonce at the end of
   the block.

If any of the checks succeed, the transaction is considered justifiably not
included and further checks are skipped. If all checks fail, the transaction is
recorded as a "miss" and inserted as such into the database.

## Usage

Build the executable with `cargo build`. Run it with
`monitor run --config config.toml` where `config.toml` is a file looking like
this:

```
log = "info,monitor=debug,sqlx=warn"

execution_http_url = ""
main_execution_ws_url = ""
secondary_execution_ws_urls = []
consensus_http_url = ""

sync_check_enabled = true

db_enabled = true
db_connection = ""

propagation_time = 8

api_host = ""
api_port = 0
api_db_connection = ""
api_max_response_rows = 0
```

The Ethereum nodes should be run in a default configuration on standard hardware
to mirror what validators can be expected to do as well.

## API

We provide a REST API at `https://api.ethereum-censorship-monitor.org` that
makes the censorship data publicly accessible. The following endpoints for GET
requests exist:

- `/v0/misses`: Query individual misses
- `/v0/txs`: Query transactions that were missed in blocks
- `/v0/blocks`: Query blocks that missed transactions

The results can be paginated using the `from` and `to` query parameters. They
can be either UNIX timestamps (`1675320718`) or UNIX timestamps with an integer
cursor value (`"1675320718,5"`) and refer to the proposal time of the block. The
responses indicate if it is complete and the time range from which it contains
results. If the result is incomplete, continue with a follow-up query with
`from` set to the value of `to` in the response.

Results can be filtered by the following query parameters:

- `from` and `to` as explained above
- `block_number`: The number of the block with the miss
- `proposer_index`: The proposer index of the block with the miss
- `sender`: The sender of the missed transaction
- `propagation_time`: A cutoff value to filter transactions that didn't
  propagate for long enough.
- `min_tip`: A minimum tip to filter transactions that were too cheap.

In addition, `txs` and `blocks` can be filtered with the `min_num_misses`
parameter. Only transactions that have been missed at least `min_num_misses`
times or, respectively, blocks that missed at least `min_num_misses`
transactions will be included.

Note that for a transaction or a block to match the time filter given on `from`
and `to` only one corresponding miss has to fall into the window.

Example response for the query
`http https://api.ethereum-censorship-monitor.org/v0/txs\?min_num_misses=2\&from=1677024000\&to\=1677030000`:

```json
{
  "complete": true,
  "from": 1677024000,
  "items": [
    {
      "misses": [
        {
          "block_hash": "0x5491ca988720cdf025297b56b27760832e25f06f389d0d6484e4016ca4136ee4",
          "block_number": 16680602,
          "proposal_time": 1677027731,
          "proposer_index": 93365,
          "slot": 5850309,
          "tip": 2000000000
        },
        {
          "block_hash": "0x2eadd4593f9ab67d77b9d25ad1b7a12a83106c39d875fa1caa9b906bef603776",
          "block_number": 16680603,
          "proposal_time": 1677027743,
          "proposer_index": 268623,
          "slot": 5850310,
          "tip": 2000000000
        }
      ],
      "num_misses": 2,
      "sender": "0xab97925eb84fe0260779f58b7cb08d77dcb1ee2b",
      "tx_first_seen": 1677027718,
      "tx_hash": "0x90a6e5eed36febc999cc73210d17fde60210768ba8082149dcd8f42e8f3f3160",
      "tx_quorum_reached": 1677027721
    },
    {
      "misses": [
        {
          "block_hash": "0x1aeb008f064a33b3212cb5d9f090deefd23fa6a4e5c2c601cc24db88fdc12d6d",
          "block_number": 16680677,
          "proposal_time": 1677028643,
          "proposer_index": 60648,
          "slot": 5850385,
          "tip": 2862107286
        },
        {
          "block_hash": "0x05826cbaa6d6f670522aa37d12c00cd1f06ce3f1ab09478f5f15d4a6a31391f9",
          "block_number": 16680678,
          "proposal_time": 1677028655,
          "proposer_index": 106535,
          "slot": 5850386,
          "tip": 3000000000
        },
        {
          "block_hash": "0xb2a0c40ed3a6717247c56671853fafc1c5ea19427e78b745e0702157b991ee9f",
          "block_number": 16680679,
          "proposal_time": 1677028667,
          "proposer_index": 415544,
          "slot": 5850387,
          "tip": 1980920525
        }
      ],
      "num_misses": 3,
      "sender": "0x6eb9f934a4b3f77b495da71ad2c6676ac5c99dcc",
      "tx_first_seen": 1677028628,
      "tx_hash": "0xb629824e0407e6ea3a55ccb622546199fef9d17ad148f8ee3c24612878dc67b6",
      "tx_quorum_reached": 1677028633
    }
  ],
  "to": 1677030000
}
```
