import time
from copy import deepcopy

from util import hex_to_int


class Chain:

    def __init__(self):
        self.pending_transactions = dict()
        self.nonces = dict()
        self.timestamps = dict()
        self.latest_block_header = None
        self.rpc_client = None

    def acknowledge_transaction(self, tx_hash, timestamp):
        self.timestamps[tx_hash] = timestamp

    def create_transaction(self, transaction_data):
        internal_transaction = self.pending_transactions.pop(transaction_data["hash"], None)
        if internal_transaction is not None:
            return internal_transaction

        timestamp = self.timestamps.pop(transaction_data["hash"], int(time.time()))

        if hex_to_int(transaction_data["type"]) == 2:
            return TransactionType2.from_dict(transaction_data, timestamp)
        return TransactionType0.from_dict(transaction_data, timestamp)

    async def new_block(self, new_block_header, block):
        self.process_transactions(block)
        await self.analyze_censorship(block)
        self.latest_block_header = new_block_header
        print(f"Internal Transactions: {len(self.pending_transactions)}")

    def on_mempool(self, mempool):
        transactions_pending_in_mempool = mempool["pending"]
        persist_transactions = dict()

        for address, transactions in transactions_pending_in_mempool.items():
            min_nonce = min(transactions.keys())
            transaction = self.create_transaction(transactions[min_nonce])
            persist_transactions[transaction.hash] = transaction

        self.pending_transactions = persist_transactions

    def process_transactions(self, block):
        transactions = block["transactions"]
        for transaction_data in transactions:
            transaction = self.create_transaction(transaction_data)
            self.process_transaction(transaction)

    def process_transaction(self, transaction):
        ignore_transactions = list()
        self.nonces[transaction.sender] = transaction.nonce

        for internal_transaction in self.pending_transactions.values():
            if internal_transaction.sender == transaction.sender:
                ignore_transactions.append(internal_transaction)

        for ignore_transaction in ignore_transactions:
            self.pending_transactions.pop(ignore_transaction.hash)

    async def analyze_censorship(self, block):
        pending_transactions = deepcopy(list(self.pending_transactions.values()))

        for transaction in pending_transactions:
            if hex_to_int(self.latest_block_header["timestamp"]) < transaction.timestamp:
                continue
            lowest_priority_fee = _find_lowest_priority_fee(block)
            max_priority_fee = _get_max_priority_fee(transaction, hex_to_int(block["baseFeePerGas"]))

            if max_priority_fee < lowest_priority_fee:
                continue

            if isinstance(transaction, TransactionType2):
                max_base_fee = transaction.max_fee_per_gas - transaction.max_priority_fee_per_gas
            else:
                max_base_fee = transaction.gas_price

            if int(hex_to_int(block["baseFeePerGas"]) * 1.5) > max_base_fee:
                continue

            if transaction.gas > hex_to_int(block["gasLimit"]) - hex_to_int(block["gasUsed"]):
                continue

            address = transaction.sender
            if address not in self.nonces:
                nonce = await self.rpc_client.get_transaction_count(address)
                self.nonces[address] = nonce

            if self.nonces[address] == transaction.nonce:
                print(f"CENSORED TRANSACTION FOUND: {transaction.hash}, seen: {transaction.timestamp}")
                transaction.censored_blocks.append(block["number"])


def _get_max_priority_fee(transaction, base_fee_per_gas):
    if isinstance(transaction, TransactionType2):
        return transaction.max_priority_fee_per_gas

    else:
        gas_price = transaction.gas_price
        return gas_price - base_fee_per_gas



def _find_lowest_priority_fee(block):
    base_fee_per_gas = hex_to_int(block["baseFeePerGas"])
    lowest_priority_fee = 0
    for transaction in block["transactions"]:
        priority_fee = _get_max_priority_fee(transaction, base_fee_per_gas)
        lowest_priority_fee = min(lowest_priority_fee, priority_fee)

    return lowest_priority_fee


def _camel_to_snake(s: str) -> str:
    return "".join("_" + c.lower() if c.isupper() else c for c in s).lstrip("_")


class TransactionType0:

    def __init__(self, tx_hash, sender, gas, gas_price, nonce, timestamp, *args, **kwargs):
        self.hash = tx_hash
        self.sender = sender
        self.gas = gas
        self.gas_price = gas_price
        self.nonce = nonce
        self.timestamp = timestamp
        self.censored_blocks = list()

    @classmethod
    def from_dict(cls,data, timestamp=None):
        if timestamp is None:
            timestamp = int(time.time())
        data["sender"] = data["from"]
        data["tx_hash"] = data["hash"]
        data = {_camel_to_snake(name): value for name, value in data.items()}
        return cls(**data, timestamp=timestamp)


class TransactionType2(TransactionType0):
    def __init__(self, max_fee_per_gas, max_priority_fee_per_gas, timestamp, *args, **kwargs):
        super(TransactionType2, self).__init__(timestamp=timestamp, *args, **kwargs)
        self.max_fee_per_gas = max_fee_per_gas
        self.max_priority_fee_per_gas = max_priority_fee_per_gas
