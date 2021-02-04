# Just some quick debug demo
import asyncio
import hpyer

async def main():
    session = hpyer.ClientSession()
    response = await session.get("https://python.org")
    print(await response.read())

asyncio.run(main())
