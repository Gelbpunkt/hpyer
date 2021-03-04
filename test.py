# Just some quick debug demo
import asyncio
import aiohttp
import hpyer
import time

import faulthandler
faulthandler.enable()

async def main():
    session = hpyer.ClientSession()
    response = await session.post("https://httpbin.org/anything", json={"hello": "world", "please": {"not": "you"}}, params={"ay": "b"})
    body = await response.json()
    print(body)

asyncio.run(main())
