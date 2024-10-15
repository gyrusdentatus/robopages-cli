import pathlib
import os
import re
import platform

from pydantic import BaseModel
from pydantic_yaml import parse_yaml_raw_as, to_yaml_str
from rich import print

from robopages.defaults import DEFAULT_EXTENSION, DEFAULT_PATH, DEFAULT_PATH_ENV_VAR


class Parameter(BaseModel):
    type: str
    description: str
    required: bool = True
    examples: list[str] = []


class Container(BaseModel):
    image: str | None = None
    name: str | None = None
    build: str | None = None
    args: list[str]
    volumes: list[str] = []


class Function(BaseModel):
    description: str = ""
    parameters: dict[str, Parameter] = {}
    container: Container | None = None
    cmdline: list[str] | None = None
    platforms: dict[str, list[str]] | None = None

    def _arg_value(self, arg: str, arguments: dict[str, str]) -> str:
        """Parse interpolated variables with optional default values."""

        pattern = r"\${(\s*[\w\.]+)\s*(\s+or\s+([^}]+))?}"
        matches = list(re.finditer(pattern, arg, re.I))
        if not matches:
            return arg

        for match in matches:
            expression = match.group(0)
            var_name = match.group(1).strip()
            default_value = match.group(3)

            if var_name in arguments:
                # variable provided as argument
                replace_with = arguments[var_name]
            elif default_value is not None:
                # variable not provided as argument, but with a default value
                replace_with = default_value.strip()
            else:
                raise Exception(
                    f'variable "${var_name}" not found and no default value provided'
                )

            arg = arg.replace(expression, replace_with)

        return arg

    def get_command_line(self, arguments: dict[str, str]) -> list[str]:
        """Get the command line to execute for the provided arguments."""

        cmdline = self.cmdline
        if not cmdline:
            # check for platform specific command lines
            if self.platforms:
                current_system = platform.system().lower()
                if current_system in self.platforms:
                    cmdline = self.platforms[current_system]

        if not cmdline:
            raise Exception("no command line to execute")

        return [self._arg_value(arg, arguments) for arg in cmdline]

    def to_string(self, name: str) -> str:
        args = []
        for param_name, param in self.parameters.items():
            args.append(f"[yellow]{param_name}[/][dim]:{param.type}[/]")

        return f"{name}({', '.join(args)})"


class Robopage(BaseModel):
    name: str | None = None
    description: str | None = None
    functions: dict[str, Function]
    categories: list[str] = []

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

        from robopages import execution

        results = {}

        for call in [
            Robocall(**call) if isinstance(call, dict) else call for call in calls
        ]:
            output = None
            for page in self.pages.values():
                if call.function.name in page.functions:
                    function = page.functions[call.function.name]
                    output = execution.execute(call, function, interactive)
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

        function_names = {}
        for robopath in robopaths:
            if not filter or filter in str(robopath):
                text = robopath.read_text()
                # preprocess any occurrence of ${cwd}
                text = text.replace("${cwd}", str(robopath.parent.resolve()))
                # parse into model
                robopage = parse_yaml_raw_as(Robopage, text)

                # if name is not set, use the file name
                if not robopage.name:
                    robopage.name = robopath.stem

                # if categories are not set, use the path
                if not robopage.categories:
                    relative_parts = list(robopath.relative_to(path).parts)
                    robopage.categories = relative_parts[:-1]

                # make sure function names are unique
                renames = {}
                for func_name in robopage.functions.keys():
                    if func_name in function_names:
                        new_func_name = f"{robopage.name}_{func_name}"
                        if new_func_name not in function_names:
                            print(
                                f":x: function name [yellow]{func_name}[/] in [blue]{robopath}[/] is not unique, renaming to [green]{new_func_name}[/]"
                            )
                            renames[func_name] = new_func_name
                            func_name = new_func_name
                        else:
                            raise Exception(
                                f"function name {func_name} in {robopath} is not unique"
                            )
                    function_names[func_name] = 1

                for old_name, new_name in renames.items():
                    robopage.functions[new_name] = robopage.functions[old_name]
                    del robopage.functions[old_name]

                robopages[robopath] = robopage

        print(f":book: loaded {len(function_names)} functions from {path}")

        return Robook(pages=robopages)
