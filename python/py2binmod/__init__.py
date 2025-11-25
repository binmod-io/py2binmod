import asyncio
import sys

from py2binmod._py2binmod import __version__  # noqa: F401


try:
    import uvloop  # type: ignore
    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
except (ImportError):
    if sys.version_info >= (3, 8) and sys.platform == "win32":
        asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())
