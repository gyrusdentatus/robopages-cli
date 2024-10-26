import asyncio
import requests
import rigging as rg
from rich import print


# we need to wrap the tools in a class that Rigging can understand
class Wrapper(rg.Tool):
    # we'll set these in the constructor
    name = "_"
    description = "_"

    def __init__(self, tool: dict):
        self.tool = tool
        self.name = tool["name"]
        self.description = tool["description"]

        # declare dynamically the functions by their name
        for function in tool["functions"]:
            setattr(
                Wrapper,
                function["name"],
                lambda self, *args, **kwargs: self._execute_function(
                    function["name"], *args, **kwargs
                ),
            )

    def _execute_function(self, func_name: str, *args, **kwargs):
        print(f"executing {self.name}.{func_name}{kwargs} ...")
        # execute the call via robopages and return the result to Rigging
        return requests.post(
            "http://localhost:8000/process",
            json=[
                {
                    "type": "function",
                    "function": {
                        "name": func_name,
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
                    name=function["name"],
                    description=function["description"],
                    parameters=[
                        rg.tool.ToolParameter(
                            name=param["name"],
                            type=param["type"],
                            description=param["description"],
                        )
                        for param in function["parameters"]
                    ],
                )
                for function in self.tool["functions"]
            ],
        )


async def run(model: str):
    # get the tools from the Robopages server and wrap each function for Rigging
    tools = [
        Wrapper(tool)
        for tool in requests.get("http://localhost:8000/?flavor=rigging").json()
    ]

    chat = (
        await rg.get_generator(model)
        .chat("Find open ports on 127.0.0.1")
        .using(*tools, force=True)
        .run()
    )

    print(chat.last.content)


asyncio.run(run("gpt-4o"))
