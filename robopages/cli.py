import pathlib
import typing as t

import typer
from rich import box, print
from rich.prompt import Prompt
from rich.table import Table

from robopages.defaults import (
    DEFAULT_ADDRESS,
    DEFAULT_EXTENSION,
    DEFAULT_PAGE_FILE_NAME,
    DEFAULT_PATH,
    DEFAULT_PORT,
)
from robopages.models import Robook, Robopage
import robopages.api as api

cli = typer.Typer(no_args_is_help=True, help="Man pages but for robots!")


@cli.command(help="Create a new robopage file.")
def create(
    path: t.Annotated[
        pathlib.Path,
        typer.Argument(
            help="File name.",
            file_okay=True,
            resolve_path=True,
        ),
    ] = DEFAULT_PAGE_FILE_NAME,
) -> None:
    if path.exists():
        if (
            Prompt.ask(f":axe: overwrite {path.name}?", choices=["y", "n"], default="n")
            == "n"
        ):
            return

    Robopage.create_example_in_path(path)


@cli.command(help="View robopages.")
def view(
    path: t.Annotated[
        pathlib.Path,
        typer.Argument(
            help="Base path to search for robopages.",
            file_okay=True,
            resolve_path=True,
        ),
    ] = DEFAULT_PATH,
    filter: t.Annotated[
        str | None,
        typer.Option(
            "--filter",
            "-f",
            help="Filter results by this string.",
        ),
    ] = None,
) -> None:
    book = Robook.from_path(path, filter)

    print()

    table = Table(box=box.ROUNDED)
    table.add_column("categories")
    table.add_column("page")
    table.add_column("function")
    table.add_column("description")

    for page in book.pages.values():
        first_page = True
        for function_name, function in page.functions.items():
            if first_page:
                first_page = False
                table.add_row(
                    " > ".join(page.categories),
                    page.name,
                    function.to_string(function_name),
                    function.description,
                )
            else:
                table.add_row(
                    "", "", function.to_string(function_name), function.description
                )

    print(table)


@cli.command(
    help="Print an OpenAI / OLLAMA compatible JSON schema for tool calling from the robopages."
)
def to_json(
    path: t.Annotated[
        pathlib.Path,
        typer.Option(
            "--path",
            "-p",
            help="Robopage or directory containing multiple robopages.",
            file_okay=True,
            resolve_path=True,
        ),
    ] = DEFAULT_PATH,
    output: t.Annotated[
        pathlib.Path,
        typer.Option(
            "--output",
            "-o",
            help="Output file.",
            file_okay=True,
            resolve_path=True,
        ),
    ]
    | None = None,
    filter: t.Annotated[
        str,
        typer.Option(
            "--filter",
            "-f",
            help="Filter by this string.",
        ),
    ]
    | None = None,
) -> None:
    import json

    data = json.dumps(Robook.from_path(path, filter).to_openai(), indent=2)
    if output:
        output.write_text(data)
        print(f":file_folder: saved to {output}")
    else:
        print(data)


@cli.command(help="Serve the robopages as a local API.")
def serve(
    path: t.Annotated[
        pathlib.Path,
        typer.Option(
            "--path",
            "-p",
            help="Robopage or directory containing multiple robopages.",
            file_okay=True,
            resolve_path=True,
        ),
    ] = DEFAULT_PATH,
    filter: t.Annotated[
        str,
        typer.Option(
            "--filter",
            "-f",
            help="Filter by this string.",
        ),
    ]
    | None = None,
    address: t.Annotated[
        str,
        typer.Option(
            "--address",
            "-a",
            help="Address to bind to.",
        ),
    ] = DEFAULT_ADDRESS,
    port: t.Annotated[
        int,
        typer.Option(
            "--port",
            "-p",
            help="Port to bind to.",
        ),
    ] = DEFAULT_PORT,
) -> None:
    import uvicorn

    api.book = Robook.from_path(path, filter)

    uvicorn.run(api.app, host=address, port=port)
