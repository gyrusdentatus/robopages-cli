import os
import pathlib

from rich import print

from docker.models.images import Image
import docker

from robopages.models import Container

client = docker.from_env()


def image_exists(image: str) -> bool:
    """Return True if the image was already pulled."""

    return len(client.images.list(all=True, filters={"reference": image})) > 0


def pull(image: str) -> None:
    """Pull a docker image if it is not already present."""

    if not image_exists(image):
        prev: str | None = None
        for item in client.api.pull(image, stream=True, decode=True):
            if "error" in item:
                raise Exception(item["error"])
            elif "status" in item and item["status"] != prev:
                print("[dim]" + item["status"].strip() + "[/]")
                prev = item["status"]


def build(dockerfile: str, tag: str) -> None:
    """Build a docker image."""

    print(f":toolbox: building {dockerfile} as '{tag}' ...")

    dockerfile_path = pathlib.Path(dockerfile)
    if not dockerfile_path.exists():
        raise Exception(f"dockerfile {dockerfile} not found")

    prev: str | None = None
    id: str | None = None

    for item in client.api.build(
        path=str(dockerfile_path.parent),
        dockerfile=dockerfile,
        tag=tag,
        decode=True,
    ):
        if "error" in item:
            raise Exception(item["error"])
        elif "stream" in item and item["stream"] != prev:
            print("[dim]" + item["stream"].strip() + "[/]")
            prev = item["stream"]
        elif "aux" in item:
            id = item["aux"]["ID"]

    if id is None:
        raise Exception("Failed to build image")


def dockerize_command_line(
    cmdline: list[str], app_name_idx: int, container: Container
) -> list[str]:
    """Create a new command line by replacing the app name with the docker equivalent command line."""

    dockerized = []
    for idx, arg in enumerate(cmdline):
        if arg == "sudo":
            # remove sudo from command line since we're running as a container
            continue
        elif idx != app_name_idx:
            dockerized.append(arg)
        else:
            dockerized.extend(["docker", "run", "--rm"])
            # add volumes if any
            for volume in container.volumes:
                # expand any environment variable
                expanded_volume = os.path.expandvars(volume)
                dockerized.extend(["-v", expanded_volume])

            # add any additional args
            if container.args:
                dockerized.extend(container.args)

            # add image
            dockerized.append(container.image)

    return dockerized
