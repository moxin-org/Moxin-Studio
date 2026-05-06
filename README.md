<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="moxin-studio-logo-dark.png">
  <img width="300" alt="Moxin Studio" src="moxin-studio-logo.png" />
</picture>

**A native desktop AI app built with pure Rust and [Makepad](https://github.com/makepad/makepad).**

Chat with local and cloud models, generate images, transcribe speech, and manage your model library — all without a Python runtime.

[Getting Started](#getting-started) | [Cloud Providers](#cloud-providers) | [Local Inference](#local-inference-setup)

</div>

---

## Getting Started

### Requirements

- macOS 14.0+ (Sonoma) on Apple Silicon (M1-M5)
- [Rust 1.82+](https://rustup.rs/)
- Xcode Command Line Tools (`xcode-select --install`)

### 1. Install OminiX-API

Moxin Studio uses [OminiX-API](https://github.com/OminiX-ai/OminiX-API) as the local inference server. Install it first:

```bash
curl -fsSL https://raw.githubusercontent.com/OminiX-ai/OminiX-API/main/install.sh | sh
```

This installs `ominix-api` to `/usr/local/bin` and creates `~/.OminiX/` for config and models.

### 2. Build and run Moxin Studio

```bash
git clone https://github.com/moxin-org/Moxin-Studio.git
cd Moxin-Studio
cargo run -p moly-shell --bin moxin-studio
```

The first build takes a few minutes to compile all dependencies. Subsequent runs are fast.

### 3. Download a model and start chatting

Open the **Model Hub** from the sidebar, click **Download** on any model, then click **Load**. Moxin Studio will auto-start OminiX-API and route your chat through it.

## Features

- **Local AI inference** — Run LLMs, vision models, image generation, speech recognition, and TTS directly on your Mac via OminiX-API
- **Model Hub** — Discover, download, and run models directly from the app
- **Voice I/O** — Speech-to-text and text-to-speech with voice cloning
- **MCP support** — Model Context Protocol for tool use
- **Chat history** — Persistent, searchable conversation history
- **Cloud fallback** — Optionally connect to cloud providers alongside local models

### Cloud Providers (Optional)

For models not available locally, you can add cloud API keys in Settings:

| Provider | What you get |
|----------|-------------|
| OpenAI | GPT-4o, DALL-E, Whisper |
| Anthropic | Claude Opus, Sonnet, Haiku |
| Google Gemini | Gemini Pro, Flash |
| DeepSeek | DeepSeek-V3, R1 |
| OpenRouter | Access to 100+ models |
| SiliconFlow | Cost-effective inference |
| Ollama | Local models via Ollama |

### Supported Local Models

Every model below has a dedicated, optimized implementation — not a generic wrapper. The pure Rust models run directly via [OminiX-MLX](https://github.com/OminiX-ai/OminiX-MLX) with Metal GPU acceleration.

#### LLM — Large Language Models

| Model | Implementation | Notes |
|-------|---------------|-------|
| Qwen3 | Pure Rust | 0.6B, 4B, 8B variants |
| Qwen3.5-27B | Pure Rust | Hybrid DeltaNet + Attention |
| GLM-4 | Pure Rust | |
| GLM-4.7-Flash | Pure Rust | MoE + MLA architecture |
| GLM-4.5 MoE | Pure Rust | Mixture of Experts |
| Mistral / Nemo | Pure Rust | |
| Mixtral | Pure Rust | MoE |
| MiniCPM-SALA | Pure Rust | Hybrid attention |

#### VLM — Vision Language Models

| Model | Implementation |
|-------|---------------|
| Qwen3-VL | Pure Rust |
| Moxin-7B | Pure Rust |
| DeepSeek-OCR-2 | Pure Rust |

#### ASR — Speech Recognition

| Model | Implementation | Notes |
|-------|---------------|-------|
| Qwen3-ASR | Pure Rust | 30+ languages |
| Paraformer (FunASR) | Pure Rust | |
| FunASR-Nano | Pure Rust | Lightweight |
| SenseVoice + Qwen3-4B | Pure Rust | LLM-enhanced ASR |

#### TTS — Text to Speech

| Model | Implementation | Notes |
|-------|---------------|-------|
| Qwen3-TTS | Pure Rust | Preset voices + voice cloning |
| GPT-SoVITS | Pure Rust | Zero-shot voice cloning |
| Step-Audio 2 | Pure Rust | |

#### Image Generation

| Model | Implementation | Notes |
|-------|---------------|-------|
| FLUX.2-klein | Pure Rust | Also available as GGUF |
| Z-Image-Turbo | Pure Rust | |
| Qwen-Image-2512 | Pure Rust | |
| Qwen-Image-Edit-2511 | Python MLX | Image editing |
| Cosmos Predict2 14B | Python MLX | Text-to-image |

#### Video Generation

| Model | Implementation | Notes |
|-------|---------------|-------|
| Wan2.2 5B | Python MLX | Text-to-video |

## The Moxin / OminiX Platform

Moxin Studio is the user-facing layer of a three-part pure Rust AI platform:

```
┌─────────────────────────────────────────────┐
│            Moxin Studio (this repo)         │  Desktop UI (Rust + Makepad)
│         Chat · Models · Voice · Settings    │
└──────────────────────┬──────────────────────┘
                       │ OpenAI-compatible REST/WS
┌──────────────────────▼──────────────────────┐
│               OminiX-API                    │  Local inference server (pure Rust)
│    LLM · ASR · TTS · Image endpoints       │
└──────────────────────┬──────────────────────┘
                       │ Rust crate interface
┌──────────────────────▼──────────────────────┐
│               OminiX-MLX                    │  On-device inference backend
│      Metal-accelerated · MLX framework      │  (Apple Silicon)
└─────────────────────────────────────────────┘
```

- [**OminiX-MLX**](https://github.com/OminiX-ai/OminiX-MLX) — Apple Silicon inference engine. Pure-Rust bindings to Apple's MLX framework with Metal GPU acceleration. Supports LLMs, VLMs, ASR, TTS, and image generation.
- [**OminiX-API**](https://github.com/OminiX-ai/OminiX-API) — Local inference server. OpenAI-compatible HTTP and WebSocket endpoints with dynamic model loading at runtime.
- **Moxin Studio** (this repo) — Desktop application. Connects to OminiX-API for local inference and cloud providers for remote models.

## Project Structure

```
Moxin-Studio/
├── moly-shell/          # Main application binary
├── moly-data/           # Shared state, persistence, API clients
├── moly-widgets/        # Reusable UI components and theming
└── apps/
    ├── moly-chat/       # Chat interface
    ├── moly-hub/        # Model Hub (discovery, download, load/unload)
    ├── moly-settings/   # Provider and API key configuration
    ├── moly-mcp/        # MCP server configuration
    └── moly-voice/      # Voice I/O
```

## License

[Apache 2.0](LICENSE)
