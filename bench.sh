#!/usr/bin/bash
wrk http://localhost:8000/aiohttp/single
wrk http://localhost:8000/httpx/single
wrk http://localhost:8000/hpyer/single
wrk http://localhost:8000/aiohttp/session
wrk http://localhost:8000/httpx/session
wrk http://localhost:8000/hpyer/session
