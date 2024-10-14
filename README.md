# Robopages

A YAML based format for describing tools to LLMs, like man pages but for robots!

## Installation

```bash
poetry install 
```

Pages are loaded by default from the `~/.robopages/` directory, see the `examples/robopages/` folder for examples.

## Usage

This project includes a CLI for creating and viewing robopages and can be used as a python library or as a REST API.

### CLI

Enter the poetry shell:

```bash
poetry shell
```

View installed robopages:

```bash
robopages view
```

Create a robopage:

```bash
robopages create my_first_page.yml
```

Convert to OpenAI compatible tools:

```bash
robopages to-json --path ./examples/robopages
```

Start the REST API:

```bash
robopages serve
```

### SDK

Use with OLLAMA (or any OpenAI function calling schema compatible client and model) via the REST API:

```python
import ollama
import asyncio
import requests

from rich import print


async def run(model: str):
    client = ollama.AsyncClient()

    messages = [
        {
            "role": "user",
            "content": "Find open ports on 127.0.0.1",
        }
    ]

    tools = requests.get("http://localhost:8000/offensive").json()

    response = await client.chat(
        model=model,
        messages=messages,
        tools=tools,
    )

    print(response)

    # if the response contains tool calls
    if response["message"]["tool_calls"]:
        # execute them via the API
        results = requests.post(
            "http://localhost:8000/process", json=response["message"]["tool_calls"]
        )
        # do whatever you want with the results
        print(results)


asyncio.run(run("llama3.1"))
```

Or by using the package:

```python
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
```


Use with [Rigging]( https://github.com/dreadnode/rigging), Dreadnonde's lightweight LLM framework:

```python
import asyncio
import rigging as rg
from rich import print

from robopages.models import Robook

async def run(model: str):
    robopages = Robook.load()

    chat = (
        await rg.get_generator(model)
        # assumes that examples/robopages/nmap.yml is in ~/.robopages/
        .chat("Find open ports on 127.0.0.1")
        .using(*robopages.to_rigging(), force=True)
        .run()
    )

    print(chat.last.content)


asyncio.run(run("ollama/llama3.1,api_base=http://your-server:11434"))
```
