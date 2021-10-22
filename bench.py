import aiohttp
import httpx
from starlette.applications import Starlette
from starlette.responses import PlainTextResponse
from starlette.routing import Route

import hpyer

HOST, PORT = "localhost", 8000
URL = f"http://{HOST}:{PORT}/"


async def index(request):
    return PlainTextResponse("world")


async def aiohttp_single(request):
    async with aiohttp.ClientSession() as client:
        async with client.get(URL) as r:
            return _response(await r.text())


async def aiohttp_session(request):
    async with aiohttp_session.get(URL) as r:
        return _response(await r.text())


async def hpyer_single(request):
    client = hpyer.ClientSession()

    async with client.get(URL) as r:
        return _response(await r.text())


async def hpyer_session(request):
    async with hpyer_session.get(URL) as r:
        return _response(await r.text())


async def httpx_single(request):
    async with httpx.AsyncClient() as client:
        r = await client.get(URL)
        return _response(r.text)


async def httpx_session(request):
    r = await httpx_session.get(URL)
    return _response(r.text)


def _response(name):
    return PlainTextResponse("Hello, " + name)


routes = [
    Route("/", endpoint=index),
    Route("/aiohttp/single", endpoint=aiohttp_single),
    Route("/aiohttp/session", endpoint=aiohttp_session),
    Route("/hpyer/single", endpoint=hpyer_single),
    Route("/hpyer/session", endpoint=hpyer_session),
    Route("/httpx/single", endpoint=httpx_single),
    Route("/httpx/session", endpoint=httpx_session),
]


async def on_startup():
    global aiohttp_session, hpyer_session, httpx_session
    aiohttp_session = aiohttp.ClientSession()
    hpyer_session = hpyer.ClientSession()
    httpx_session = httpx.AsyncClient()


app = Starlette(debug=True, routes=routes, on_startup=[on_startup])


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host=HOST, port=PORT)
