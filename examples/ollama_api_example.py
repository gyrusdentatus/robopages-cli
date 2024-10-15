import ollama
import asyncio
import requests

from rich import print


async def run(model: str):
    client = ollama.AsyncClient()

    messages = [
        {
            "role": "user",
            "content": "Find vulnerabilities on 127.0.0.1",
        }
    ]

    tools = requests.get("http://localhost:8000/").json()

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
        print(results.json())


asyncio.run(run("llama3.1"))
