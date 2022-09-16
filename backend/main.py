from db.database import Database

SCHEMA_PATH = "./backend/db/data/schema.sql"
DB_PATH = "./backend/db/data/ethereum-censorship-monitor.db"


def main():

    database = Database(DB_PATH, SCHEMA_PATH, allow_create=True)
    database.insert("blocks", {"block_number": 1, "validator": "0xtest"})


if __name__ == "__main__":
    main()
