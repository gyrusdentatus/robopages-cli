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
