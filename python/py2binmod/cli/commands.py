from pathlib import Path
from typing import Annotated

import typer

from py2binmod.core import build_command, transpile_command


async def transpile_cli(
    typer_context: typer.Context,
    project_dir: Annotated[Path, typer.Argument(
        help="Path to the Python project directory.",
    )] = Path.cwd(),
    out_dir: Annotated[Path | None, typer.Option(
        "--out-dir",
        "-o",
        help="Directory to write the transpiled Binmod module source code to.",
    )] = None,
    stdout: Annotated[bool, typer.Option(
        "--stdout",
        help="If set, print the generated Binmod module source code to stdout.",
        is_flag=True,
    )] = False,
) -> None:
    """
    Transpile a Binmod module from a Python project directory.
    """
    await transpile_command(
        project_dir=str(project_dir.resolve().absolute()),
        out_dir=str(
            out_dir.resolve().absolute()
            if out_dir
            else project_dir.joinpath("artifacts").resolve().absolute()
        ),
        stdout=stdout,
    )


async def build_cli(
    typer_context: typer.Context,
    project_dir: Annotated[Path, typer.Argument(
        help="Path to the Python project directory.",
    )] = Path.cwd(),
    out_dir: Annotated[Path | None, typer.Option(
        "--out-dir",
        "-o",
        help="Directory to write the compiled Binmod module to.",
    )] = None,
    release: Annotated[bool, typer.Option(
        "--release",
        help="Build the Binmod module in release mode.",
        is_flag=True,
    )] = False,
) -> None:
    """
    Build a Binmod module from a Python project directory.
    """
    await build_command(
        project_dir=str(project_dir.resolve().absolute()),
        out_dir=str(
            out_dir.resolve().absolute()
            if out_dir
            else project_dir.joinpath("artifacts").resolve().absolute()
        ),
        release=release,
    )
