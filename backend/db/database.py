from pathlib import Path
from typing import Dict, Any, Optional, List
import sqlite3
import os
import logging

log = logging.getLogger(__name__)

def populate_database(db):
    blocks = [
        {
            "block_number": 1000000,
            "validator": 329690,
            "hash": "0xa11f10fa987b8e410317285e479917d0d47c2e0c9ca7c85ccb341d672a474b8b",
            "timestamp": 1663453521,
        },
        {
            "block_number": 1001000,
            "validator": 123456,
            "hash": "0x0b51c8d0ba5dd9e9f9773a241f6a0c20bdb58d8e37424b40da6196d99859ea7e",
            "timestamp": 1663453302,
        },
        {
            "block_number": 15555098,
            "validator": 229690,
            "hash": "0x1106290f5fff61aa181e26d9fcd985fed8404d35de2d448adb428a4501628cb3",
            "timestamp": 1663453781,
        },
    ]
    transactions = [
        {
            "hash": "0x704ee73a7321961a12004b660ef943a1140079874b08d8f739658dc6c4b36241",
            "first_seen": 1663394442,
            "sender": "0x388c818ca8b9251b393131c08a736a67ccb19297",
        },
        {
            "hash": "0x66e184c04b58a073a5b15ffb4d5a77e66f20f484ec3071a72edabf70bbe4c030",
            "first_seen": 1663394402,
            "sender": "0xebec795c9c8bbd61ffc14a6662944748f299cacf",
        },
        {
            "hash": "0xbe4ee7bd5db427d3d213951c9b99eaa29b714dc161e3ca524816ac987b6874d5",
            "first_seen": 1663394442,
            "sender": "0x388c818ca8b9251b393131c08a736a67ccb19297",
        },
    ]
    for block in blocks:
        db.insert("blocks", block)
    for transaction in transactions:
        db.insert("transactions", transaction)

class Database:
    def __init__(self, filename: str, schema_filename: str, allow_create: bool = False):
        log.info(f"Opening database {filename}")

        os.makedirs(os.path.dirname(filename), exist_ok=True)
        if not os.path.exists(filename):
            path = Path(filename)
            path.touch()
            db_file_exists = False
        else:
            db_file_exists = True

        conn_args = {
            "uri": True,
            "detect_types": sqlite3.PARSE_DECLTYPES,
            "isolation_level": None
        }

        write_mode = "rwc" if allow_create else "rw"
        self.write_conn = sqlite3.connect(
            f"file:{filename}?mode={write_mode}",
            **conn_args,
        )
        self.read_conn = sqlite3.connect(
            f"file:{filename}?mode=ro",
            **conn_args,
        )
        self.write_conn.row_factory = sqlite3.Row
        self.read_conn.row_factory = sqlite3.Row
        self.write_conn.execute("PRAGMA foreign_keys = ON")

        if not db_file_exists:
            self.create_new_database(schema_filename)

    def create_new_database(self, schema_filename) -> None:
        with open(schema_filename) as schema_file:
            result = self.write_conn.executescript(schema_file.read())
        populate_database(self)

    def _execute(self, conn, command, content=None):
        if content is None:
            return conn.execute(command)
        else:
            return conn.execute(command, content)

    def _execute_write(self, command, content=None):
        return self._execute(self.write_conn, command, content)

    def _execute_read(self, command, content=None):
        return self._execute(self.read_conn, command, content)

    def insert(self, table_name: str, fields_by_colname: Dict[str, Any]) -> sqlite3.Cursor:
        cols = ", ".join(fields_by_colname.keys())
        values = ", ".join(":" + col_name for col_name in fields_by_colname)
        return self._execute_write(f"INSERT OR IGNORE INTO {table_name}({cols}) VALUES ({values})", fields_by_colname)

    def select(self, query: str) -> List[Dict[str, Any]]:
        cursor = self._execute_read(query)
        rows = cursor.fetchall()
        result = []
        for row in rows:
            result.append(dict(row))
        return result
