from aiohttp import web

async def handle(request):
    print(request)
    return web.Response(text="hi")

app = web.Application()
app.add_routes([
    web.get("/", handle),
])

async def serve(host, port):
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, host, port)
    await site.start()
