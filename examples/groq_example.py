import json
import groq
import asyncio
import requests

from rich import print


async def run(model: str):
    client = groq.AsyncClient()

    messages = [
        {
            "role": "user",
            "content": "Find open ports on 127.0.0.1",
        }
    ]

    response = await client.chat.completions.create(
        model=model,
        messages=messages,
        # get the tools from the Robopages server
        tools=requests.get("http://localhost:8000/").json(),
    )

    print(response)

    # if the response contains tool calls
    if response.choices[0].message.tool_calls:
        # execute them via the API
        results = requests.post(
            "http://localhost:8000/process",
            json=[
                {
                    "id": tool_call.id,
                    "type": tool_call.type,
                    "function": {
                        "name": tool_call.function.name,
                        # for some reason the arguments are returned as a string
                        "arguments": json.loads(tool_call.function.arguments),
                    },
                }
                for tool_call in response.choices[0].message.tool_calls
            ],
        )
        results.raise_for_status()
        # do whatever you want with the results
        print(results.json())


asyncio.run(run("llama-3.1-70b-versatile"))
