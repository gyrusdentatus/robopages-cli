import pathlib
import os
import subprocess

from pydantic import BaseModel
from pydantic_yaml import parse_yaml_raw_as, to_yaml_str
from rich import print
from rich.prompt import Prompt

from robopages.defaults import DEFAULT_EXTENSION, DEFAULT_PATH, DEFAULT_PATH_ENV_VAR


class Parameter(BaseModel):
    type: str  # TODO: make this a enum
    description: str
    required: bool = True
    examples: list[str] | None = None


class Function(BaseModel):
    description: str | None = None
    parameters: dict[str, Parameter] = {}
    # if command
    cmdline: list[str] | None = None

    def execute(self, arguments: dict[str, str]) -> str:
        if not self.cmdline:
            raise Exception("No command line to execute")

        cmdline = []
        for arg in self.cmdline:
            if arg.startswith("${"):
                arg_name = arg[2:-1]
                if arg_name in arguments:
                    cmdline.append(arguments[arg_name])
                else:
                    raise Exception(f"Argument {arg_name} not found")
            else:
                cmdline.append(arg)

        res = subprocess.run(cmdline, capture_output=True, text=True)
        err = res.stderr.strip()
        out = res.stdout.strip()
        return f"{err}\n{out}" if err else out


class Robopage(BaseModel):
    name: str | None = None
    description: str | None = None
    functions: dict[str, Function]

    @staticmethod
    def create_example_in_path(path: pathlib.Path) -> None:
        yml = to_yaml_str(
            Robopage(
                description="You can use this for a description.",
                functions={
                    "example_function_name": Function(
                        description="This is an example function describing a command line.",
                        parameters={
                            "foo": Parameter(
                                type="string",
                                description="An example paramter named foo.",
                                examples=["bar", "baz"],
                            ),
                        },
                        cmdline=["echo", "${foo}"],
                    )
                },
            ),
            default_flow_style=False,
        )

        path.write_text(yml)

        print(f":brain: created {path}")

    def to_openai(self) -> list[dict]:
        # https://platform.openai.com/docs/guides/function-calling
        tools = []

        for func_name, func in self.functions.items():
            parameters = {}
            required = []
            for param_name, param in func.parameters.items():
                parameters[param_name] = {
                    "type": param.type,
                    "description": param.description,
                }
                if param.required:
                    required.append(param_name)

            tools.append(
                {
                    "type": "function",
                    "function": {
                        "name": func_name,
                        "description": func.description,
                        "parameters": {
                            "type": "object",
                            "properties": parameters,
                            "required": required,
                        },
                    },
                }
            )

        return tools


class FunctionCall(BaseModel):
    name: str
    arguments: dict[str, str]


class Robocall(BaseModel):
    function: FunctionCall


class Robook(BaseModel):
    pages: dict[pathlib.Path, Robopage]

    def to_openai(self) -> list[dict]:
        """Convert all robopages to OpenAI compatible tools (https://platform.openai.com/docs/guides/function-calling)."""

        return [tool for page in self.pages.values() for tool in page.to_openai()]

    def to_ollama(self) -> list[dict]:
        """Convert all robopages to OLLAMA compatible tools (https://python.langchain.com/v0.1/docs/integrations/chat/ollama_functions/)."""

        return self.to_openai()

    def process(self, calls: list[dict], interactive: bool = True) -> dict[str, str]:
        """Process a list of tool calls from an LLM and return the result for each."""

        results = {}

        for call in [Robocall(**call) for call in calls]:
            output = None
            for page in self.pages.values():
                if call.function.name in page.functions:
                    print(
                        f":robot: executing [bold]{page.name}[/].{call.function.name}({', '.join(call.function.arguments.values())})"
                    )

                    func = page.functions[call.function.name]
                    if (
                        not interactive
                        or Prompt.ask(
                            ":eyes: execute?",
                            choices=["y", "n"],
                            default="n",
                        )
                        == "y"
                    ):
                        output = func.execute(call.function.arguments)
                    else:
                        output = "<not executed>"

                    break

            if output is None:
                raise Exception(f"Function {call.function.name} not found")
            else:
                results[call.function.name] = output

        return results

    @staticmethod
    def load() -> "Robook":
        """Load all robopages from the environment variable ROBOPAGES_PATH if set or the default path."""

        return Robook.from_path(
            pathlib.Path(os.getenv(DEFAULT_PATH_ENV_VAR) or DEFAULT_PATH)
        )

    @staticmethod
    def from_path(path: pathlib.Path = DEFAULT_PATH) -> "Robook":
        """Load all robopages from the given file or directory."""

        robopaths: list[pathlib.Path] = []
        robopages: dict[pathlib.Path, Robopage] = {}

        if path.is_file():
            robopaths.append(path)
        elif path.is_dir():
            robopaths.extend(path.rglob(f"*.{DEFAULT_EXTENSION}"))

        if not robopaths:
            raise Exception(f"No robopages found in {path}")

        print(f":robot: loading from {path}")

        num_functions = 0
        for robopath in robopaths:
            robopage = parse_yaml_raw_as(Robopage, robopath.read_text())
            if robopage.name is None:
                robopage.name = robopath.stem

            num_functions += len(robopage.functions)
            # TODO: make sure function names are unique
            robopages[robopath] = robopage

        print(f":book: loaded {num_functions} functions from {path}")

        return Robook(pages=robopages)
