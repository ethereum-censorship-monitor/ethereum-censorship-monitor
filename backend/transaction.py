import time

class Chain:

    def __init__(self):
        self.pending_transactions = dict()
        self.timestamps = dict()
        self.latest_block_header = None

    def acknowledge_transaction(self, tx_hash, timestamp):
        self.timestamps[tx_hash] = timestamp

    def add_transaction(self, transaction_data):
        internal_transaction = self.pending_transactions.pop(transaction_data["hash"], None)
        if internal_transaction is not None:
            return internal_transaction

        timestamp = self.timestamps.pop(transaction_data["hash"], time.time())

        return Transaction(transaction_data, timestamp)

    def new_block(self, new_block_header, block):
        self.process_transactions(block)
        self.analyze_censorship(block)
        self.latest_block_header = new_block_header
        print(f"Internal Transactions: {len(self.pending_transactions)}")

    def on_mempool(self, mempool):
        transactions_pending_in_mempool = mempool["pending"]

        persist_transactions = {
            transactions[min(transactions.keys())]["hash"]: self.add_transaction(transactions[min(transactions.keys())])
            for transactions in transactions_pending_in_mempool.values()}
        self.pending_transactions = persist_transactions

    def process_transactions(self, block):
        transactions = block["transactions"]
        for transaction in transactions:
            self.process_transaction(transaction)

    def process_transaction(self, transaction):

        ignore_transactions = list()

        for internal_transaction in self.pending_transactions:
            if internal_transaction.data["from"] == transaction["from"]:
                ignore_transactions.append(internal_transaction)

        for ignore_transaction in ignore_transactions:
            self.pending_transactions.pop(ignore_transaction.data["hash"])

    def analyze_censorship(self, block):
        for transaction in self.pending_transactions:
            if self.latest_block_header["timestamp"] < transaction.timestamp:
                continue
            lowest_priority_fee = _find_lowest_priority_fee(block)
            max_priority_fee = _get_max_priority_fee(transaction, int(block["baseFeePerGas"],16))

            if max_priority_fee < lowest_priority_fee:
                continue

            if block["baseFeePerGas"] > transaction.data["baseFeePerGas"]:
                continue

            if transaction.data["gasLimit"] > block["gasLimit"] - block["gasUsed"]:
                continue

            print(f"CENSORED TRANSACTION FOUND")
            transaction.censored_blocks.append(block["number"])


def _get_max_priority_fee(transaction, base_fee_per_gas):
    if int(transaction["type"], 16) == 0:
        gas_price = int(transaction["gasPrice"], 16)
        return gas_price - base_fee_per_gas

    else:
        return int(transaction["maxPriorityFeePerGas"], 16)


def _find_lowest_priority_fee(block):
    base_fee_per_gas = int(block["baseFeePerGas"],16)
    lowest_priority_fee = 0
    for transaction in block["transactions"]:
        priority_fee = _get_max_priority_fee(transaction, base_fee_per_gas)

        lowest_priority_fee = min(lowest_priority_fee, priority_fee)

    return lowest_priority_fee


class Transaction:

    def __init__(self, data, timestamp):
        self.timestamp = timestamp
        self.data = data
        self.censored_blocks = list()


