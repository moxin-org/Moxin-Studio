<div align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/75442e77-0211-4fb5-9a17-1a0bed89426f">
  <img width="500" alt="Moxin Studio" src="[https://github.com/user-attachments/assets/b168cf1c-8e2f-4969-bffa-b57ee33950c0](https://github.com/user-attachments/assets/063e750e-ac4b-48e6-ba20-f9b4ac5bbe04)" />
</picture>

# Moxin Studio

</div>

A native desktop AI application built with pure Rust and [Makepad](https://github.com/makepad/makepad). Chat with local and cloud models, generate images, transcribe speech, and manage your model library — all without a Python runtime.

## The OminiX Platform

OminiX is a full-stack, pure Rust AI platform for on-device inference. Moxin Studio is the user-facing layer of a three-part stack:

```
┌─────────────────────────────────────────────┐
│            Moxin Studio (this repo)         │  Desktop UI (Rust + Makepad)
│         Chat · Models · Voice · Settings    │
└──────────────────────┬──────────────────────┘
                       │ OpenAI-compatible REST/WS
┌──────────────────────▼──────────────────────┐
│               OminiX-API                    │  Local inference server (pure Rust)
│    LLM · ASR · TTS · Image endpoints        │
└──────────────────────┬──────────────────────┘
                       │ Rust crate interface
┌──────────────────────▼──────────────────────┐
│               OminiX-MLX                    │  On-device inference backend
│      Metal-accelerated · MLX framework      │  (Apple Silicon — more platforms coming)
└─────────────────────────────────────────────┘
```

- [**OminiX-MLX**](https://github.com/OminiX-ai/OminiX-MLX) — The Apple Silicon inference engine. Pure-Rust bindings to Apple's MLX framework — Metal GPU, unified memory, lazy evaluation. Supports LLMs (Qwen, GLM, Mistral, MiniCPM), VLMs, ASR (Paraformer, Qwen3-ASR), TTS (GPT-SoVITS), and image generation (FLUX, Z-Image).

- [**OminiX-API**](https://github.com/OminiX-ai/OminiX-API) — Local AI inference server in pure Rust. OpenAI-compatible HTTP and WebSocket endpoints for chat completions, transcription, TTS, and image generation. Supports dynamic model loading at runtime without restarts.

- **Moxin Studio** (this repo) — The desktop application. Connects to OminiX-API for local inference, and also supports cloud providers (OpenAI, Anthropic, Google Gemini, DeepSeek, OpenRouter, SiliconFlow, and more).

All three projects are at [github.com/OminiX-ai](https://github.com/OminiX-ai).

## Features

- **Multi-provider chat** — Local models via OminiX-API or Ollama; cloud via OpenAI, Anthropic, Gemini, DeepSeek, OpenRouter, SiliconFlow
- **Model Hub** — Discover, download, and run models directly. Supports LLM, VLM, ASR, TTS, and image generation
- **Image generation** — Local or cloud image endpoints
- **Voice I/O** — Speech-to-text and text-to-speech with voice cloning
- **MCP support** — Model Context Protocol for tool use
- **Chat history** — Persistent, searchable conversation history
- **Dark mode** — Full light/dark theme

## Project Structure

```
Moxin-Studio/
├── moly-shell/          # Main application binary (moxin-studio)
├── moly-data/           # Shared state, persistence, API clients
├── moly-widgets/        # Reusable UI components and theming
└── apps/
    ├── moly-chat/       # Chat interface
    ├── moly-hub/        # Model Hub (discovery, download, load/unload)
    ├── moly-settings/   # Provider and API key configuration
    ├── moly-mcp/        # MCP server configuration
    └── moly-voice/      # Voice I/O
```

## Requirements

- macOS 14.0+ (Sonoma)
- Rust 1.82+
- For local inference: OminiX-API with an Apple Silicon Mac (M1–M5)

## Getting Started

```bash
git clone https://github.com/OminiX-ai/Moxin-Studio.git
cd Moxin-Studio
cargo run -p moly-shell --bin moxin-studio
```

For local model inference, set up [OminiX-API](https://github.com/OminiX-ai/OminiX-API) — see its README. Moxin Studio will auto-start the API server when you load a model from the Hub.

For cloud providers, open Settings in the app and configure your API keys.

## License

[Apache 2.0](LICENSE)
