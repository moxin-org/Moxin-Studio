# Moxin Studio — MLX Model Integration Plan

## Vision

Moxin Studio becomes the native desktop hub for all MLX-accelerated models.
Users download, load, and interact with any model type (LLM, VLM, ASR, TTS, Image)
without touching a terminal. New models from ominix-mlx appear automatically via a
versioned JSON registry — zero code changes required.

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│  Moxin Studio (moly-shell)                                       │
│                                                                  │
│  Sidebar                   Main Content                          │
│  ──────────                ─────────────────────────────────     │
│  [Chat History]            Model Hub (new unified app)           │
│  [Model Hub]  ◄── NEW      ┌────────────┬─────────────────────┐  │
│  [Local Models]            │ Model List │ Type-Aware Panel    │  │
│  [Voice]                   │  (filter)  │                     │  │
│  [Settings]                │ ● LLM      │ Adapts based on     │  │
│                            │ ● VLM      │ selected model type │  │
│                            │ ● ASR      │                     │  │
│                            │ ● TTS      │ Download / Load /   │  │
│                            │ ● Image    │ Use controls        │  │
│                            └────────────┴─────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
           │
           ▼  REST API  (OpenAI-compatible where possible)
┌──────────────────────────────────────────────────────────────────┐
│  ominix-api  (localhost:8080)                                     │
│                                                                  │
│  /v1/chat/completions       ← LLM + VLM                          │
│  /v1/audio/transcriptions   ← ASR (Paraformer, FunASR-Nano)      │
│  /v1/audio/speech           ← TTS (GPT-SoVITS)                   │
│  /v1/images/generations     ← Image (FLUX.2-klein, Z-Image)      │
│  /v1/models                 ← List loaded models + status        │
│  /v1/models/{id}/load       ← Load model into memory             │
│  /v1/models/{id}/unload     ← Free memory                        │
└──────────────────────────────────────────────────────────────────┘
           │
           ▼  MLX / Rust inference
┌──────────────────────────────────────────────────────────────────┐
│  ominix-mlx crates                                               │
│  qwen3-mlx, glm4-mlx, mixtral-mlx, mistral-mlx                  │
│  moxin-vlm-mlx, funasr-mlx, funasr-nano-mlx                     │
│  gpt-sovits-mlx, flux-klein-mlx, zimage-mlx                     │
└──────────────────────────────────────────────────────────────────┘
```

---

## The Futureproof Core: JSON Model Registry

**The single source of truth for all model metadata.**

Location: `moly-data/src/models_registry.json` (bundled) +
          `~/.ominix/models_registry_override.json` (user override, fetched from server)

```json
{
  "version": "1.0.0",
  "models": [
    {
      "id": "qwen3-8b",
      "name": "Qwen3 8B",
      "description": "High-quality multilingual chat model, 8-bit quantized",
      "category": "LLM",
      "subcategory": "Chat",
      "size_bytes": 8589934592,
      "tags": ["multilingual", "reasoning", "code"],
      "source": {
        "type": "HuggingFace",
        "repo": "mlx-community/Qwen3-8B-8bit",
        "files": ["*.safetensors", "tokenizer*", "config.json"],
        "backup_urls": []
      },
      "storage": {
        "local_path": "~/.ominix/models/qwen3-8b",
        "expected_files": ["model.safetensors", "tokenizer.json"]
      },
      "runtime": {
        "api_type": "chat_completions",
        "model_id_for_api": "qwen3-8b",
        "memory_gb": 9.5,
        "platforms": ["apple_silicon"],
        "load_endpoint": "/v1/models/qwen3-8b/load",
        "unload_endpoint": "/v1/models/qwen3-8b/unload"
      },
      "ui": {
        "panel_type": "LLMChat",
        "color": "#6366f1",
        "icon": "chat"
      }
    },
    {
      "id": "moxin-7b-vlm",
      "name": "Moxin-7B VLM",
      "description": "Vision-language model — chat with images",
      "category": "VLM",
      "source": { "type": "HuggingFace", "repo": "moxin-org/moxin-vlm-7b-mlx" },
      "runtime": {
        "api_type": "chat_completions",
        "model_id_for_api": "moxin-7b-vlm",
        "supports_images": true
      },
      "ui": { "panel_type": "VLMChat", "color": "#8b5cf6" }
    },
    {
      "id": "funasr-paraformer",
      "name": "Paraformer ASR",
      "description": "Chinese/English speech recognition — 18x real-time",
      "category": "ASR",
      "runtime": {
        "api_type": "audio_transcription",
        "model_id_for_api": "paraformer-large"
      },
      "ui": { "panel_type": "ASRTranscription", "color": "#10b981" }
    },
    {
      "id": "gpt-sovits",
      "name": "GPT-SoVITS TTS",
      "description": "Few-shot voice cloning — 4x real-time",
      "category": "TTS",
      "runtime": {
        "api_type": "audio_speech",
        "model_id_for_api": "gpt-sovits"
      },
      "ui": { "panel_type": "TTSSynthesis", "color": "#f59e0b" }
    },
    {
      "id": "flux-klein",
      "name": "FLUX.2-klein",
      "description": "Fast image generation with Qwen3 text encoder",
      "category": "ImageGen",
      "runtime": {
        "api_type": "image_generation",
        "model_id_for_api": "flux-klein"
      },
      "ui": { "panel_type": "ImageGeneration", "color": "#ec4899" }
    }
  ]
}
```

**To add any new model: add one JSON entry. No Rust code changes required.**

---

## Phase 1 — Registry Foundation (Week 1)

### 1a. Extend `moly-data`

**File: `moly-data/src/model_registry.rs`** (new)

```rust
// The core registry types — JSON-driven, covers all model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub version: String,
    pub models: Vec<RegistryModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ModelCategory,     // LLM | VLM | ASR | TTS | ImageGen
    pub source: ModelSource,         // HuggingFace | ModelScope | DirectUrl | Local
    pub storage: ModelStorage,
    pub runtime: ModelRuntime,
    pub ui: ModelUiHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelCategory { LLM, VLM, ASR, TTS, ImageGen }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiType {
    ChatCompletions,      // OpenAI /v1/chat/completions
    AudioTranscription,   // OpenAI /v1/audio/transcriptions
    AudioSpeech,          // OpenAI /v1/audio/speech
    ImageGeneration,      // OpenAI /v1/images/generations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRuntime {
    pub api_type: ApiType,
    pub model_id_for_api: String,
    pub memory_gb: f32,
    pub platforms: Vec<String>,       // ["apple_silicon", "cuda", "cpu"]
    pub load_endpoint: Option<String>,
    pub unload_endpoint: Option<String>,
    pub supports_images: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUiHints {
    pub panel_type: PanelType,    // drives which UI panel to show
    pub color: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelType {
    LLMChat,
    VLMChat,
    ASRTranscription,
    TTSSynthesis,
    ImageGeneration,
}

impl ModelRegistry {
    /// Load bundled registry, then overlay user override from ~/.ominix/
    pub fn load() -> Self { ... }

    /// Check for updated registry from OminiX server (non-blocking, saves to override file)
    pub fn fetch_updates_async() { ... }

    /// Overlay another registry on top (for user overrides / server updates)
    pub fn merge(&mut self, other: ModelRegistry) { ... }
}
```

**File: `moly-data/src/download_manager.rs`** (extracted from local_models.rs)

```rust
// Generic download manager — works for any model from the registry
pub struct DownloadManager {
    pub states: HashMap<String, Arc<ModelDownloadState>>,
}

impl DownloadManager {
    pub fn start(&mut self, model: &RegistryModel) -> Result<()>;
    pub fn cancel(&mut self, model_id: &str);
    pub fn get_progress(&self, model_id: &str) -> Option<DownloadProgress>;
    pub fn is_downloaded(&self, model: &RegistryModel) -> bool;
}
```

### 1b. Bundle Default Registry

`moly-data/src/models_registry.json` — include all current models from ominix-mlx:

| Category | Model IDs |
|----------|-----------|
| LLM | qwen2-0.5b, qwen2-7b, qwen2-72b, qwen3-0.6b, qwen3-8b, qwen3-72b, qwen3-235b-moe, glm4-9b, glm4-moe-9b, mixtral-8x7b, mixtral-8x22b, mistral-7b, minicpm-sala-9b |
| VLM | moxin-7b-vlm |
| ASR | funasr-paraformer, funasr-nano |
| TTS | gpt-sovits |
| ImageGen | flux-klein, zimage-turbo |

---

## Phase 2 — Model Hub App (Week 2)

**New app: `apps/moly-hub/`** — replaces the current `moly-local-models` entirely
(or `moly-local-models` is extended to become the hub)

### Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ Model Hub                                             [↺ Update] │
├────────────────────┬─────────────────────────────────────────────┤
│ Search: [________] │                                             │
│                    │         RIGHT PANEL                         │
│ Filter: [All ▾]    │  (changes based on selected model type)     │
│  ○ LLM             │                                             │
│  ○ VLM             │  See Phase 3 for each panel type            │
│  ○ ASR             │                                             │
│  ○ TTS             │                                             │
│  ○ Image           │                                             │
│                    │                                             │
│ ── LLM ──          │                                             │
│ ● Qwen3 8B  [Use]  │                                             │
│ ○ Qwen3 72B [Get]  │                                             │
│ ○ GLM-4 9B  [Get]  │                                             │
│                    │                                             │
│ ── VLM ──          │                                             │
│ ○ Moxin-7B  [Get]  │                                             │
│                    │                                             │
│ ── ASR ──          │                                             │
│ ● Paraformer [Use] │                                             │
│ ○ FunASR Nano[Get] │                                             │
│                    │                                             │
│ ── TTS ──          │                                             │
│ ● GPT-SoVITS [Use] │                                             │
│                    │                                             │
│ ── Image ──        │                                             │
│ ○ FLUX.2    [Get]  │                                             │
│ ○ Z-Image   [Get]  │                                             │
└────────────────────┴─────────────────────────────────────────────┘
```

### State Machine

```rust
pub struct ModelHubApp {
    registry: ModelRegistry,
    download_manager: DownloadManager,
    selected_model_id: Option<String>,
    active_model_id: Option<String>,        // currently loaded in ominix-api
    filter_category: Option<ModelCategory>,
    search_query: String,
    model_states: HashMap<String, ModelUiState>,
}

pub enum ModelUiState {
    NotDownloaded,
    Downloading { progress: f32, file: String },
    Downloaded,
    Loading,   // being loaded into ominix-api memory
    Loaded,    // ready to use
    Error(String),
}
```

### App Registration in moly-shell

Following the existing 8-touch-point pattern:
1. `ICON_HUB` dep
2. `use moly_hub::screen::design::*`
3. `hub_btn` sidebar button
4. `hub_app = <ModelHubApp>` in main_content
5. `NavigationTarget::ModelHub` variant
6. String ↔ enum mappings
7. Visibility in `apply_view_state`
8. Button click handler

---

## Phase 3 — Type-Aware Right Panels (Week 3)

Each `PanelType` gets a distinct right-panel UI that renders in the Model Hub.

### Panel 1: LLMChat

```
┌─────────────────────────────────────────────────────────────────┐
│ Qwen3 8B                           [● Loaded] [Unload] [Chat ▸] │
│ LLM · 8B params · 8-bit · 9.5 GB RAM                            │
├─────────────────────────────────────────────────────────────────┤
│ Quick Test:                                                      │
│ [System prompt: ___________________________________________]     │
│ [User: ____________________________________________________]    │
│ [Max tokens: 512] [Temperature: 0.7]    [▶ Generate]            │
├─────────────────────────────────────────────────────────────────┤
│ Response:                                                        │
│ ┌───────────────────────────────────────────────────────────┐   │
│ │ (streaming output appears here)                           │   │
│ └───────────────────────────────────────────────────────────┘   │
│ [Open in Chat ▸]   (sends to moly-chat with this model active)  │
└─────────────────────────────────────────────────────────────────┘
```

**API call:** `POST /v1/chat/completions` with `model: "qwen3-8b"`, streaming

### Panel 2: VLMChat

Same as LLMChat + image upload button:

```
│ [📎 Upload Image]  [Current: my_photo.jpg ×]                    │
│ [User: Describe this image ___________________________]         │
```

**API call:** `POST /v1/chat/completions` with `content: [{type: image_url}, {type: text}]`

### Panel 3: ASRTranscription

```
┌─────────────────────────────────────────────────────────────────┐
│ Paraformer ASR                      [● Loaded] [Unload]         │
│ ASR · Chinese/English · 18x real-time                           │
├─────────────────────────────────────────────────────────────────┤
│ Input:                                                           │
│ [📎 Audio File: _______________ Browse]  or  [🎙 Record]        │
│ Language: [Auto ▾]    [▶ Transcribe]                            │
├─────────────────────────────────────────────────────────────────┤
│ Transcript:                                                      │
│ ┌───────────────────────────────────────────────────────────┐   │
│ │ (transcription appears here)                              │   │
│ └───────────────────────────────────────────────────────────┘   │
│ [📋 Copy]  [💾 Save .txt]                                        │
└─────────────────────────────────────────────────────────────────┘
```

**API call:** `POST /v1/audio/transcriptions` (multipart form: file + model)

### Panel 4: TTSSynthesis

```
┌─────────────────────────────────────────────────────────────────┐
│ GPT-SoVITS TTS                      [● Loaded] [Unload]         │
│ TTS · Few-shot voice cloning · 4x real-time                     │
├─────────────────────────────────────────────────────────────────┤
│ Voice: [voice1 ▾] [Manage Voices ▸]                             │
│ Speed: [0.5 ──●──── 2.0]                                         │
│ Text:                                                            │
│ ┌──────────────────────────────────────────────────────────┐    │
│ │ Enter text to speak...                                   │    │
│ └──────────────────────────────────────────────────────────┘    │
│ [▶ Generate & Play]  [💾 Save .wav]                              │
│ Status: Ready                                                    │
└─────────────────────────────────────────────────────────────────┘
```

**API call:** `POST /v1/audio/speech` → save WAV → `afplay`

### Panel 5: ImageGeneration

```
┌─────────────────────────────────────────────────────────────────┐
│ FLUX.2-klein                        [● Loaded] [Unload]         │
│ Image · Qwen3 text encoder · ~3s/image                          │
├─────────────────────────────────────────────────────────────────┤
│ Prompt: [_______________________________________________]        │
│ Neg. Prompt: [__________________________________________]        │
│ Size: [1024×1024 ▾]  Steps: [20]  CFG: [7.5]  Seed: [auto]     │
│                                            [🎨 Generate]        │
├─────────────────────────────────────────────────────────────────┤
│ ┌──────────────────────────────────────────────────────────┐    │
│ │                                                          │    │
│ │              (generated image displayed here)            │    │
│ │                                                          │    │
│ └──────────────────────────────────────────────────────────┘    │
│ [💾 Save PNG]  [🔁 Regenerate]  [📋 Copy Prompt]                 │
└─────────────────────────────────────────────────────────────────┘
```

**API call:** `POST /v1/images/generations` → render PNG bytes via Makepad Image widget

---

## Phase 4 — Load/Unload Manager (Week 4)

Models must be explicitly loaded into ominix-api memory before use.

### API Contracts

```
GET  /v1/models
→ { "data": [{ "id": "qwen3-8b", "status": "loaded", "memory_gb": 9.5 }, ...] }

POST /v1/models/{id}/load
→ 200 OK when loaded
→ Progress via SSE: data: {"progress": 0.65, "stage": "loading weights"}

POST /v1/models/{id}/unload
→ 200 OK when freed
```

### Load/Unload UX

- Only **one model per category** can be loaded at a time (memory constraint)
- Loading shows progress bar (weights loading is slow for large models)
- Unloading is instant
- Downloaded-but-not-loaded models show `[Load]` button
- Loaded models show `[● Loaded]` badge + `[Unload]` button
- Loading in progress shows `[Loading... 65%]`

### Memory Guard

```rust
impl ModelHubApp {
    fn can_load_model(&self, model: &RegistryModel) -> Result<(), String> {
        // Check if enough RAM available
        let available = self.get_available_memory_gb();
        if model.runtime.memory_gb > available {
            return Err(format!(
                "Needs {:.1} GB, only {:.1} GB available. Unload another model first.",
                model.runtime.memory_gb, available
            ));
        }
        Ok(())
    }
}
```

---

## Phase 5 — Chat Integration for LLM/VLM (Week 5)

When user clicks "Open in Chat ▸" from the Model Hub:

1. Set the active model in `Store` (`store.set_active_local_model("qwen3-8b")`)
2. Navigate to `NavigationTarget::ActiveChat`
3. `ChatApp` picks up the active model and routes messages to `localhost:8080/v1/chat/completions`
   with the local model ID instead of a cloud provider

This reuses the existing `moly-chat` app entirely — no new chat UI needed.

**Required changes in moly-data:**
```rust
impl Store {
    pub fn set_active_local_model(&mut self, model_id: Option<String>);
    pub fn get_active_local_model(&self) -> Option<&str>;
}
```

**Required changes in moly-chat:**
- When `store.get_active_local_model()` is Some, use `localhost:8080` as base URL
- Model dropdown shows local model name with `[local]` badge
- Falls back to cloud providers when None

---

## Futureproofing Strategy

### Adding a New Model (Zero Code Changes)

1. Add entry to `models_registry.json`
2. ominix-api implements the inference
3. Studio auto-discovers it on next launch

**Example: Adding Llama-4 next month**

```json
{
  "id": "llama4-scout-17b",
  "name": "Llama 4 Scout 17B",
  "category": "LLM",
  "source": {
    "type": "HuggingFace",
    "repo": "mlx-community/Llama-4-Scout-17B-8bit"
  },
  "runtime": {
    "api_type": "ChatCompletions",
    "model_id_for_api": "llama4-scout-17b",
    "memory_gb": 18.0
  },
  "ui": { "panel_type": "LLMChat", "color": "#3b82f6" }
}
```

Done. The model appears in the LLM section, can be downloaded and used.

### Adding a New Model Category (Minimal Code)

If a new modality appears (e.g., video generation, 3D generation):

1. Add variant to `ModelCategory` enum in `moly-data`
2. Add variant to `PanelType` enum
3. Implement one new Makepad widget for the right panel
4. Add to match arms in Model Hub's `draw_walk`

That's ~200 lines of Rust + a JSON registry entry.

### Registry Server Updates

```rust
impl ModelRegistry {
    /// On app launch, fetch in background — no blocking startup
    pub fn fetch_updates_async() {
        std::thread::spawn(|| {
            let url = "https://registry.ominix.ai/models_registry.json";
            if let Ok(resp) = reqwest::blocking::get(url) {
                if let Ok(registry) = resp.json::<ModelRegistry>() {
                    registry.save_to_override(); // ~/.ominix/models_registry_override.json
                }
            }
        });
    }
}
```

Next launch: override file merges on top of bundled registry. New models appear without an app update.

---

## Implementation Sequence

### Phase 1 — Registry (Week 1)
- [ ] `moly-data/src/model_registry.rs` — registry types + JSON load/save
- [ ] `moly-data/src/models_registry.json` — all current models
- [ ] `moly-data/src/download_manager.rs` — generic downloader extracted from local_models.rs
- [ ] Wire into `moly-data/src/lib.rs`

### Phase 2 — Model Hub Scaffold (Week 2)
- [ ] `apps/moly-hub/Cargo.toml`
- [ ] `apps/moly-hub/src/lib.rs` — MolyHubApp struct + impl MolyApp
- [ ] `apps/moly-hub/src/screen/design.rs` — split-panel shell (left list + right placeholder)
- [ ] `apps/moly-hub/src/screen/mod.rs` — list rendering, filter, download wiring
- [ ] Register in `moly-shell` (8 touch points)

### Phase 3 — Right Panels (Week 3)
- [ ] `LLMChatPanel` widget + API client (`POST /v1/chat/completions`)
- [ ] `VLMChatPanel` widget + image file picker
- [ ] `ASRPanel` widget + audio file picker + API client
- [ ] `TTSPanel` widget + API client (reuse voice app logic)
- [ ] `ImagePanel` widget + image renderer + API client

### Phase 4 — Load/Unload (Week 4)
- [ ] `moly-data/src/model_runtime_client.rs` — load/unload/status API client
- [ ] Load progress panel + memory guard in Model Hub
- [ ] Status polling via background thread + mpsc

### Phase 5 — Chat Integration (Week 5)
- [ ] `Store::set_active_local_model` / `get_active_local_model`
- [ ] `moly-chat` routes to local endpoint when local model is active
- [ ] "Open in Chat" button in LLMChat + VLMChat panels

---

## File Map (Complete)

```
moly-data/
  src/
    model_registry.rs         ← NEW: registry types, JSON load, merge, server fetch
    models_registry.json      ← NEW: all model definitions (bundled)
    download_manager.rs       ← NEW: generic downloader (refactored from local_models.rs)
    model_runtime_client.rs   ← NEW: load/unload/status HTTP client
    local_models.rs           ← KEEP: backwards compat, wraps registry
    lib.rs                    ← MODIFY: export new modules

apps/
  moly-hub/
    Cargo.toml                ← NEW
    src/
      lib.rs                  ← NEW: MolyHubApp impl MolyApp
      screen/
        design.rs             ← NEW: live_design! for all panels
        mod.rs                ← NEW: state machine + panel routing
        panels/
          llm_chat.rs         ← NEW: LLMChatPanel widget
          vlm_chat.rs         ← NEW: VLMChatPanel widget
          asr.rs              ← NEW: ASRPanel widget
          tts.rs              ← NEW: TTSPanel widget (port from moly-voice)
          image_gen.rs        ← NEW: ImageGenPanel widget

moly-shell/
  src/
    app.rs                    ← MODIFY: +hub_btn, +ModelHub nav, +8 touch points
  Cargo.toml                  ← MODIFY: +moly-hub dep
  resources/
    icons/
      hub.svg                 ← NEW: model hub icon
```

---

## API Contracts Summary

All calls go to `http://localhost:8080` (ominix-api).

| Panel | Endpoint | Method | Body | Response |
|-------|----------|--------|------|----------|
| LLM/VLM | `/v1/chat/completions` | POST | `{model, messages, stream}` | SSE chunks |
| ASR | `/v1/audio/transcriptions` | POST | multipart: `file`, `model` | `{text}` |
| TTS | `/v1/audio/speech` | POST | `{model, input, voice, speed}` | WAV bytes |
| Image | `/v1/images/generations` | POST | `{model, prompt, n, size}` | `{data: [{b64_json}]}` |
| Load | `/v1/models/{id}/load` | POST | `{}` | SSE progress |
| Unload | `/v1/models/{id}/unload` | POST | `{}` | `200 OK` |
| Status | `/v1/models` | GET | — | `{data: [{id, status}]}` |

All endpoints are OpenAI-compatible where the modality allows it.
Custom extensions (load/unload/status) follow OpenAI naming conventions.
