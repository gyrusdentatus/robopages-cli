# Robopages

CLI for https://github.com/dreadnode/robopages

Pages are loaded by default from the `~/.robopages/` directory (or any folder set in the `ROBOPAGES_PATH` environment variable), see the `https://github.com/dreadnode/robopages` repository for examples.


## Build Docker image

To build the Docker image for the tool, run:

```bash
docker build . -t robopages  
```

## Build from source


Alternatively you can build the project from source, in which case you'll need to have Rust and Cargo [installed on your system](https://rustup.rs/).

Once you have those set up, clone the repository:

```bash
git clone https://github.com/dreadnode/robopages-cli.git
cd robopages-cli
```

Build the project:

```bash
cargo build --release
```

The compiled binary will be available in the `target/release` directory. You can run it directly or add it to your system's PATH:

```bash
# Run directly
./target/release/robopages

# Or, copy to a directory in your PATH (e.g., /usr/local/bin)
sudo cp target/release/robopages /usr/local/bin/
```


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

Create a robopage with the preferred template:

```bash
# create with the basic template, will run the command in the current shell
robopages create --name my_first_page.yml --template basic

# create with the docker-image template, will use a docker image to run the command
robopages create --name my_first_page.yml --template docker-image

# create with the docker-build template, will build a docker image to run the command
robopages create --name my_first_page.yml --template docker-build
```

Start the REST API:

```bash
# this will pre build and pull all containers
robopages serve

# this will build or pull containers on demand
robopages serve --lazy
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

from rich import print


async def run(model: str):
    client = ollama.AsyncClient()

    messages = [
        {
            "role": "user",
            "content": "Find open ports on 127.0.0.1",
        }
    ]

    response = await client.chat(
        model=model,
        messages=messages,
        # get the tools from the Robopages server
        tools=requests.get("http://localhost:8000/").json(),
    )

    print(response)

    # if the response contains tool calls
    if response["message"]["tool_calls"]:
        # execute them via the API
        results = requests.post(
            "http://localhost:8000/process", json=response["message"]["tool_calls"]
        )
        results.raise_for_status()
        # do whatever you want with the results
        print(results.json())


asyncio.run(run("llama3.1"))
```

For more examples, see the `examples/` folder.