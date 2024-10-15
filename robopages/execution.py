import shutil
import subprocess

from rich import print
from rich.prompt import Prompt

from robopages.models import Function, Robocall


def resolve_function_call(function: Function, call: Robocall) -> list[str]:
    """Return a command line by resolving the binary and replacing it with a docker command line if needed."""

    cmdline = function.get_command_line(call.function.arguments)

    app_name_idx = 0
    app_name = cmdline[app_name_idx]
    if app_name == "sudo":
        app_name_idx = 1
        app_name = cmdline[app_name_idx]

    print("DEBUG: REMOVE ME")
    binary = None  # shutil.which(app_name)
    if not binary:
        # binary not in $PATH
        if not function.container:
            # no container set, just report the error
            raise Exception(
                f"binary {app_name} not found in $PATH and container not set"
            )

        from robopages import docker

        # TODO: implement build with local Dockerfile

        # pull the image if needed
        if function.container.image:
            docker.pull(function.container.image)
        elif function.container.build:
            docker.build(function.container)
            # set newly created image name
            function.container.image = function.container.name
        else:
            raise Exception(
                f"container for function {call.function.name} not found, please set either image or build"
            )

        # TODO: if command line is dockerized, remove sudo

        # create a new command line by replacing the app name
        # with the docker equivalent command line
        cmdline = docker.dockerize_command_line(
            cmdline, app_name_idx, function.container
        )

    return cmdline


def execute(call: Robocall, function: Function, interactive: bool = True) -> str:
    print(
        f":robot: executing [bold]{call.function.name}[/]({', '.join(call.function.arguments.values())})"
    )

    output = None
    cmdline = resolve_function_call(function, call)

    if (
        not interactive
        or Prompt.ask(
            f":eyes: execute? [yellow]{' '.join(cmdline)}[/] ",
            choices=["y", "n"],
            default="n",
        )
        == "y"
    ):
        print(f":robot: [yellow]{' '.join(cmdline)}[/]")
        res = subprocess.run(
            " ".join(cmdline),
            capture_output=True,
            text=True,
            shell=True,
        )
        err = res.stderr.strip()
        out = res.stdout.strip()
        output = f"{err}\n{out}" if err else out
    else:
        output = "<not executed>"

    return output
