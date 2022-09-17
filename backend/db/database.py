from pathlib import Path
from typing import Dict, Any, Optional, List
import sqlite3
import os
import logging

log = logging.getLogger(__name__)


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
        return self._execute_write(f"INSERT INTO {table_name}({cols}) VALUES ({values})", fields_by_colname)

    def select(self, table_name: str, limit: Optional[int] = None) -> List[Dict[str, Any]]:
        if limit is None:
            limit_part = ""
        else:
            limit_part = f"LIMIT {limit}"
        cursor = self._execute_read(f"SELECT * FROM {table_name}" + limit_part)
        rows = cursor.fetchall()
        result = []
        for row in rows:
            result.append(dict(row))
        return result
