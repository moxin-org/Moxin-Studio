# Adding Models to Moxin Studio

## Overview

Models are registered in `moly-data/src/models_registry.json`. Each entry
describes where to download the weights, how to run the model, and how to
display it in the Model Hub UI.

---

## Step 1 – Add inference code to OminiX-MLX

Each model architecture needs a Rust crate in `OminiX-MLX/` that:
- Loads `.safetensors` weights using `mlx-rs`
- Implements the forward pass for that architecture
- Exposes it through `ominix-api` (OpenAI-compatible HTTP server)

If a model shares an architecture that already exists (e.g. another Qwen3
variant), no new crate is needed — only a new registry entry.

---

## Step 2 – Verify the HuggingFace repo

Before adding a registry entry, confirm the repo is usable:

```bash
# Check repo exists and list all files
curl "https://huggingface.co/api/models/<owner>/<repo>?blobs=true" | \
  python3 -c "import json,sys; d=json.load(sys.stdin); [print(s['rfilename'], s.get('size',0)) for s in d['siblings']]"
```

Common issues:
| HTTP code | Meaning | Fix |
|-----------|---------|-----|
| 200 | OK | Proceed |
| 401 | Gated / private repo | User must accept license at huggingface.co and add `~/.huggingface/hub/token` |
| 404 | Repo does not exist | Check the repo ID spelling |

> **Note:** The Studio download uses `?blobs=true` which returns ALL files
> recursively (including subdirectories). No special handling needed for
> nested layouts like `transformer/model.safetensors`.

---

## Step 3 – Add the registry entry

Edit `moly-data/src/models_registry.json` and append to the `models` array.

### Full schema

```json
{
  "id": "my-model-4b",
  "name": "My Model 4B",
  "description": "One-line description shown in the Hub list.",
  "category": "llm",
  "tags": ["llm", "chat", "fast"],

  "source": {
    "kind": "hugging_face",
    "repo_id": "mlx-community/My-Model-4B-4bit",
    "revision": "main"
  },

  "storage": {
    "local_path": "~/.cache/huggingface/hub/models--mlx-community--My-Model-4B-4bit",
    "size_bytes": 2500000000,
    "size_display": "~2.5 GB"
  },

  "runtime": {
    "api_type": "chat_completions",
    "api_model_id": "my-model-4b",
    "memory_gb": 4.0,
    "platforms": ["apple_silicon"],
    "supports_images": false,
    "supports_streaming": true,
    "quantization": "4bit"
  },

  "ui": {
    "panel_type": "llm_chat",
    "icon": "chat"
  }
}
```

### Field reference

| Field | Required | Notes |
|-------|----------|-------|
| `id` | ✓ | Unique slug, lowercase-hyphen. Used as the key everywhere. |
| `name` | ✓ | Display name shown in the list. |
| `description` | ✓ | Short sentence shown in the panel header. |
| `category` | ✓ | One of: `llm`, `vlm`, `asr`, `tts`, `image_gen` |
| `tags` | | Used for search filtering. |
| `source.kind` | ✓ | `hugging_face`, `model_scope`, or `manual` |
| `source.repo_id` | ✓ (HF) | `owner/repo` on HuggingFace |
| `source.revision` | | Default `"main"`. Use a commit SHA to pin a version. |
| `storage.local_path` | ✓ | Where weights are stored locally. Use `~/.cache/huggingface/hub/models--<owner>--<repo>` (replace `/` with `--`) for HF models. |
| `storage.size_bytes` | ✓ | Approximate total size in bytes (used for progress bar). |
| `storage.size_display` | ✓ | Human-readable string shown in UI. |
| `runtime.api_type` | ✓ | `chat_completions`, `speech_recognition`, `text_to_speech`, or `image_generation` |
| `runtime.api_model_id` | ✓ | The model ID sent to `ominix-api` (must match what the server expects). |
| `runtime.memory_gb` | ✓ | RAM required at inference time. Shown in panel header. |
| `runtime.supports_images` | | `true` for VLM models that accept image inputs. |
| `runtime.quantization` | | `"4bit"`, `"8bit"`, `"fp16"`, `"bf16"` |
| `ui.panel_type` | ✓ | Controls which panel is shown: `llm_chat`, `vlm_chat`, `asr`, `tts`, `image_gen` |
| `ui.icon` | | Icon hint for future use. |

### Category → panel_type mapping

| category | panel_type |
|----------|-----------|
| `llm` | `llm_chat` |
| `vlm` | `vlm_chat` |
| `asr` | `asr` |
| `tts` | `tts` |
| `image_gen` | `image_gen` |

---

## Step 4 – Add the model to OminiX-API

In `OminiX-MLX/ominix-api/src/`, register the model ID so the server knows
which inference crate to load when `/v1/models` lists it and when
`/v1/chat/completions` is called with that model ID.

---

## Model source kinds

### `hugging_face` (most common)
Downloads from `https://huggingface.co/{repo_id}/resolve/{revision}/{file}`.
Studio automatically lists all files via `?blobs=true`.

### `model_scope`
Downloads from ModelScope (Chinese mirror). Used for models not on HuggingFace.
Set `kind: "model_scope"` and `repo_id` to the ModelScope path.

### `manual`
No download button shown. User installs weights themselves.
Set `local_path` to wherever the user is expected to place the files.
Used for models with restrictive licenses (e.g. GPT-SoVITS).

---

## Current model status

| Model | Status | Notes |
|-------|--------|-------|
| qwen2-0.5b / 7b / 72b | ✓ Public | 72b needs ~39 GB |
| qwen3-0.6b / 8b / 14b | ✓ Public | |
| qwen3-72b | 🔒 Gated | Accept license at huggingface.co/mlx-community/Qwen3-72B-8bit |
| qwen3-235b-moe | ✓ Public | Needs ~123 GB |
| glm4-9b | 🔒 Gated | Accept license at huggingface.co/mlx-community/glm-4-9b-chat-4bit |
| glm4-moe-9b | ✓ Public | |
| mixtral-8x7b / 8x22b | ✓ Public | |
| mistral-7b | ✓ Public | |
| minicpm-sala-9b | 🔒 Gated | Accept license on HuggingFace |
| moxin-7b-vlm | 🔒 Gated | Accept license on HuggingFace |
| funasr-paraformer | ✓ Public | ModelScope source |
| funasr-nano | ✓ Public | |
| funasr-qwen4b | ✓ Public | |
| qwen3-asr-0.6b / 1.7b | ✓ Public | |
| gpt-sovits | Manual | User installs weights |
| flux-klein | ✓ Public | Needs ~22 GB |
| zimage-turbo | ✓ Public | Multi-directory layout |
