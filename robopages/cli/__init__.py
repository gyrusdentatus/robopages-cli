import pathlib
import typing as t

import typer
from rich import print
from rich.prompt import Prompt

from robopages.defaults import DEFAULT_PAGE_FILE_NAME, DEFAULT_PATH
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
