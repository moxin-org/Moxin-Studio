# Plan: Voice Cloning Frontend for Moxin Studio

## Context

The gpt-sovits-mlx voice cloning engine is fully implemented in OminiX-MLX and already wired into OminiX-API (`gpt-sovits-mlx` is a path dep of OminiX-API). The API already exposes complete endpoints for voice training, listing, and TTS synthesis. The `moly-voice` app in Studio is currently a build stub with no UI. This plan builds the complete voice cloning frontend inside that stub.

**No API changes needed** — all endpoints already exist and are registered in the router.

---

## Existing API Endpoints (Already Live at localhost:8080)

| Endpoint | Purpose |
|----------|---------|
| `GET /v1/voices` | List registered voices from `~/.OminiX/models/voices.json` |
| `POST /v1/voices/train` | Upload ref audio (base64) + transcript → start training, returns `{task_id}` |
| `GET /v1/voices/train/status?task_id=X` | Poll: returns stage + progress (0.0–1.0) |
| `POST /v1/voices/train/cancel` | Cancel in-progress training |
| `POST /v1/audio/speech` | Synthesize: `{input, voice, speed, response_format:"wav"}` → binary WAV |

Training request: `{ voice_name, audio (base64 WAV), transcript, quality: "fast"|"standard"|"high", language: "zh"|"en"|"auto", denoise: bool }` — 50 MB JSON limit.

---

## UI Layout (Split-Panel, following moly-local-models pattern)

```
┌──────────────────┬───────────────────────────────────────────┐
│  Voices (260px)  │  Right Panel                              │
│──────────────────│───────────────────────────────────────────│
│ [+ New Voice]    │  ── Train New Voice ──                    │
│                  │  Voice Name: [___________]                 │
│ ● doubao         │  Audio File: [/path/to/ref.wav_______]    │
│ ○ custom1        │  Transcript: [____________________]       │
│                  │  Quality:  [Standard ▼]                   │
│                  │  Language: [Auto ▼]    Denoise: [✓]       │
│                  │  [ Upload & Train ]  [ Cancel ]           │
│                  │                                           │
│                  │  Stage: Feature Extraction  ████░░  62%   │
│                  │                                           │
│                  │  ── Synthesize ──                         │
│                  │  Voice: doubao   Speed: [1.0x ▼]          │
│                  │  ┌──────────────────────────────────┐     │
│                  │  │ Enter text to synthesize...      │     │
│                  │  └──────────────────────────────────┘     │
│                  │  [ Generate ]  [ ▶ Play ]                 │
│                  │  Ready — 2.3s generated                   │
└──────────────────┴───────────────────────────────────────────┘
```

- **Left panel** — voice list fetched from `GET /v1/voices`. Green dot = ready, gray = not trained.
- **"+ New Voice"** button — clears the right panel form for entering a new voice.
- **Right panel** — training section (top) and synthesis section (bottom). Training uses the currently-entered voice name; synthesis uses the selected voice from the left list.
- **Audio File path** — typed/pasted text input (no system file picker in Makepad).
- **Play** — saves generated WAV to `/tmp/ominix-voice-out.wav`, calls `afplay` (macOS).

---

## State Design (`mod.rs`)

```rust
enum TrainingState {
    Idle,
    Training { task_id: String, stage: String, progress: f32 },
    Done,
    Error(String),
}

enum SynthesisState {
    Idle,
    Generating,
    Done { duration_secs: f32 },
    Error(String),
}

struct VoiceEntry { name: String, is_ready: bool }

// Channels for background threads
enum TrainingUpdate { Progress { stage: String, progress: f32 }, Done, Error(String) }
enum SynthesisUpdate { Done { duration_secs: f32 }, Error(String) }
```

---

## Files to Create / Modify

### Create
| File | What |
|------|------|
| `apps/moly-voice/src/screen/design.rs` | Complete `live_design!{}` widget tree |
| `apps/moly-voice/src/screen/mod.rs` | Widget struct + handle_event + draw_walk |
| `moly-shell/resources/icons/voice.svg` | Microphone SVG icon for sidebar button |

### Modify
| File | Change |
|------|--------|
| `apps/moly-voice/src/lib.rs` | Replace stub with full `MolyApp` impl |
| `apps/moly-voice/Cargo.toml` | Add `reqwest` (with `blocking`, `json`), `base64`, `serde_json` deps |
| `moly-shell/Cargo.toml` | Add `moly-voice = { path = "../apps/moly-voice" }` |
| `moly-shell/src/app.rs` | 7 touch points (see below) |

---

## Exact Touch Points in `moly-shell/src/app.rs`

1. **Icon constant** (top of `live_design!{}`, ~line 32):
   ```rust
   ICON_VOICE = dep("crate://self/resources/icons/voice.svg")
   ```

2. **Import screen** (~line 23, inside `live_design!{}`):
   ```rust
   use moly_voice::screen::design::*;
   ```

3. **Sidebar button** (after `local_models_btn`, ~line 335):
   ```rust
   voice_btn = <SidebarButton> {
       text: "Voice"
       draw_icon: { svg_file: (ICON_VOICE) }
   }
   ```

4. **Widget instance in main content** (after `local_models_app`, ~line 596):
   ```rust
   voice_app = <VoiceApp> {}
   ```

5. **`live_register` call** (~line 709):
   ```rust
   <moly_voice::MolyVoiceApp as MolyApp>::live_design(cx);
   ```

6. **`NavigationTarget` enum** (~line 617):
   ```rust
   Voice,
   ```

7. **`navigate_to` / `apply_view_state`** (6 sub-changes):
   - Add `"Voice" => NavigationTarget::Voice` mapping (~line 674)
   - Add `NavigationTarget::Voice => "Voice"` (~line 955)
   - Add `set_visible` for `voice_app` in `apply_view_state`
   - Handle `voice_btn` click → `navigate_to(cx, NavigationTarget::Voice)` (~line 781 area)

---

## Key Implementation Notes

### HTTP — background thread pattern (same as local-models downloader)
All reqwest calls happen inside `std::thread::spawn`. Results returned via `std::sync::mpsc::channel`. `handle_event` checks `rx.try_recv()` each frame and calls `cx.new_next_frame()` while in-progress.

### Training flow
1. Read bytes from `ref_audio_path`, base64-encode
2. POST `/v1/voices/train` → get `task_id`
3. Thread polls `GET /v1/voices/train/status?task_id=X` every 500ms
4. Push `TrainingUpdate::Progress` into channel → redraw progress bar
5. On completion: re-fetch `GET /v1/voices` to refresh left panel voice list

### Synthesis flow
1. POST `/v1/audio/speech` → raw WAV bytes in response body
2. Write to `/tmp/ominix-voice-out.wav`
3. Push `SynthesisUpdate::Done` → enable Play button

### Audio playback
```rust
std::process::Command::new("afplay")
    .arg("/tmp/ominix-voice-out.wav")
    .spawn().ok();
```

### reqwest blocking calls in thread
Use `reqwest::blocking::Client` (simpler than async inside `std::thread::spawn`). Add `features = ["blocking", "json"]` to the dep in `Cargo.toml`.

---

## Verification

1. `cargo check -p moly-voice` — compiles clean
2. `cargo run` (Studio debug) — window opens with "Voice" button in sidebar
3. Click Voice → panel appears with empty voice list and training form
4. Paste a `.wav` path, enter a voice name + transcript, click **Upload & Train** — progress bar advances through stages
5. Training completes → voice appears in left list with green dot
6. Select voice, type text, click **Generate** — WAV synthesized
7. Click **Play** — audio plays via `afplay`
