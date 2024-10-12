# Robopages

A YAML based format for describing tools to LLMs, like man pages but for robots!

## Installation

```bash
poetry install 
```

Pages are loaded by default from the `~/.robopages/` directory, see the `examples` folder for examples.

## Usage

This python package includes a CLI for creating and converting robopages and a library for using it as an SDK.

### CLI

Enter the poetry shell:

```bash
poetry shell
```

Create a robopage:

```bash
robopages create my_first_page.yml
```

Convert to OpenAI compatible tools:

```bash
robopages to-openai --path ./examples/robopages
```

### SDK

Use with OLLAMA (or any OpenAI function calling schema compatible client and model):

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
    # Alternatively you can:
    # - override the default path by setting the ROBOPAGES_PATH environment variable
    # - load a single page with Robook.from_path("my_page.yml").to_ollama()
    # - load a directory of pages with Robook.from_path("./my_pages_dir/").to_ollama()
    robopages = Robook.load()

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
