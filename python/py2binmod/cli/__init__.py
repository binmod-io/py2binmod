from typing import Annotated, Optional

import typer

from py2binmod.cli.commands import build_cli, transpile_cli
from py2binmod.cli.utils import AsyncCLI, apply_decorators, catch_exceptions, show_version


cli = AsyncCLI(
    name="py2binmod",
    help="A tool to convert Python modules into Binmod modules.",
)

apply_decorators(build_cli, catch_exceptions(), cli.command(name="build"))
apply_decorators(transpile_cli, catch_exceptions(), cli.command(name="transpile"))

@cli.callback()
@catch_exceptions()
async def entrypoint(
    typer_context: typer.Context,
    version: Annotated[Optional[bool], typer.Option(
        "--version",
        "-v",
        help="Show the version and exit.",
        callback=show_version,
        is_eager=True,
    )] = None,
):
    """
    A tool to convert Python modules into Binmod modules.
    """
    ...
