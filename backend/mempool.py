import json
import threading
import time

from web3 import Web3
from web3.geth import GethTxPool


class Mempool:

    def __init__(self, web3: Web3, fetch_interval: int):
        self.web3 = web3
        self.fetch_interval = fetch_interval
        self._thread = None
        self.transactions = dict()
        self._mempool_results = list()

    def start(self):
        self._thread = threading.Thread(name="Mempool", target=self.fetch_mempool)
        self._thread.start()

    def stop(self):
        assert self._thread is not None
        self._thread.stop()

    def fetch_mempool(self):
        while True:
            content = self._fetch()
            current_timestamp = time.time()
            self._mempool_results.append((content, current_timestamp))
            time.sleep(self.fetch_interval)

    def process_mempool_results(self):
        if not self._mempool_results:
            return

        content, timestamp = self._mempool_results.pop(0)

        pending_transactions = content["pending"]

        for transactions_per_address in pending_transactions.values():
            lowest_nonce = min(transactions_per_address.keys())
            transaction = transactions_per_address[lowest_nonce]
            if transaction["hash"] not in self.transactions:
                self.transactions[transaction["hash"]] = (transaction, timestamp)

    def _fetch(self):
        mem_pool: GethTxPool = getattr(self.web3.geth, "txpool", None)
        return json.loads(self.web3.toJSON(mem_pool.content()))

