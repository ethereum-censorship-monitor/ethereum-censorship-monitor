import asyncio
import json
import time

from aiohttp import ClientSession
from websockets import connect


class RPCClient:

    def __init__(self, rpc, ws_url, on_new_block, on_new_transaction, on_mempool):
        self.rpc = rpc
        self.ws_url = ws_url
        self.on_new_block = on_new_block
        self.on_new_transaction = on_new_transaction
        self.on_mempool = on_mempool
        self.session = ClientSession()
        self.new_block_event = asyncio.Event()




    async def fetch_new_heads(self):
        async with connect(self.ws_url) as ws:
            await ws.send(json.dumps({"id": 1, "method": "eth_subscribe", "params": ["newHeads"]}))
            await ws.recv()

            while True:
                response = json.loads(await ws.recv())
                header = response["params"]["result"]
                block = await self.get_block(header["number"], True)

                self.new_block_event.set()
                self.on_new_block(header, block)

    async def fetch_new_transactions(self):
        async with connect(self.ws_url) as ws:
            await ws.send(json.dumps({"id": 1, "method": "eth_subscribe", "params": ["newPendingTransactions"]}))
            await ws.recv()

            while True:
                response = json.loads(await ws.recv())
                self.on_new_transaction(response["params"]["result"], time.time())

    async def get_block(self, number, transaction_content=False):
        result = None
        while result is None:
            data = {"jsonrpc": "2.0", "method": "eth_getBlockByNumber", "params": [number, transaction_content], "id": 1}
            response = await self.session.post(self.rpc, json=data)
            result = (await response.json())["result"]
            if result is None:
                print(f"getBlock({int(number, 0)}) failed")
        return result

    async def fetch_mempool(self):
        while True:
            await self.new_block_event.wait()
            await asyncio.sleep(6)
            data = {"jsonrpc": "2.0", "method": "txpool_content", "id": 1}
            result = await self.session.post(self.rpc, json=data)
            mempool = (await result.json())["result"]
            counter = 0
            for transactions in mempool["pending"].values():
                counter += len(transactions)
            print(f"Mempool size: {counter}")
            self.on_mempool(mempool)

    def start(self):
        loop = asyncio.get_event_loop()
        loop.create_task(self.fetch_new_heads())
        loop.create_task(self.fetch_new_transactions())
        loop.create_task(self.fetch_mempool())
