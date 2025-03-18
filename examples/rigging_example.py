import asyncio
import os
from loguru import logger
import requests
import rigging as rg
from rigging import logging
from rich import print
from typing import Annotated

os.environ["LOGFIRE_IGNORE_NO_CONFIG"] = "1"
logging.configure_logging("DEBUG", None, "DEBUG")


class RoboPagesTool(rg.Tool):
    """Base class for RoboPages tools that are dynamically wrapped from the server"""

    name = "robopages_tool"
    description = "A tool from the RoboPages server"

    def __init__(self, tool_data):
        self.data = tool_data
        self.name = tool_data["name"]
        self.description = tool_data["description"]

        for function in tool_data["functions"]:
            func_name = function["name"]
            params = function.get("parameters", [])

            def make_func(f_name, f_params):
                param_list = []
                annotations = {"return": str}

                for p in f_params:
                    param_name = p["name"]
                    param_desc = p.get("description", "")
                    annotations[param_name] = Annotated[str, param_desc]
                    param_list.append(param_name)

                def dynamic_func(self, **kwargs):
                    """Dynamically created function"""
                    filtered_kwargs = {
                        k: v for k, v in kwargs.items() if k in param_list
                    }
                    return self._call_function(f_name, filtered_kwargs)

                dynamic_func.__name__ = f_name
                dynamic_func.__annotations__ = annotations
                dynamic_func.__doc__ = function.get("description", "")

                return dynamic_func

            setattr(
                self,
                func_name,
                make_func(func_name, params).__get__(self, self.__class__),
            )

    def _call_function(self, func_name: str, args: dict) -> str:
        """Call the function on the RoboPages server"""
        print(f"Calling {self.name}.{func_name}({args})")

        try:
            response = requests.post(
                "http://localhost:8000/process",
                json=[
                    {
                        "type": "function",
                        "function": {
                            "name": func_name,
                            "arguments": args,
                        },
                    }
                ],
            )

            return response.json()[0]["content"]
        except Exception as e:
            print(f"Error calling function: {e}")
            return f"Error: {str(e)}"


def create_tool_class(tool_data):
    """Create a new Tool class dynamically for a specific tool from RoboPages"""
    tool_name = tool_data["name"]
    tool_desc = tool_data["description"]

    class_name = f"{tool_name.replace(' ', '')}Tool"

    new_class = type(
        class_name,
        (rg.Tool,),
        {
            "name": tool_name,
            "description": tool_desc,
        },
    )

    for function in tool_data["functions"]:
        func_name = function["name"]
        params = function.get("parameters", [])

        param_list = []
        annotations = {"return": str}

        for p in params:
            param_name = p["name"]
            param_desc = p.get("description", "")
            annotations[param_name] = Annotated[str, param_desc]
            param_list.append(param_name)

        def make_function(fn_name, fn_params):
            def dynamic_func(self, **kwargs):
                """Dynamically created function"""
                filtered_kwargs = {k: v for k, v in kwargs.items() if k in fn_params}

                # Call the RoboPages server
                try:
                    response = requests.post(
                        "http://localhost:8000/process",
                        json=[
                            {
                                "type": "function",
                                "function": {
                                    "name": fn_name,
                                    "arguments": filtered_kwargs,
                                },
                            }
                        ],
                    )
                    return response.json()[0]["content"]
                except Exception as e:
                    print(f"Error calling function: {e}")
                    return f"Error: {str(e)}"

            dynamic_func.__name__ = fn_name
            dynamic_func.__annotations__ = annotations
            dynamic_func.__doc__ = function.get("description", "")

            return dynamic_func

        setattr(new_class, func_name, make_function(func_name, param_list))

    return new_class


async def run():
    """Main function that runs the chat with RoboPages tools"""

    try:
        # Fetch tools from the RoboPages server
        response = requests.get("http://localhost:8000/?flavor=rigging")
        tools_data = response.json()

        logger.info(f"Fetched {len(tools_data)} tools from RoboPages server")
        for tool in tools_data:
            logger.info(f"Tool: {tool['name']} - {tool['description']}")
            for func in tool.get("functions", []):
                logger.info(f"  Function: {func['name']}")

        tools = []
        for tool_data in tools_data:
            tool_class = create_tool_class(tool_data)
            tools.append(tool_class())

        logger.info(f"Created {len(tools)} tool instances")

        prompt = """
        I need you to find all open ports on the local machine (127.0.0.1).
        Please use the available tools to scan the ports and provide a summary of the results.

        Be thorough but concise in your analysis. Present the information in a clear format.

        After scanning, list all the open ports you found and what services might be running on them.
        """

        logger.info("Starting chat with model")
        generator = rg.get_generator("gpt-4o")

        chat = await generator.chat(prompt).using(*tools, force=True).run()

        logger.info("Chat completed. Full conversation:")
        for i, message in enumerate(chat.messages):
            logger.info(f"Message {i + 1} ({message.role}):")
            logger.info(
                message.content[:200] + ("..." if len(message.content) > 200 else "")
            )

        print("\n--- RESULT ---\n")
        print(chat.last.content)

    except Exception as e:
        logger.error(f"Error: {e}")
        import traceback

        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(run())
