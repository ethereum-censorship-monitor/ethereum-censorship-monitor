import json
import time
import asyncio
import click
import aiohttp
from web3 import Web3, HTTPProvider

from rpc_client import RPCClient
from transaction import Chain
from db.database import Database
from websockets import connect
SCHEMA_PATH = "./backend/db/data/schema.sql"
DB_PATH = "./backend/db/data/ethereum-censorship-monitor.db"


def initialize_database():
    database = Database(DB_PATH, SCHEMA_PATH, allow_create=True)
    return database




@click.command()
@click.option("--rpc", type=str, required=True, help="Ethereum RPC endpoint. Required to be a GETH client.")
def main(rpc):
    loop = asyncio.get_event_loop()
    loop.run_until_complete(run_monitor(rpc))
    loop.run_forever()

async def run_monitor(rpc):
    web3 = Web3(HTTPProvider(rpc))
    database = initialize_database()
    loop = asyncio.get_event_loop()
    chain = Chain()
    rpc_client = RPCClient(rpc,
                           "ws://1.geth.mainnet.ethnodes.brainbot.com:8546",
                           chain.new_block,
                           chain.acknowledge_transaction,
                           chain.on_mempool)
    rpc_client.start()

    while True:
        await asyncio.sleep(10)


if __name__ == "__main__":
    main()
