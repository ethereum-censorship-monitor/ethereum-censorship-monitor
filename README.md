# Ethereum Censorship Monitor

The Ethereum Censorship Monitor is a tool that continously observes the mempool and the blockchain
looking for evidence of censorship by validators. The collected data is aggregated and displayed by
a web interface. You can find a hosted version at [](https://www.ethereum-censorship-monitor.org).

## Censorship Criteria

Consider a transaction showing up in our mempool at some point in time. We now look at all blocks
subsequently proposed that don't include it, until one does, or until the transaction disappears
from the mempool. Here are the reasons the monitor considers valid for not including the
transaction (note that validity of the transaction itself is assumed -- otherwise it wouldn't be
in the mempool):

- The block is the first block after the transaction was observed. This criterion makes sure the
  transaction had ample time (at least 1 slot) to propagate through the whole network.
- The block is full.
- The block's base fee is smaller than the maximum base fee the transaction is willing to pay.
- The smallest tip in the block is smaller than the transaction's tip.
- The block contains a transaction that replaces the not-included one.

In all other cases, the monitor claims the proposer of the block has censored the transaction.

The following diagram illustrates how transactions are analyzed when a new block arrived

![censorship_with_background](https://user-images.githubusercontent.com/10088275/190893936-2d438299-3830-4fec-bc35-316ef7df2062.png)


## Usage

Install and run the background with the following commands, to be executed in the root of the
repository:

1. `poetry install`
2. `poetry shell`
3. `python backend/main.py --rpc <geth-endpoint> --rest-host localhost --rest-port 8080`

For the frontend, go to the `frontend` directory and run:

1. `npm install`
2. `npm run dev`

Define the endpoint of the REST API backend using the `VITE_REST_API_ENDPOINT` environment
variable.
