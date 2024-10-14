import ollama
import asyncio

from rich import print

from robopages.models import Robook


async def run(model: str):
    client = ollama.AsyncClient()

    messages = [
        {
            "role": "user",
            # assumes that examples/robopages/nmap.yml is in ~/.robopages/
            "content": "Find open ports on 127.0.0.1",
        }
    ]

    # This will load all pages from ~/.robopages/ and convert to OLLAMA compatible tools.
    #
    # The default path can be overridden by setting the ROBOPAGES_PATH environment variable.
    robopages = Robook.load()

    # to load a specific subset of tools
    # robopages = Robook.load("cybersecurity/offensive")

    # to load a single page
    # robopages = Robook.from_path("my_page.yml")

    # to load a directory of pages
    # robopages = Robook.from_path("./my_pages_dir/")

    response = await client.chat(
        model=model,
        messages=messages,
        tools=robopages.to_ollama(),  # where the magic happens
    )

    print(response)

    # if the response contains tool calls
    if response["message"]["tool_calls"]:
        # execute them in interactive mode
        results = robopages.process(response["message"]["tool_calls"], interactive=True)
        # do whatever you want with the results
        print(results)


asyncio.run(run("llama3.1"))
