# Robopages

A YAML based format for describing tools to LLMs, like man pages but for robots!

## Installation

```bash
poetry install 
```

Pages are loaded by default from the `~/.robopages/` directory (or any folder set in the `ROBOPAGES_PATH` environment variable), see the `examples/robopages/` folder for examples.

## Usage

This project consists of a CLI for creating, viewing and serving robopages as a REST API.

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

Execute a function manually without user interaction:

```bash
robopages run nikto_scan --auto
```

### Robopage

A robopage is a YAML file describing one or more tools that can be used by an LLM.

```yaml
# General description of this page.
description: Scan web server for known vulnerabilities.

# Define one or more functions that can be used by the LLM.
functions:
  nikto_scan:
    # Description of what the function does.
    description: Scan a specific target web server for known vulnerabilities.
    # Function parameters.
    parameters:
      target:
        type: string
        description: The URL of the target to scan.
        examples:
          - http://www.target.com/
          - https://target.tld

    # If the binary from cmdline is not found in $PATH, specify which container to pull and run it with.
    container:
      image: frapsoft/nikto
      args:
        - --net=host

    # The command line to execute.
    cmdline:
      - nikto
      - -host
      # Use these placeholders for the parameters.
      # Supported syntax variations: `${param}` and `${param or default_value}`
      - ${target}
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