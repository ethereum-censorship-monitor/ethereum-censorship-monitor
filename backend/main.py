import asyncio
import click

from rpc_client import RPCClient
from transaction import Chain
from db.database import Database

SCHEMA_PATH = "./backend/db/data/schema.sql"
DB_PATH = "./backend/db/data/ethereum-censorship-monitor.db"


def initialize_database():
    database = Database(DB_PATH, SCHEMA_PATH, allow_create=True)
    return database


@click.command()
@click.option("--rpc", type=str, required=True, help="Ethereum RPC endpoint. Required to be a GETH client.")
def main(rpc):
    loop = asyncio.get_event_loop()
    run_monitor(rpc)
    loop.run_forever()


def run_monitor(rpc):
    database = initialize_database()
    loop = asyncio.get_event_loop()
    chain = Chain()
    rpc_client = RPCClient(rpc, "ws://1.geth.mainnet.ethnodes.brainbot.com:8546", chain)
    rpc_client.start()


if __name__ == "__main__":
    main()
