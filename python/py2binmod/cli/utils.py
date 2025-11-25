import asyncio
import os
from collections.abc import Callable, Coroutine
from functools import wraps
from inspect import iscoroutinefunction
from typing import Any, ParamSpec, TypeVar, cast

import typer
from rich import print as pprint

from py2binmod import __version__


P = ParamSpec("P")
R = TypeVar("R")

DEV_MODE = os.getenv("DEV_MODE", "false") == "true"


class AsyncCLI(typer.Typer):
    def command(self, *args, **kwargs):
        decorator = super().command(*args, **kwargs)

        def wrapper(fn):
            if iscoroutinefunction(fn):
                fn = to_sync(fn)
            return apply_decorators(fn, decorator)
        return wrapper

    def callback(self, *args, **kwargs):
        decorator = super().callback(*args, **kwargs)

        def wrapper(fn):
            if iscoroutinefunction(fn):
                fn = to_sync(fn)
            return apply_decorators(fn, decorator)
        return wrapper


def show_version(show: bool):
    if show:
        print("py2binmod version:", __version__)
        raise typer.Exit()


def to_sync(func: Callable[P, Coroutine[Any, Any, R]]) -> Callable[P, R]:
    """
    Convert an async function to a sync function.

    :param func: async function to convert
    :return: sync function
    """
    if not iscoroutinefunction(func):
        return cast(Callable[P, R], func)

    @wraps(func)
    def wrapper(*args: P.args, **kwargs: P.kwargs) -> R:
        return asyncio.run(func(*args, **kwargs))
    return wrapper


def apply_decorators(func: Callable, *decorators: Callable) -> Callable:
    """
    Apply a list of decorators to a function

    :param func: The function to apply the decorators to
    :type func: Callable
    :param decorators: The decorators to apply
    :type decorators: list[Callable]
    """
    for decorator in decorators:
        func = decorator(func)
    return func


def catch_exceptions(
    exceptions: tuple[type[BaseException]] | type[BaseException] = BaseException,
):
    """
    Catch an exception and exit with the given code.

    :param exceptions: The exceptions to catch
    :type exceptions: tuple[type[BaseException]] | type[BaseException]
    :raises typer.Exit: If the exception is caught
    """
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            try:
                return func(*args, **kwargs)
            except typer.Exit:
                pass
            except exceptions as e:
                if not DEV_MODE:
                    handle_error(e)
                else:
                    raise e

        @wraps(func)
        async def async_wrapper(*args, **kwargs):
            try:
                return await func(*args, **kwargs)
            except typer.Exit:
                pass
            except exceptions as e:
                if not DEV_MODE:
                    handle_error(e)
                else:
                    raise e

        if iscoroutinefunction(func):
            return async_wrapper
        return wrapper
    return decorator

def handle_error(e: BaseException):
    """
    Handle an error by rendering it to the error panel
    and exiting with the given code.
    """
    exit_with_code(
        getattr(e, "status", None) or 1
    )


def exit_with_code(code: int):
    """
    Exit the application with the given code

    :param code: The exit code to use
    :type code: int
    """
    raise typer.Exit(code)


def exit_with_error(message: str, name: str, code: int = 1):
    """
    Exit the application with an error message and status code.

    :param message: The error message to display
    :type message: str
    :param name: The error name to display
    :type name: str
    :param code: The exit code to use
    :type code: int
    """
    pprint(f"[red]{name}[/red]: {message}")
    exit_with_code(code)
