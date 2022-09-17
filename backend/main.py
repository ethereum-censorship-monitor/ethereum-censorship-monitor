import json
import time
import asyncio
import click
import aiohttp
from web3 import Web3, HTTPProvider

from transaction import Chain
from mempool import Mempool
from db.database import Database
from websockets import connect
import api
SCHEMA_PATH = "./backend/db/data/schema.sql"
DB_PATH = "./backend/db/data/ethereum-censorship-monitor.db"


def initialize_database():
    database = Database(DB_PATH, SCHEMA_PATH, allow_create=True)
    return database


async def fetch_new_heads(rpc,on_new_block):
    async with connect("wss://mainnet.infura.io/ws/v3/9d7e06efd1f04cc9b6c9b36e60cdd80a") as ws:
        await ws.send(json.dumps({"id": 1, "method": "eth_subscribe", "params": ["newHeads"]}))
        subscription_response = await ws.recv()
        print(subscription_response)
        # Infinite loop waiting for WebSocket data
        while True:
            response = await ws.recv()
            header = response["result"]["blockNumber"]
            session = aiohttp.ClientSession()
            result = await session.put(rpc, data={"jsonrpc":"2.0","method":"eth_getBlockByHash","params": [header,True],"id":1})
            on_new_block(result)
async def fetch_new_transactions(on_new_transaction):
    async with connect("wss://mainnet.infura.io/ws/v3/9d7e06efd1f04cc9b6c9b36e60cdd80a") as ws:
        await ws.send(json.dumps({"id": 1, "method": "eth_subscribe", "params": ["newPendingTransactions"]}))
        subscription_response = await ws.recv()
        print(subscription_response)
        # Infinite loop waiting for WebSocket data
        while True:
            result = await ws.recv()
            on_new_transaction(result["result"])

@click.command()
@click.option("--rpc", type=str, required=True, help="Ethereum RPC endpoint. Required to be a GETH client.")
@click.option("--rest-host", type=str, required=True, help="Host of the REST API")
@click.option("--rest-port", type=int, required=True, help="Port of the REST API")
def main(rpc, rest_host, rest_port):
    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        run_monitor(rpc),
        api.serve(rest_host, rest_port),
    ))

async def run_monitor(rpc):
    web3 = Web3(HTTPProvider(rpc))
    mempool = Mempool(web3, 5)
    database = initialize_database()
    loop = asyncio.get_event_loop()
    session = aiohttp.ClientSession()
    data = {"jsonrpc": "2.0", "method": "eth_getBlockByNumber", "params": ["latest", True],"id": 1}
    result = await session.post(rpc, json=data)

    print(await result.json())
    await session.close()
    return
    transactions = Chain()

    loop.create_task(fetch_new_transactions(transactions.add_transaction))
    loop.create_task(fetch_new_heads(rpc, transactions.new_block))

    loop.run_forever()

    while True:
        transaction_hashes_before = len(mempool.transactions.keys())
        mempool.process_mempool_results()
        transaction_hashes_after = len(mempool.transactions.keys())
        from pprint import pprint
        pprint(f"New transactions: {transaction_hashes_after - transaction_hashes_before}")
        time.sleep(5)


if __name__ == "__main__":
    main()
