import pathlib
import os

DEFAULT_PATH_ENV_VAR = "ROBOPAGES_PATH"
DEFAULT_PATH = pathlib.Path(os.path.expanduser("~/.robopages/"))
DEFAULT_EXTENSION = "yml"
DEFAULT_PAGE_FILE_NAME = pathlib.Path(f"robopage.{DEFAULT_EXTENSION}")

DEFAULT_ADDRESS = "127.0.0.1"
DEFAULT_PORT = 8000
