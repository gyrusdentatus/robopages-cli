import pathlib
import typing as t

import typer
from rich import box, print
from rich.prompt import Prompt
from rich.table import Table

from robopages.defaults import DEFAULT_EXTENSION, DEFAULT_PAGE_FILE_NAME, DEFAULT_PATH
from robopages.models import Robook, Robopage

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
    book = Robook.from_path(path)

    print()

    table = Table(box=box.ROUNDED)
    table.add_column("category")
    table.add_column("page")
    table.add_column("function")
    table.add_column("description")

    for page_path, page in book.pages.items():
        first_page = True
        for function_name, function in page.functions.items():
            if first_page:
                relative_parts = list(page_path.relative_to(path).parts)
                relative_parts[-1] = relative_parts[-1].removesuffix(
                    f".{DEFAULT_EXTENSION}"
                )
                category = (
                    " > ".join(relative_parts[:-1]) if len(relative_parts) > 1 else ""
                )
                page_name = relative_parts[-1] if relative_parts else ""
                first_page = False
                table.add_row(
                    category,
                    page_name,
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
def to_openai(
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
) -> None:
    import json

    data = json.dumps(Robook.from_path(path).to_openai(), indent=2)
    if output:
        output.write_text(data)
        print(f":file_folder: saved to {output}")
    else:
        print(data)
