# Robopages

CLI for https://github.com/dreadnode/robopages

## Installation

TODO: Add new installation instructions

Pages are loaded by default from the `~/.robopages/` directory (or any folder set in the `ROBOPAGES_PATH` environment variable), see the `https://github.com/dreadnode/robopages` repository for examples.

## Usage

This project consists of a CLI for creating, viewing and serving robopages as a REST API.

### CLI

Install robopages:

```bash
# install https://github.com/dreadnode/robopages to ~/.robopages/robopages-main
robopages install 

# install a custom repository
robopages install --source user/repo

# install from a local archive
robopages install --source /path/to/archive.zip
```

View installed robopages:

```bash
robopages view
```

Create a robopage:

```bash
robopages create --name my_first_page.yml
```

Start the REST API:

```bash
robopages serve
```

Execute a function manually without user interaction:

```bash
robopages run --function nikto_scan --auto
```

### SDK

Use with OLLAMA (or any OpenAI function calling schema compatible client and model) via the REST API:

```python
import ollama
import asyncio
import requests

async def run(model: str):
    client = ollama.AsyncClient()

    messages = [
        {
            "role": "user",
            "content": "Find open ports on 127.0.0.1",
        }
    ]

    # assumes that the `robopages serve` API is running
    tools = requests.get("http://localhost:8000/").json()

    response = await client.chat(
        model=model,
        messages=messages,
        tools=tools,
    )

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

For more examples, see the `examples/` folder.