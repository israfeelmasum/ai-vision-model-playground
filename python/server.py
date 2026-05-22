"""
AI Vision Playground — FastAPI backend (Python 3.13)
Proxies requests to a local Ollama instance.
Run: uvicorn server:app --reload --port 8000
"""

import base64
from pathlib import Path
from typing import Annotated

import httpx
import uvicorn
from fastapi import FastAPI, File, Form, UploadFile
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse, StreamingResponse

app = FastAPI(title="AI Vision Playground")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

OLLAMA_BASE = "http://localhost:11434"

VISION_KEYWORDS = (
    "llava", "moondream", "qwen2-vl", "qwen2.5vl",
    "gemma3", "minicpm-v", "bakllava", "llava-phi3", "llava-llama3",
)


def _is_vision(name: str) -> bool:
    n = name.lower()
    return any(k in n for k in VISION_KEYWORDS)


@app.get("/", response_class=HTMLResponse)
async def index():
    html = Path(__file__).parent.parent / "index.html"
    return HTMLResponse(content=html.read_text(encoding="utf-8"))


@app.get("/api/models")
async def list_models():
    async with httpx.AsyncClient(timeout=10) as client:
        try:
            resp = await client.get(f"{OLLAMA_BASE}/api/tags")
            resp.raise_for_status()
            raw = resp.json().get("models", [])
            return {
                "models": [
                    {"name": m["name"], "vision": _is_vision(m["name"])}
                    for m in raw
                ]
            }
        except httpx.ConnectError:
            return {"models": [], "error": "Cannot connect to Ollama — is it running on port 11434?"}
        except Exception as exc:
            return {"models": [], "error": str(exc)}


@app.post("/api/chat")
async def chat(
    model: Annotated[str, Form()],
    prompt: Annotated[str, Form()],
    system: Annotated[str, Form()] = "",
    image: Annotated[UploadFile | None, File()] = None,
    image_url: Annotated[str, Form()] = "",
):
    images: list[str] = []

    if image and image.size:
        raw = await image.read()
        images = [base64.b64encode(raw).decode()]
    elif image_url.strip():
        async with httpx.AsyncClient(timeout=30, follow_redirects=True) as client:
            try:
                r = await client.get(image_url.strip())
                r.raise_for_status()
                images = [base64.b64encode(r.content).decode()]
            except Exception as exc:
                return {"error": f"Failed to fetch image URL: {exc}"}

    messages: list[dict] = []
    if system.strip():
        messages.append({"role": "system", "content": system.strip()})

    user_msg: dict = {"role": "user", "content": prompt or "Describe this image."}
    if images:
        user_msg["images"] = images
    messages.append(user_msg)

    payload = {"model": model, "messages": messages, "stream": True}

    async def stream():
        async with httpx.AsyncClient(timeout=httpx.Timeout(300.0)) as client:
            async with client.stream("POST", f"{OLLAMA_BASE}/api/chat", json=payload) as resp:
                async for line in resp.aiter_lines():
                    if line:
                        yield line + "\n"

    return StreamingResponse(stream(), media_type="application/x-ndjson")


if __name__ == "__main__":
    uvicorn.run("server:app", host="0.0.0.0", port=8000, reload=True)
