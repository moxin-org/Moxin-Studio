<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="moxin-studio-logo-dark.png">
  <img width="300" alt="Moxin Studio" src="moxin-studio-logo.png" />
</picture>

**A native desktop AI app built with pure Rust and [Makepad](https://github.com/makepad/makepad).**

Chat with local and cloud models, generate images, transcribe speech, and manage your model library — all without a Python runtime.

[Download](#download) | [Build from Source](#build-from-source) | [Local Inference Setup](#local-inference-setup) | [Cloud Providers](#cloud-providers)

</div>

---

## Download

Get the latest `.dmg` from the [Releases page](https://github.com/moxin-org/Moxin-Studio/releases).

1. Open the DMG and drag **Moxin Studio** to Applications
2. Before first launch, run in Terminal:
   ```bash
   xattr -cr /Applications/Moxin\ Studio.app
   ```
3. Open Moxin Studio from Applications

> **Requirements:** macOS 14.0+ (Sonoma) on Apple Silicon (M1-M5)

## Features

- **Multi-provider chat** — Local models via OminiX-API or Ollama; cloud via OpenAI, Anthropic, Gemini, DeepSeek, OpenRouter, SiliconFlow
- **Model Hub** — Discover, download, and run models directly. Supports LLM, VLM, ASR, TTS, and image generation
- **Image generation** — Local or cloud image endpoints
- **Voice I/O** — Speech-to-text and text-to-speech with voice cloning
- **MCP support** — Model Context Protocol for tool use
- **Chat history** — Persistent, searchable conversation history

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

## Cloud Providers

No local setup required — just open Settings and add your API keys:

| Provider | What you get |
|----------|-------------|
| OpenAI | GPT-4o, DALL-E, Whisper |
| Anthropic | Claude Opus, Sonnet, Haiku |
| Google Gemini | Gemini Pro, Flash |
| DeepSeek | DeepSeek-V3, R1 |
| OpenRouter | Access to 100+ models |
| SiliconFlow | Cost-effective inference |
| Ollama | Local models via Ollama |

## Local Inference Setup

To run models locally on your Mac, you need **OminiX-API** (the inference server). Moxin Studio connects to it automatically.

### 1. Install OminiX-API

```bash
curl -fsSL https://raw.githubusercontent.com/OminiX-ai/OminiX-API/main/install.sh | sh
```

This installs `ominix-api` to `/usr/local/bin` and creates `~/.OminiX/` for config and models.

### 2. Download a model from the Hub

Open Moxin Studio, go to the **Model Hub** (sidebar), and click **Download** on any model. Models are downloaded to `~/.OminiX/models/`.

### 3. Load and chat

Click **Load** on a downloaded model. Moxin Studio will auto-start OminiX-API and route your chat through it. No manual server management needed.

### Supported local model types

| Type | Examples |
|------|----------|
| LLM | Qwen3, GLM-4, Mistral, MiniCPM |
| VLM | Qwen3-VL (vision + language) |
| ASR | Paraformer, Qwen3-ASR |
| TTS | GPT-SoVITS (voice cloning) |
| Image | FLUX.2-klein, Z-Image-Turbo |

## Build from Source

<details>
<summary>For contributors and developers</summary>

### Requirements

- macOS 14.0+ (Sonoma) on Apple Silicon
- Rust 1.82+
- Xcode Command Line Tools (`xcode-select --install`)

### Clone and run

```bash
git clone https://github.com/moxin-org/Moxin-Studio.git
cd Moxin-Studio
cargo run -p moly-shell --bin moxin-studio
```

### Build the .app bundle

```bash
./build-and-run.sh
```

This compiles a release build, copies it into the `Moxin Studio.app` bundle, and launches it.

### Project structure

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

</details>

## License

[Apache 2.0](LICENSE)
