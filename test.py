# Just some quick debug demo
import asyncio
import aiohttp
import hpyer

async def main():
    session = aiohttp.ClientSession()
    response = await session.get("https://python.org")
    print(response.version)
    response.close()
    await session.close()
    session = hpyer.ClientSession()
    response = await session.get("https://python.org")
    print(response.version)

asyncio.run(main())
