//! AI Vision Playground — Axum backend (Rust)
//! Proxies requests to a local Ollama instance.
//! Run: cargo run --release   (serves on :8001)

use axum::{
    body::Body,
    extract::{Multipart, State},
    http::{header, Response, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

const OLLAMA: &str = "http://localhost:11434";

const VISION_KEYWORDS: &[&str] = &[
    "llava", "moondream", "qwen2-vl", "qwen2.5vl",
    "gemma3", "minicpm-v", "bakllava", "llava-phi3", "llava-llama3",
];

fn is_vision(name: &str) -> bool {
    let lower = name.to_lowercase();
    VISION_KEYWORDS.iter().any(|k| lower.contains(k))
}

#[derive(Clone)]
struct AppState {
    client: Client,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        client: Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/models", get(list_models))
        .route("/api/chat", post(chat))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:8001";
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Rust (Axum) server → http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> impl IntoResponse {
    // index.html lives two levels up from rust/
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("index.html");
    match fs::read_to_string(&path) {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
    }
}

// ── Models ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Deserialize)]
struct OllamaTagsResp {
    models: Vec<OllamaModel>,
}

#[derive(Serialize)]
struct ModelInfo {
    name: String,
    vision: bool,
}

async fn list_models(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    match s.client.get(format!("{OLLAMA}/api/tags")).send().await {
        Ok(r) => match r.json::<OllamaTagsResp>().await {
            Ok(tags) => {
                let models: Vec<ModelInfo> = tags
                    .models
                    .into_iter()
                    .map(|m| {
                        let vision = is_vision(&m.name);
                        ModelInfo { name: m.name, vision }
                    })
                    .collect();
                Json(json!({ "models": models })).into_response()
            }
            Err(e) => Json(json!({ "models": [], "error": e.to_string() })).into_response(),
        },
        Err(e) => Json(json!({
            "models": [],
            "error": format!("Cannot connect to Ollama: {e}")
        }))
        .into_response(),
    }
}

// ── Chat ──────────────────────────────────────────────────────────────────

async fn chat(State(s): State<Arc<AppState>>, mut mp: Multipart) -> impl IntoResponse {
    let mut model = String::new();
    let mut prompt = String::new();
    let mut system = String::new();
    let mut image_b64: Option<String> = None;
    let mut image_url = String::new();

    while let Some(field) = mp.next_field().await.unwrap_or(None) {
        match field.name().unwrap_or("") {
            "model"     => model     = field.text().await.unwrap_or_default(),
            "prompt"    => prompt    = field.text().await.unwrap_or_default(),
            "system"    => system    = field.text().await.unwrap_or_default(),
            "image_url" => image_url = field.text().await.unwrap_or_default(),
            "image" => {
                let bytes = field.bytes().await.unwrap_or_default();
                if !bytes.is_empty() {
                    image_b64 = Some(B64.encode(&bytes));
                }
            }
            _ => {}
        }
    }

    // Fetch image from URL if no file was uploaded
    if image_b64.is_none() && !image_url.trim().is_empty() {
        if let Ok(r) = s.client.get(image_url.trim()).send().await {
            if let Ok(bytes) = r.bytes().await {
                image_b64 = Some(B64.encode(&bytes));
            }
        }
    }

    let mut messages: Vec<Value> = vec![];
    if !system.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": system.trim() }));
    }

    let content = if prompt.trim().is_empty() { "Describe this image." } else { prompt.trim() };
    let mut user_msg = json!({ "role": "user", "content": content });
    if let Some(b64) = image_b64 {
        user_msg["images"] = json!([b64]);
    }
    messages.push(user_msg);

    let payload = json!({ "model": model, "messages": messages, "stream": true });

    match s
        .client
        .post(format!("{OLLAMA}/api/chat"))
        .json(&payload)
        .send()
        .await
    {
        Ok(ollama_resp) => {
            let stream = ollama_resp
                .bytes_stream()
                .map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
            Response::builder()
                .header(header::CONTENT_TYPE, "application/x-ndjson")
                .body(Body::from_stream(stream))
                .unwrap()
                .into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}
