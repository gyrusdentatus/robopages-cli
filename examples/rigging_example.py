import asyncio
import os
from loguru import logger
import rigging as rg
from rigging import logging
from rich import print

os.environ["LOGFIRE_IGNORE_NO_CONFIG"] = "1"
logging.configure_logging("DEBUG", None, "DEBUG")


async def run():
    """Main function that runs the chat with RoboPages tools"""

    try:
        logger.info("Fetching tools from RoboPages server")
        # Use the built-in robopages integration
        tools = rg.integrations.robopages("http://localhost:8000")

        logger.info(f"Fetched {len(tools)} tools from RoboPages server")

        prompt = """
        I need you to find all open ports on the local machine (127.0.0.1).
        Please use the available tools to scan the ports and provide a summary of the results.

        Be thorough but concise in your analysis. Present the information in a clear format.

        After scanning, list all the open ports you found and what services might be running on them.
        """

        logger.info("Starting chat with model")
        generator = rg.get_generator("gpt-4o")

        chat = await generator.chat(prompt).using(*tools, force=True).run()

        logger.info("Chat completed. Full conversation:")
        for i, message in enumerate(chat.messages):
            logger.info(f"Message {i + 1} ({message.role}):")
            logger.info(
                message.content[:200] + ("..." if len(message.content) > 200 else "")
            )

        print("\n--- RESULT ---\n")
        print(chat.last.content)

        return chat

    except Exception as e:
        logger.error(f"Error: {e}")
        import traceback

        traceback.print_exc()
        return None


if __name__ == "__main__":
    chat = asyncio.run(run())
    if chat:
        print(chat.conversation)
