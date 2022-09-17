class Chain:

    def __init__(self, latest_block_header):
        self.pending_transactions = dict()
        self.latest_block_header = latest_block_header

    def add_transaction(self, hash, timestamp):
        self.pending_transactions[hash] = Transaction(timestamp)

    def new_block(self, new_block_header, block):
        self.latest_block_header = new_block_header
        from pprint import pprint
        pprint(block)




class Transaction:

    def __init__(self, first_seen):
        self.first_seen = first_seen
        self.data = None


    def add_data(self, data):
        self.data = data

