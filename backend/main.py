import time

import click
from web3 import Web3, HTTPProvider

from mempool import Mempool
from db.database import Database

SCHEMA_PATH = "./backend/db/data/schema.sql"
DB_PATH = "./backend/db/data/ethereum-censorship-monitor.db"


def initialize_database():
    database = Database(DB_PATH, SCHEMA_PATH, allow_create=True)
    
@click.command()
@click.option("--rpc", type=str, required=True, help="Ethereum RPC endpoint. Required to be a GETH client.")
def main(rpc):
    web3 = Web3(HTTPProvider(rpc))
    mempool = Mempool(web3, 5)
    mempool.start()

    while True:
        mempool.process_mempool_results()
        from pprint import pprint
        pprint(mempool.transactions)
        time.sleep(5)


if __name__ == "__main__":
    main()
