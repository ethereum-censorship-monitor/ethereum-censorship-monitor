import asyncio
from aiohttp import web
import aiohttp_cors
import time

async def handle_stats(request):
    current_timestamp = int(time.time())
    cutoff_timestamp = current_timestamp - 24 * 60 * 60
    num_transactions_query = f"SELECT count(*) FROM transactions WHERE first_seen > {cutoff_timestamp}"
    num_blocks_query = f"SELECT count(*) FROM blocks WHERE timestamp > {cutoff_timestamp}"
    num_validators_query = f"SELECT count(DISTINCT validator) FROM blocks WHERE timestamp > {cutoff_timestamp}"

    db = request.app["database"]
    num_blocks = db.select(num_blocks_query)[0]["count(*)"]
    num_transactions = db.select(num_transactions_query)[0]["count(*)"]
    num_validators = db.select(num_validators_query)[0]["count(DISTINCT validator)"]
    data = {
        "num_blocks": num_blocks,
        "num_transactions": num_transactions,
        "num_validators": num_validators,
    }
    return web.json_response(data)

async def handle_blocks(request):
    query = "SELECT * FROM blocks ORDER BY block_number DESC LIMIT 100"
    data = request.app["database"].select(query)
    return web.json_response(data)

async def handle_transactions(request):
    query = "SELECT * FROM transactions ORDER BY first_seen DESC LIMIT 100"
    data = request.app["database"].select(query)
    return web.json_response(data)

async def handle_validators(request):
    query = "SELECT * FROM blocks ORDER BY block_number DESC LIMIT 100"
    blocks = request.app["database"].select(query)
    validators = {}
    for block in blocks:
        if block["validator"] in validators:
            continue
        validators[block["validator"]] = {
            "validator": block["validator"],
            "last_censored_block": block["hash"],
        }
    data = list(validators.values())
    return web.json_response(data)

app = web.Application()
cors = aiohttp_cors.setup(app, defaults={"*": aiohttp_cors.ResourceOptions(
    expose_headers="*",
    allow_headers="*",
    max_age=3600,
)})
routes = [
    web.get("/v1/stats", handle_stats),
    web.get("/v1/blocks", handle_blocks),
    web.get("/v1/transactions", handle_transactions),
    web.get("/v1/validators", handle_validators),
]
app.add_routes(routes)
for route in app.router.routes():
    cors.add(route)

async def serve(host, port, database):
    app["database"] = database
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, host, port)
    await site.start()
    while True:
        await asyncio.sleep(3600)
