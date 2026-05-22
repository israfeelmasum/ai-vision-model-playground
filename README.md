# AI Vision Model Playground

A single-page web playground for vision-language models running locally via **Ollama**.

Supports **LLaVA**, **Gemma 3**, **Qwen2-VL / Qwen2.5-VL**, **Moondream**, **BakLLaVA**, and any other vision model available in Ollama.

---

## Features

- Upload an image (drag & drop / file picker) or provide a URL
- Select any Ollama model — vision models are automatically detected
- Edit the system message from the UI
- Streaming responses with token count and timing
- Quick-prompt presets (describe, OCR, mood analysis, etc.)
- Lightweight markdown rendering in responses
- Two backend implementations: **Python (FastAPI)** and **Rust (Axum)**

---

## Prerequisites

1. Install [Ollama](https://ollama.com) and start it:
   ```
   ollama serve
   ```
2. Pull a vision model, e.g.:
   ```
   ollama pull llava
   ollama pull qwen2-vl
   ollama pull gemma3
   ollama pull moondream
   ```

---

## Python Backend (FastAPI) — Port 8000

**Requires Python 3.13**

```bash
cd python
pip install -r requirements.txt
python server.py
```

Then open: http://localhost:8000

---

## Rust Backend (Axum) — Port 8001

**Requires Rust (stable)**

```bash
cd rust
cargo run --release
```

Then open: http://localhost:8001

---

## Project Structure

```
.
├── index.html          # Single-page frontend (served by either backend)
├── python/
│   ├── server.py       # FastAPI app
│   └── requirements.txt
├── rust/
│   ├── src/main.rs     # Axum app
│   └── Cargo.toml
├── .gitignore
└── README.md
```

---

## API

Both backends expose the same two endpoints:

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/models` | List installed Ollama models with vision flag |
| `POST` | `/api/chat` | Send image + prompt, streams NDJSON response |

### POST `/api/chat` — multipart/form-data fields

| Field | Type | Description |
|-------|------|-------------|
| `model` | string | Ollama model name |
| `prompt` | string | User prompt |
| `system` | string | System message (optional) |
| `image` | file | Uploaded image (optional) |
| `image_url` | string | Image URL (optional, used if no file) |

---

## Recommended Vision Models

| Model | Pull Command | Notes |
|-------|-------------|-------|
| LLaVA 7B | `ollama pull llava` | Fast general vision |
| LLaVA 13B | `ollama pull llava:13b` | More accurate |
| Qwen2-VL 7B | `ollama pull qwen2-vl` | Excellent OCR & diagrams |
| Qwen2.5-VL 7B | `ollama pull qwen2.5vl` | Latest Qwen vision |
| Gemma 3 4B | `ollama pull gemma3:4b` | Google, fast |
| Gemma 3 12B | `ollama pull gemma3:12b` | Higher quality |
| Moondream | `ollama pull moondream` | Tiny, very fast |
| MiniCPM-V | `ollama pull minicpm-v` | High resolution support |
