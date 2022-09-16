from pathlib import Path
from typing import Dict, Any
import sqlite3
import os
import logging

log = logging.getLogger(__name__)


class Database:
    def __init__(self, filename: str, schema_filename: str, allow_create: bool = False):
        log.info(f"Opening database {filename}")
        db_file_exists = False
        if filename == ":memory:":
            self.conn = sqlite3.connect(
                ":memory:",
                detect_types=sqlite3.PARSE_DECLTYPES,
                isolation_level=None
            )
        else:

            os.makedirs(os.path.dirname(filename), exist_ok=True)
            if not os.path.exists(filename):
                path = Path(filename)
                path.touch()
            else:
                db_file_exists = True
            mode = "rwc" if allow_create else "rw"
            self.conn = sqlite3.connect(
                f"file:{filename}?mode={mode}",
                uri=True,
                detect_types=sqlite3.PARSE_DECLTYPES,
                isolation_level=None
                )
        self.conn.row_factory = sqlite3.Row
        self.conn.execute("PRAGMA foreign_keys = ON")
        if not db_file_exists:
            self.create_new_database(schema_filename)

    def create_new_database(self, schema_filename) -> None:
        with open(schema_filename) as schema_file:
            result = self.conn.executescript(schema_file.read())

    def _execute(self, command, content=None):
        if content is None:
            self.conn.execute(command)
        else:
            self.conn.execute(command, content)

    def insert(self, table_name: str, fields_by_colname: Dict[str, Any]) -> sqlite3.Cursor:
        cols = ", ".join(fields_by_colname.keys())
        values = ", ".join(":" + col_name for col_name in fields_by_colname)
        return self._execute(f"INSERT INTO {table_name}({cols}) VALUES ({values})", fields_by_colname)