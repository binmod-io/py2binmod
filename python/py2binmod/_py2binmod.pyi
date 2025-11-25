async def transpile_command(
    project_dir: str,
    out_dir: str | None = None,
    stdout: bool = False,
) -> None:
    ...


async def build_command(
    project_dir: str,
    out_dir: str | None = None,
    release: bool = False,
) -> None:
    ...
