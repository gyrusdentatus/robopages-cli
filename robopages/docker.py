import os

from robopages.models import Container

# TODO: this is horrible, we should be using the docker python sdk


def pull(image: str):
    """Pull a docker image if it is not already present."""

    os.system(f"docker images -q '{image}' | grep -q . || docker pull '{image}'")


def dockerize_command_line(
    cmdline: list[str], app_name_idx: int, container: Container
) -> list[str]:
    """Create a new command line by replacing the app name with the docker equivalent command line."""

    dockerized = []
    for idx, arg in enumerate(cmdline):
        if idx != app_name_idx:
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
