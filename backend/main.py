import asyncio
import click
import api

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
@click.option("--rest-host", type=str, required=True, help="Host of the REST API")
@click.option("--rest-port", type=int, required=True, help="Port of the REST API")
def main(rpc, rest_host, rest_port):
    database = initialize_database()
    loop = asyncio.get_event_loop()
    run_monitor(rpc)
    loop.run_until_complete(api.serve(rest_host, rest_port, database))
    loop.run_forever()


def run_monitor(rpc, database):
    chain = Chain()
    rpc_client = RPCClient(rpc, "ws://1.geth.mainnet.ethnodes.brainbot.com:8546", chain)
    rpc_client.start()


if __name__ == "__main__":
    main()
