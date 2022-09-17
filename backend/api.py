import asyncio
from aiohttp import web
import aiohttp_cors

async def handle_stats(request):
    data = {
        "numBlocks": 0,
        "numTransactions": 1,
        "numValidators": 2,
    }
    return web.json_response(data)

async def handle_blocks(request):
    data = [
        {"number": 15553478, "hash": "0xa11f10fa987b8e410317285e479917d0d47c2e0c9ca7c85ccb341d672a474b8b", "validator": 329690},
        {"number": 15553477, "hash": "0xc16b8f68655418f08edc8c2845dfc295a38878633e7063dee67eb36d3f8cfd94", "validator": 123},
        {"number": 15553476, "hash": "0xd00f6d59f935836b3cdde82c8bc2884ba41f6fe192081b608f3a4a84c00e6107", "validator": 456},
    ]
    return web.json_response(data)

async def handle_transactions(request):
    data = [
        {"timestamp": 15553478, "hash": "0xa11f10fa987b8e410317285e479917d0d47c2e0c9ca7c85ccb341d672a474b8b"},
        {"timestamp": 15553477, "hash": "0xc16b8f68655418f08edc8c2845dfc295a38878633e7063dee67eb36d3f8cfd94"},
        {"timestamp": 15553476, "hash": "0xd00f6d59f935836b3cdde82c8bc2884ba41f6fe192081b608f3a4a84c00e6107"},
    ]
    return web.json_response(data)

async def handle_validators(request):
    data = [
        {"validator": 329690, "lastCensoredBlock": "0xa11f10fa987b8e410317285e479917d0d47c2e0c9ca7c85ccb341d672a474b8b"},
        {"validator": 123, "lastCensoredBlock": "0xa11f10fa987b8e410317285e479917d0d47c2e0c9ca7c85ccb341d672a474b8b"},
        {"validator": 456, "lastCensoredBlock": "0xa11f10fa987b8e410317285e479917d0d47c2e0c9ca7c85ccb341d672a474b8b"},
    ]
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

async def serve(host, port):
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, host, port)
    await site.start()
    while True:
        await asyncio.sleep(3600)
