import asyncio
import requests
import rigging as rg
from rich import print


# we need to wrap the tools in a class that Rigging can understand
class Wrapper(rg.Tool):
    # we'll set these in the constructor
    name = "_"
    description = "_"

    def __init__(self, func: dict):
        self.name = func["name"]
        self.description = func["description"]
        self.function = func
        # declare dynamically the function by its name
        setattr(Wrapper, self.name, self._execute_function)

    def _execute_function(self, *args, **kwargs):
        print(f"executing {self.name}{kwargs} ...")

        # execute the call via robopages and return the result to Rigging
        return requests.post(
            "http://localhost:8000/process",
            json=[
                {
                    "type": "function",
                    "function": {
                        "name": self.name,
                        "arguments": kwargs,
                    },
                }
            ],
        ).json()[0]["content"]

    def get_description(self) -> rg.tool.ToolDescription:
        """Creates a full description of the tool for use in prompting"""

        return rg.tool.ToolDescription(
            name=self.name,
            description=self.description,
            functions=[
                rg.tool.ToolFunction(
                    name=self.name,
                    description=self.description,
                    parameters=[
                        rg.tool.ToolParameter(
                            name=param_name,
                            # rigging expects python types
                            type="str" if param["type"] == "string" else param["type"],
                            description=param["description"],
                        )
                        for param_name, param in self.function["parameters"][
                            "properties"
                        ].items()
                    ],
                )
            ],
        )


async def run(model: str):
    # get the tools from the Robopages server and wrap each function for Rigging
    tools = [
        Wrapper(function["function"])
        for function in requests.get("http://localhost:8000/").json()
    ]

    chat = (
        await rg.get_generator(model)
        .chat("Find open ports on 127.0.0.1")
        .using(*tools, force=True)
        .run()
    )

    print(chat.last.content)


asyncio.run(run("gpt-4o"))
