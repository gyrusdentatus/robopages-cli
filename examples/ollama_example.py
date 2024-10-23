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
