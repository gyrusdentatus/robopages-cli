from typing import Optional
from fastapi import FastAPI, HTTPException

from robopages.models import Robocall, Robook

book: Robook | None = None
app = FastAPI()


@app.get(
    "/{path_filter:path}",
    summary="Retrieve the OpenAI compatible JSON schema of the Robopages.",
)
def get_book(path_filter: Optional[str] = None) -> list[dict]:
    return book.to_openai(path_filter if path_filter != "" else None)


@app.post("/process", summary="Execute the list of function calls.")
def process(calls: list[Robocall]) -> dict[str, str]:
    try:
        return book.process(calls, interactive=False)
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))
