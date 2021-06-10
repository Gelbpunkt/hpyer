# Just some quick debug demo
import asyncio
import hpyer
import time

async def main():
    session = hpyer.ClientSession()

    #async with session.post("https://httpbin.org/anything", json={"hello": "world", "please": {"not": "you"}}, params={"ay": "b"}, headers={"Authorization": "yolo"}) as req:
    #    print(await req.json())

    req = await session.post("https://google.com")
    print(req.status)
    

asyncio.run(main())
