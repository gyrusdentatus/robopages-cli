import pathlib
import os
import shutil
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


class Container(BaseModel):
    image: str
    args: list[str]
    volumes: list[str]

    def pull(self) -> None:
        print(f":water_wave: pulling container [green]{self.image}[/]\n")
        os.system(f'docker pull "{self.image}"')
        print()


class Function(BaseModel):
    description: str = ""
    parameters: dict[str, Parameter] = {}
    container: Container | None = None
    cmdline: list[str] | None = None

    def _handle_with_container(self, app_name_idx: int) -> None:
        if not self.cmdline:
            raise Exception("no command line to execute")

        elif not self.container:
            raise Exception(
                f"binary {self.cmdline[app_name_idx]} not found in $PATH and container not set"
            )

        # pull the image
        self.container.pull()

        # create a new command line by replacing the app name
        # with the docker equivalent command line
        cmdline = []
        for idx, arg in enumerate(self.cmdline):
            if idx != app_name_idx:
                cmdline.append(arg)
            else:
                cmdline.extend(["docker", "run", "--rm", "-it"])
                # add volumes if any
                for volume in self.container.volumes:
                    # expand any environment variable
                    expanded_volume = os.path.expandvars(volume)
                    cmdline.extend(["-v", expanded_volume])

                # add any additional args
                if self.container.args:
                    cmdline.extend(self.container.args)

                # add image
                cmdline.append(self.container.image)

        self.cmdline = cmdline

    def _arg_value(self, arg: str, arguments: dict[str, str]) -> str:
        # TODO: add better parsing for multiple interpolations
        if arg.startswith("${"):
            arg_name = arg[2:-1]
            if arg_name in arguments:
                return arguments[arg_name]
            else:
                raise Exception(f"Argument {arg_name} not found")
        else:
            return arg

    def get_command_line(self, arguments: dict[str, str]) -> list[str]:
        if not self.cmdline:
            raise Exception("No command line to execute")

        app_name_idx = 0
        app_name = self.cmdline[0]
        if app_name == "sudo":
            app_name = self.cmdline[1]
            app_name_idx = 1

        binary = shutil.which(app_name)
        if not binary:
            self._handle_with_container(app_name_idx)

        return [self._arg_value(arg, arguments) for arg in self.cmdline]

    def to_string(self, name: str) -> str:
        args = []
        for param_name, param in self.parameters.items():
            args.append(f"[yellow]{param_name}[/][dim]:{param.type}[/]")

        return f"{name}({', '.join(args)})"


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

    def to_openai(self, filter: str | None = None) -> list[dict]:
        """Convert all robopages to OpenAI compatible tools (https://platform.openai.com/docs/guides/function-calling)."""

        filtered: list[Robopage] = []
        for path, page in self.pages.items():
            if filter is None or filter in str(path):
                filtered.append(page)

        return [tool for page in filtered for tool in page.to_openai()]

    def to_ollama(self, filter: str | None = None) -> list[dict]:
        """Convert all robopages to OLLAMA compatible tools (https://python.langchain.com/v0.1/docs/integrations/chat/ollama_functions/)."""

        return self.to_openai(filter)

    def to_rigging(self) -> list["rg.Tool"]:
        """Convert all robopages to Rigging compatible tools (https://rigging.dreadnode.io/topics/tools/)."""
        import rigging as rg

        class RiggingWrapper(rg.Tool):
            name = "a"
            description = "b"

            def __init__(self, robopage: Robopage, func_name: str, func: Function):
                self.name = func_name
                self.description = func.description
                self.func = func

            def get_description(self) -> rg.tool.ToolDescription:
                """Creates a full description of the tool for use in prompting"""

                parameters = []
                for param_name, param in self.func.parameters.items():
                    parameters.append(
                        rg.tool.ToolParameter(
                            name=param_name,
                            type=param.type,
                            description=param.description,
                        )
                    )

                return rg.tool.ToolDescription(
                    name=self.name,
                    description=self.description,
                    functions=[
                        rg.tool.ToolFunction(
                            name=self.name,
                            description=self.description,
                            parameters=parameters,
                        )
                    ],
                )

        return [
            RiggingWrapper(page, func_name, func)
            for page in self.pages.values()
            for (func_name, func) in page.functions.items()
        ]

    def process(
        self, calls: list[dict | Robocall], interactive: bool = True
    ) -> dict[str, str]:
        """Process a list of tool calls from an LLM and return the result for each."""

        results = {}

        for call in [
            Robocall(**call) if isinstance(call, dict) else call for call in calls
        ]:
            output = None
            for page in self.pages.values():
                if call.function.name in page.functions:
                    print(
                        f":robot: executing [bold]{page.name}[/].{call.function.name}({', '.join(call.function.arguments.values())})"
                    )

                    func = page.functions[call.function.name]
                    cmdline = func.get_command_line(call.function.arguments)

                    if (
                        not interactive
                        or Prompt.ask(
                            f":eyes: execute? [yellow]{' '.join(cmdline)}[/] ",
                            choices=["y", "n"],
                            default="n",
                        )
                        == "y"
                    ):
                        res = subprocess.run(cmdline, capture_output=True, text=True)
                        err = res.stderr.strip()
                        out = res.stdout.strip()
                        output = f"{err}\n{out}" if err else out
                    else:
                        output = "<not executed>"

                    break

            if output is None:
                raise Exception(f"Function {call.function.name} not found")
            else:
                results[call.function.name] = output

        return results

    @staticmethod
    def load(filter: str | None = None) -> "Robook":
        """Load all robopages from the environment variable ROBOPAGES_PATH if set or the default path."""

        return Robook.from_path(
            pathlib.Path(os.getenv(DEFAULT_PATH_ENV_VAR) or DEFAULT_PATH), filter
        )

    @staticmethod
    def from_path(
        path: pathlib.Path = DEFAULT_PATH, filter: str | None = None
    ) -> "Robook":
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
            if not filter or filter in str(robopath):
                robopage = parse_yaml_raw_as(Robopage, robopath.read_text())
                if robopage.name is None:
                    robopage.name = robopath.stem

                num_functions += len(robopage.functions)
                # TODO: make sure function names are unique
                robopages[robopath] = robopage

        print(f":book: loaded {num_functions} functions from {path}")

        return Robook(pages=robopages)
