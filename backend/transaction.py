import time
from copy import deepcopy

from util import hex_to_int


class Chain:

    def __init__(self):
        self.pending_transactions = dict()
        self.nonces = dict()
        self.timestamps = dict()
        self.latest_block = None
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

    async def new_block(self, block_data):
        block = Block.from_dict(block_data)
        self.process_transactions(block)
        await self.analyze_censorship(block)
        self.latest_block = block
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
        for transaction in block.transactions:
            self.process_transaction(transaction)

    def process_transaction(self, transaction):
        ignore_transactions = list()
        self.nonces[transaction.sender] = transaction.nonce

        for internal_transaction in self.pending_transactions.values():
            if transaction.sender == internal_transaction.sender:
                ignore_transactions.append(internal_transaction)

        for ignore_transaction in ignore_transactions:
            self.pending_transactions.pop(ignore_transaction.hash)

    async def analyze_censorship(self, block):
        pending_transactions = deepcopy(list(self.pending_transactions.values()))

        for transaction in pending_transactions:
            if self.latest_block.timestamp < transaction.timestamp:
                continue
            lowest_priority_fee = _find_lowest_priority_fee(block)
            max_priority_fee = _get_max_priority_fee(transaction, block.base_fee_per_gas)

            if max_priority_fee < lowest_priority_fee:
                continue

            if isinstance(transaction, TransactionType2):
                max_base_fee = transaction.max_fee_per_gas - transaction.max_priority_fee_per_gas
            else:
                max_base_fee = transaction.gas_price

            if int(block.base_fee_per_gas * 1.5) > max_base_fee:
                continue

            if transaction.gas > block.gas_limit - block.gas_used:
                continue

            address = transaction.sender
            if address not in self.nonces:
                nonce = await self.rpc_client.get_transaction_count(address)
                self.nonces[address] = nonce

            if self.nonces[address] == transaction.nonce:
                print(f"CENSORED TRANSACTION FOUND: {transaction.hash}, seen: {transaction.timestamp}")
                transaction.censored_blocks.append(block.number)


def _get_max_priority_fee(transaction, base_fee_per_gas):
    if isinstance(transaction, TransactionType2):
        return transaction.max_priority_fee_per_gas
    else:
        gas_price = transaction.gas_price
        return gas_price - base_fee_per_gas


def _find_lowest_priority_fee(block):
    base_fee_per_gas = block.base_fee_per_gas
    lowest_priority_fee = _get_max_priority_fee(block.transactions[0], base_fee_per_gas)
    for transaction in block.transactions:
        priority_fee = _get_max_priority_fee(transaction, base_fee_per_gas)
        lowest_priority_fee = min(lowest_priority_fee, priority_fee)

    return lowest_priority_fee


def _camel_to_snake(s: str) -> str:
    return "".join("_" + c.lower() if c.isupper() else c for c in s).lstrip("_")


class TransactionType0:

    def __init__(self, tx_hash, sender, gas, gas_price, nonce, timestamp, *args, **kwargs):
        self.hash = tx_hash
        self.sender = sender
        self.gas = hex_to_int(gas)
        self.gas_price = hex_to_int(gas_price)
        self.nonce = hex_to_int(nonce)
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
        self.max_fee_per_gas = hex_to_int(max_fee_per_gas)
        self.max_priority_fee_per_gas = hex_to_int(max_priority_fee_per_gas)


class Block:
    def __init__(self, number, base_fee_per_gas, gas_limit, gas_used, timestamp, transactions, *args, **kwargs):
        self.number = hex_to_int(number)
        self.base_fee_per_gas = hex_to_int(base_fee_per_gas)
        self.gas_limit = hex_to_int(gas_limit)
        self.gas_used = hex_to_int(gas_used)
        self.timestamp = hex_to_int(timestamp)
        self.transactions = [
            TransactionType2.from_dict(transaction_data, self.timestamp) if hex_to_int(transaction_data["type"]) == 2 else
            TransactionType0.from_dict(transaction_data, self.timestamp)
            for transaction_data in transactions
        ]

    @classmethod
    def from_dict(cls, data):
        data = {_camel_to_snake(name): value for name, value in data.items()}
        return cls(**data)