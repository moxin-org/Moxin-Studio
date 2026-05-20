pub mod design;

use makepad_widgets::*;
use moly_data::{
    ModelRegistry, RegistryModel, RegistryCategory, SourceKind, PanelType,
    ModelRuntimeClient, ServerModelInfo, ServerModelStatus,
    StoreAction,
    ensure_server_running,
};
use serde::Deserialize;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::sync::mpsc;

use base64::Engine as _;
use rfd::FileDialog;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Strip GGUF quant suffixes to get a base model name for grouping.
/// e.g. "Qwen3 4B (Q4_K_M)" → "Qwen3 4B", "Qwen3 4B (Q8)" → "Qwen3 4B"
fn strip_quant_suffix(name: &str) -> String {
    // Strip trailing " (Q4_K_M)", " (Q4_K_S)", " (Q8)", " (FP16)", etc.
    if let Some(idx) = name.rfind(" (Q") {
        return name[..idx].to_string();
    }
    if let Some(idx) = name.rfind(" (FP") {
        return name[..idx].to_string();
    }
    if let Some(idx) = name.rfind(" (BF") {
        return name[..idx].to_string();
    }
    name.to_string()
}

// ─── List row ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum ListRow {
    Header(RegistryCategory),
    SubHeader(usize), // index into ModelHubApp::subfolder_names
    Model(usize),     // index into registry.models
    VoiceStudio,      // always-visible footer entry
}

// ─── Filter ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum Filter {
    #[default]
    All,
    Cat(RegistryCategory),
}

// ─── Download state ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum ModelUiState {
    NotDownloaded,
    Downloading,
    Downloaded,
    Error,
}

impl ModelUiState {
    fn dot_value(self) -> f64 {
        match self {
            Self::NotDownloaded => 0.0,
            Self::Downloading   => 1.0,
            Self::Downloaded    => 2.0,
            Self::Error         => 5.0, // red (above blue at 3.0)
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::NotDownloaded => "Not Downloaded",
            Self::Downloading   => "Downloading...",
            Self::Downloaded    => "Downloaded",
            Self::Error         => "Error",
        }
    }
}

// ─── Load state ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum ModelLoadState {
    #[default]
    Unloaded,
    Loading,
    Loaded,
    LoadError,
}

// ─── Combined dot / status helpers ────────────────────────────────────────────

fn combined_dot_value(dl: ModelUiState, load: ModelLoadState) -> f64 {
    match load {
        ModelLoadState::Loaded    => 3.0,
        ModelLoadState::Loading   => 2.5,
        ModelLoadState::LoadError => 5.0,
        ModelLoadState::Unloaded  => dl.dot_value(),
    }
}

fn combined_status_label(dl: ModelUiState, load: ModelLoadState) -> &'static str {
    match load {
        ModelLoadState::Loaded    => "Loaded  ●",
        ModelLoadState::Loading   => "Loading...",
        ModelLoadState::LoadError => "Load Error",
        ModelLoadState::Unloaded  => dl.label(),
    }
}

// ─── Active panel ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum ActivePanel {
    #[default]
    None,
    Llm, Vlm, Asr, Tts, Image, ImageEdit, Video, Voice, Info,
}

// ─── Per-panel interaction state ─────────────────────────────────────────────

#[derive(Default)]
struct LlmState {
    system: String, user: String, response: String,
    is_running: bool,
    rx: Option<mpsc::Receiver<Result<String, String>>>,
}

#[derive(Default)]
struct VlmState {
    image_path: String, user: String, response: String,
    is_running: bool,
    rx: Option<mpsc::Receiver<Result<String, String>>>,
}

#[derive(Default)]
struct AsrState {
    audio_path: String, transcript: String,
    is_running: bool,
    rx: Option<mpsc::Receiver<Result<String, String>>>,
}

#[derive(Default)]
struct TtsState {
    voice_id: String, text: String, voices: Vec<String>,
    output_path: String,
    is_running: bool,
    rx:        Option<mpsc::Receiver<Result<(), String>>>,
    voices_rx: Option<mpsc::Receiver<Result<Vec<String>, String>>>,
}

struct TtsVoiceEntry { id: &'static str, label: &'static str, is_chinese: bool }

static TTS_PRESET_VOICES: &[TtsVoiceEntry] = &[
    // English voices
    TtsVoiceEntry { id: "vivian",        label: "Vivian",         is_chinese: false },
    TtsVoiceEntry { id: "serena",        label: "Serena",         is_chinese: false },
    TtsVoiceEntry { id: "ryan",          label: "Ryan",           is_chinese: false },
    TtsVoiceEntry { id: "aiden",         label: "Aiden",          is_chinese: false },
    TtsVoiceEntry { id: "english_man",   label: "English Male",   is_chinese: false },
    // Chinese voices
    TtsVoiceEntry { id: "uncle_fu",      label: "Uncle Fu",       is_chinese: true  },
    TtsVoiceEntry { id: "chinese_woman", label: "Chinese Female", is_chinese: true  },
    TtsVoiceEntry { id: "chinese_man",   label: "Chinese Male",   is_chinese: true  },
    TtsVoiceEntry { id: "dialect",       label: "Dialect",        is_chinese: true  },
];

#[derive(Default)]
struct ImageState {
    prompt: String, neg_prompt: String, output_path: String,
    is_running: bool,
    rx: Option<mpsc::Receiver<Result<String, String>>>,
}

#[derive(Default)]
struct ImageEditState {
    image_path: String, prompt: String, output_path: String,
    is_running: bool,
    rx: Option<mpsc::Receiver<Result<String, String>>>,
}

#[derive(Default)]
struct VideoState {
    prompt: String, output_path: String,
    is_running: bool,
    rx: Option<mpsc::Receiver<Result<String, String>>>,
}

// ─── Model download state ─────────────────────────────────────────────────────

#[derive(Clone)]
struct ModelDownloadState {
    is_downloading:   Arc<AtomicBool>,
    cancel_requested: Arc<AtomicBool>,
    progress_bytes:   Arc<AtomicU64>,
    total_bytes:      Arc<AtomicU64>,
    current_file:     Arc<std::sync::Mutex<String>>,
    completed:        Arc<AtomicBool>,
    failed:           Arc<AtomicBool>,
    error_msg:        Arc<std::sync::Mutex<String>>,
}

impl ModelDownloadState {
    fn new() -> Self {
        Self {
            is_downloading:   Arc::new(AtomicBool::new(false)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            progress_bytes:   Arc::new(AtomicU64::new(0)),
            total_bytes:      Arc::new(AtomicU64::new(0)),
            current_file:     Arc::new(std::sync::Mutex::new(String::new())),
            completed:        Arc::new(AtomicBool::new(false)),
            failed:           Arc::new(AtomicBool::new(false)),
            error_msg:        Arc::new(std::sync::Mutex::new(String::new())),
        }
    }
    fn reset(&self) {
        self.is_downloading.store(false, Ordering::SeqCst);
        self.cancel_requested.store(false, Ordering::SeqCst);
        self.progress_bytes.store(0, Ordering::SeqCst);
        self.total_bytes.store(0, Ordering::SeqCst);
        self.completed.store(false, Ordering::SeqCst);
        self.failed.store(false, Ordering::SeqCst);
        *self.current_file.lock().unwrap() = String::new();
        *self.error_msg.lock().unwrap() = String::new();
    }
    fn fraction(&self) -> f64 {
        let total = self.total_bytes.load(Ordering::SeqCst);
        if total == 0 { return 0.0; }
        (self.progress_bytes.load(Ordering::SeqCst) as f64 / total as f64).min(1.0)
    }
    fn progress_text(&self) -> String {
        let done  = self.progress_bytes.load(Ordering::SeqCst);
        let total = self.total_bytes.load(Ordering::SeqCst);
        let file  = self.current_file.lock().unwrap().clone();
        let pct   = self.fraction() * 100.0;
        let done_mb  = done  / 1_048_576;
        let total_mb = total / 1_048_576;
        if file.is_empty() {
            format!("{:.1}%  ({}/{} MB)", pct, done_mb, total_mb)
        } else {
            format!("{:.1}%  ({}/{} MB)  {}", pct, done_mb, total_mb, file)
        }
    }
}

// ─── Voice Studio types ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct VoiceEntry {
    name:     String,
    is_ready: bool,
}

#[derive(Default)]
enum VoiceTrainingState {
    #[default]
    Idle,
    Training { task_id: String, stage: String, progress: f32 },
    Done,
    Error(String),
}

#[derive(Default)]
enum VoiceSynthesisState {
    #[default]
    Idle,
    Generating,
    Done { duration_secs: f32 },
    Error(String),
}

enum VoicesUpdate {
    Loaded(Vec<VoiceEntry>),
    Error(String),
}

enum VoiceTrainingUpdate {
    Progress { stage: String, progress: f32 },
    Done,
    Error(String),
}

enum VoiceSynthesisUpdate {
    Done { duration_secs: f32 },
    Error(String),
}

// ─── HF / MS API response types ───────────────────────────────────────────────

#[derive(Deserialize)]
struct HfBlobsResponse {
    siblings: Vec<HfSibling>,
}
#[derive(Deserialize)]
struct HfSibling {
    rfilename: String,
    size: Option<u64>,
}
#[derive(Deserialize)]
struct MsResponse {
    #[serde(rename = "Code")] code: i32,
    #[serde(rename = "Data")] data: Option<MsData>,
}
#[derive(Deserialize)]
struct MsData {
    #[serde(rename = "Files")] files: Vec<MsFile>,
}
#[derive(Deserialize)]
struct MsFile {
    #[serde(rename = "Path")] path: String,
    #[serde(rename = "Size")] size: u64,
    #[serde(rename = "Type")] file_type: String,
}

// ─── Widget ───────────────────────────────────────────────────────────────────

live_design! {
    use link::theme::*;
    use link::widgets::*;
    use crate::screen::design::*;
}

#[derive(Live, LiveHook, Widget)]
pub struct ModelHubApp {
    #[deref] view: View,

    /// 0 = All (standalone hub), 1 = LLM, 2 = VLM, 3 = ASR, 4 = TTS, 5 = Image
    /// Set from live_design in the shell to lock this instance to a category.
    #[live] hub_category: f64,

    // ── Core data ───────────────────────────────────────────────────────────
    #[rust] registry:        Option<ModelRegistry>,
    #[rust] initialized:     bool,
    #[rust] filter:          Filter,
    #[rust] search_query:    String,
    #[rust] selected_id:     Option<String>,
    #[rust] flat_list:       Vec<ListRow>,
    #[rust] subfolder_names: Vec<String>,

    // ── Download tracking ───────────────────────────────────────────────────
    #[rust] model_states:    HashMap<String, ModelUiState>,
    #[rust] download_states: HashMap<String, ModelDownloadState>,

    // ── Load / Unload tracking ──────────────────────────────────────────────
    #[rust] load_states:      HashMap<String, ModelLoadState>,
    /// Receivers for in-flight load operations (key = registry model ID)
    #[rust] load_rxs:         HashMap<String, mpsc::Receiver<Result<(), String>>>,
    /// Receivers for in-flight unload operations
    #[rust] unload_rxs:       HashMap<String, mpsc::Receiver<Result<(), String>>>,
    /// One-shot: GET /v1/models to sync server state
    #[rust] server_status_rx: Option<mpsc::Receiver<Result<Vec<ServerModelInfo>, String>>>,

    // ── Panel state ─────────────────────────────────────────────────────────
    #[rust] active_panel: ActivePanel,
    #[rust] llm_state:    LlmState,
    #[rust] vlm_state:    VlmState,
    #[rust] asr_state:    AsrState,
    #[rust] tts_state:    TtsState,
    #[rust] selected_tts_voice_idx: usize,
    #[rust] image_state:  ImageState,
    #[rust] image_edit_state: ImageEditState,
    #[rust] video_state:  VideoState,

    // ── Remove confirmation ──────────────────────────────────────────────────
    /// Model ID pending removal confirmation (first click sets this, second click confirms)
    #[rust] pending_remove_id: Option<String>,

    // ── Resizable split pane ─────────────────────────────────────────────────
    /// Width of the left panel in pixels; 0.0 means not yet initialized
    #[rust] left_panel_width:    f64,
    /// (start_mouse_x, start_panel_width) captured on FingerDown on the divider
    #[rust] drag_start:          Option<(f64, f64)>,

    // ── Voice Studio state ───────────────────────────────────────────────────
    #[rust] voices:              Vec<VoiceEntry>,
    #[rust] selected_voice_idx:  Option<usize>,
    #[rust] voice_training_state: VoiceTrainingState,
    #[rust] voice_synthesis_state: VoiceSynthesisState,
    #[rust] voice_quality:       String,
    #[rust] voice_language:      String,
    #[rust] voice_denoise:       bool,
    #[rust] voice_training_rx:   Option<mpsc::Receiver<VoiceTrainingUpdate>>,
    #[rust] voice_synthesis_rx:  Option<mpsc::Receiver<VoiceSynthesisUpdate>>,
    #[rust] voice_list_rx:       Option<mpsc::Receiver<VoicesUpdate>>,
    #[rust] voice_cancel:        Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    #[rust] voice_task_id:       String,
}

impl Widget for ModelHubApp {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.initialized { self.initialize(cx); }

        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));

        self.handle_search(&actions, cx);
        self.handle_list_clicks(cx, &actions);
        self.handle_panel_header_buttons(cx, &actions);
        self.handle_load_buttons(cx, &actions);
        self.handle_chat_button(cx, &actions, scope);
        self.handle_input_changes(&actions);
        self.handle_llm_actions(cx, &actions);
        self.handle_vlm_actions(cx, &actions);
        self.handle_asr_actions(cx, &actions);
        self.handle_tts_actions(cx, &actions);
        self.handle_image_actions(cx, &actions);
        self.handle_image_edit_actions(cx, &actions);
        self.handle_video_actions(cx, &actions);
        self.handle_voice_actions(cx, &actions);

        self.poll_downloads(cx);
        self.poll_load_channels(cx);
        self.poll_panel_channels(cx);
        self.check_server_status_result(cx);
        self.poll_voice_channels(cx);

        // ── Resizable divider drag ────────────────────────────────────────────
        let divider_area = self.view.view(ids!(hub_main_divider)).area();
        match event.hits(cx, divider_area) {
            Hit::FingerHoverIn(_) | Hit::FingerHoverOver(_) => {
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerDown(f) => {
                self.drag_start = Some((f.abs.x, self.left_panel_width));
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerMove(f) => {
                if let Some((start_x, start_w)) = self.drag_start {
                    let new_w = (start_w + f.abs.x - start_x).max(160.0).min(600.0);
                    self.left_panel_width = new_w;
                    // Apply width to the left panel once per drag event (NOT in draw_walk,
                    // which would invalidate GPU buffers every frame)
                    self.view.view(ids!(hub_left_panel)).apply_over(cx, live! { width: (new_w) });
                    self.view.redraw(cx);
                }
                cx.set_cursor(MouseCursor::ColResize);
            }
            Hit::FingerUp(_) | Hit::FingerHoverOut(_) => {
                self.drag_start = None;
            }
            _ => {}
        }

        // ── VLM image drop zone (drag-and-drop from Finder) ─────────────────
        let drop_zone_area = self.view.view(ids!(hub_vlm_panel.vlm_drop_zone)).area();
        match event.drag_hits(cx, drop_zone_area) {
            DragHit::Drag(e) => {
                match e.state {
                    DragState::In | DragState::Over => {
                        *e.response.lock().unwrap() = DragResponse::Copy;
                        self.view.view(ids!(hub_vlm_panel.vlm_drop_zone)).apply_over(cx, live! { draw_bg: { drag_over: (1.0) } });
                        self.view.redraw(cx);
                    }
                    DragState::Out => {
                        self.view.view(ids!(hub_vlm_panel.vlm_drop_zone)).apply_over(cx, live! { draw_bg: { drag_over: (0.0) } });
                        self.view.redraw(cx);
                    }
                }
            }
            DragHit::Drop(e) => {
                self.view.view(ids!(hub_vlm_panel.vlm_drop_zone)).apply_over(cx, live! { draw_bg: { drag_over: (0.0) } });
                for item in e.items.iter() {
                    if let DragItem::FilePath { path, .. } = item {
                        let lower = path.to_lowercase();
                        if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
                            || lower.ends_with(".bmp") || lower.ends_with(".gif") || lower.ends_with(".webp") {
                            self.vlm_state.image_path = path.clone();
                            self.view.text_input(ids!(hub_vlm_panel.vlm_image_path)).set_text(cx, path);
                            self.view.redraw(cx);
                            break;
                        }
                    }
                }
            }
            _ => {}
        }

        // ── Image Edit reference image drop zone ─────────────────────────────
        let edit_drop_area = self.view.view(ids!(hub_image_edit_panel.img_edit_drop_zone)).area();
        match event.drag_hits(cx, edit_drop_area) {
            DragHit::Drag(e) => {
                match e.state {
                    DragState::In | DragState::Over => {
                        *e.response.lock().unwrap() = DragResponse::Copy;
                        self.view.view(ids!(hub_image_edit_panel.img_edit_drop_zone)).apply_over(cx, live! { draw_bg: { drag_over: (1.0) } });
                        self.view.redraw(cx);
                    }
                    DragState::Out => {
                        self.view.view(ids!(hub_image_edit_panel.img_edit_drop_zone)).apply_over(cx, live! { draw_bg: { drag_over: (0.0) } });
                        self.view.redraw(cx);
                    }
                }
            }
            DragHit::Drop(e) => {
                self.view.view(ids!(hub_image_edit_panel.img_edit_drop_zone)).apply_over(cx, live! { draw_bg: { drag_over: (0.0) } });
                for item in e.items.iter() {
                    if let DragItem::FilePath { path, .. } = item {
                        let lower = path.to_lowercase();
                        if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
                            || lower.ends_with(".bmp") || lower.ends_with(".gif") || lower.ends_with(".webp") {
                            self.image_edit_state.image_path = path.clone();
                            self.view.text_input(ids!(hub_image_edit_panel.img_edit_image_path)).set_text(cx, path);
                            self.view.redraw(cx);
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if !self.initialized { self.initialize(cx); }

        // Initialize width tracking on first draw (layout comes from live_design)
        if self.left_panel_width == 0.0 { self.left_panel_width = 270.0; }

        let hub_list      = self.view.portal_list(ids!(hub_model_list));
        let hub_list_uid  = hub_list.widget_uid();
        let voice_list    = self.view.portal_list(ids!(hub_voice_panel.voice_list));
        let voice_list_uid = voice_list.widget_uid();
        let tts_voice_list    = self.view.portal_list(ids!(hub_tts_panel.tts_voice_list));
        let tts_voice_list_uid = tts_voice_list.widget_uid();

        while let Some(widget) = self.view.draw_walk(cx, scope, walk).step() {
            if widget.widget_uid() == hub_list_uid {
                self.draw_hub_list(cx, scope, widget);
            } else if widget.widget_uid() == voice_list_uid {
                self.draw_voice_list(cx, scope, widget);
            } else if widget.widget_uid() == tts_voice_list_uid {
                self.draw_tts_voice_list(cx, scope, widget);
            }
        }
        DrawStep::done()
    }
}

impl ModelHubApp {
    // ── Draw list ─────────────────────────────────────────────────────────────

    fn draw_hub_list(&mut self, cx: &mut Cx2d, scope: &mut Scope, widget: WidgetRef) {
        let binding = widget.as_portal_list();
        let Some(mut list) = binding.borrow_mut() else { return };
        list.set_item_range(cx, 0, self.flat_list.len());

        while let Some(item_id) = list.next_visible_item(cx) {
            match self.flat_list.get(item_id).copied() {
                Some(ListRow::Header(cat)) => {
                    let item = list.item(cx, item_id, live_id!(HubCategoryHeader));
                    item.label(ids!(category_header_label)).set_text(cx, cat.label());
                    item.draw_all(cx, scope);
                }
                Some(ListRow::SubHeader(idx)) => {
                    let name = self.subfolder_names.get(idx).map(|s| s.as_str()).unwrap_or("");
                    let item = list.item(cx, item_id, live_id!(HubSubfolderHeader));
                    item.label(ids!(subfolder_header_label)).set_text(cx, name);
                    item.draw_all(cx, scope);
                }
                Some(ListRow::Model(gi)) => {
                    let model_id = self.registry.as_ref()
                        .and_then(|r| r.models.get(gi)).map(|m| m.id.as_str()).unwrap_or("");
                    let name = self.registry.as_ref()
                        .and_then(|r| r.models.get(gi)).map(|m| m.name.as_str()).unwrap_or("");
                    let dl   = self.model_states.get(model_id).copied().unwrap_or(ModelUiState::NotDownloaded);
                    let load = self.load_states.get(model_id).copied().unwrap_or_default();
                    let dot  = combined_dot_value(dl, load);
                    let sel  = self.selected_id.as_deref() == Some(model_id);
                    let dl_frac = self.download_states.get(model_id).map(|d| d.fraction());

                    let item = list.item(cx, item_id, live_id!(HubModelItem));
                    item.label(ids!(model_name)).set_text(cx, &name);
                    item.view(ids!(model_status)).apply_over(cx, live! { draw_bg: { status: (dot) } });
                    item.apply_over(cx, live! { draw_bg: { selected: (if sel { 1.0_f64 } else { 0.0_f64 }) } });
                    // Show "Downloaded" badge for downloaded or loaded models
                    let show_badge = dl == ModelUiState::Downloaded || load != ModelLoadState::Unloaded;
                    item.view(ids!(downloaded_badge)).set_visible(cx, show_badge);
                    if let Some(pct) = dl_frac {
                        item.view(ids!(inline_progress)).set_visible(cx, true);
                        item.view(ids!(inline_progress)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                    } else {
                        item.view(ids!(inline_progress)).set_visible(cx, false);
                    }
                    item.draw_all(cx, scope);
                }
                Some(ListRow::VoiceStudio) => {
                    let sel = self.active_panel == ActivePanel::Voice;
                    let item = list.item(cx, item_id, live_id!(HubVoiceStudioItem));
                    item.apply_over(cx, live! { draw_bg: { selected: (if sel { 1.0_f64 } else { 0.0_f64 }) } });
                    item.draw_all(cx, scope);
                }
                None => {}
            }
        }
    }

    // ── Draw voice list ───────────────────────────────────────────────────────

    fn draw_voice_list(&mut self, cx: &mut Cx2d, scope: &mut Scope, widget: WidgetRef) {
        let binding = widget.as_portal_list();
        let Some(mut list) = binding.borrow_mut() else { return };
        list.set_item_range(cx, 0, self.voices.len());

        while let Some(item_id) = list.next_visible_item(cx) {
            if let Some(voice) = self.voices.get(item_id) {
                let sel   = self.selected_voice_idx == Some(item_id);
                let ready = voice.is_ready;
                let name  = voice.name.clone();

                let item = list.item(cx, item_id, live_id!(HubVoiceListItem));
                item.label(ids!(voice_item_name)).set_text(cx, &name);
                item.apply_over(cx, live! { draw_bg: { selected: (if sel { 1.0_f64 } else { 0.0_f64 }) } });
                item.view(ids!(voice_status_dot)).apply_over(cx, live! {
                    draw_bg: { ready: (if ready { 1.0_f64 } else { 0.0_f64 }) }
                });
                item.draw_all(cx, scope);
            }
        }
    }

    // ── Draw TTS voice selector list ──────────────────────────────────────────

    fn draw_tts_voice_list(&mut self, cx: &mut Cx2d, scope: &mut Scope, widget: WidgetRef) {
        let binding = widget.as_portal_list();
        let Some(mut list) = binding.borrow_mut() else { return };
        list.set_item_range(cx, 0, TTS_PRESET_VOICES.len());

        while let Some(item_id) = list.next_visible_item(cx) {
            if let Some(voice) = TTS_PRESET_VOICES.get(item_id) {
                let sel    = self.selected_tts_voice_idx == item_id;
                let is_zh  = if voice.is_chinese { 1.0_f64 } else { 0.0_f64 };
                let lang   = if voice.is_chinese { "ZH" } else { "EN" };

                let item = list.item(cx, item_id, live_id!(HubTtsVoiceItem));
                item.label(ids!(tts_voice_name)).set_text(cx, voice.label);
                item.label(ids!(tts_lang_badge.tts_lang_label)).set_text(cx, lang);
                item.apply_over(cx, live! { draw_bg: { selected: (if sel { 1.0_f64 } else { 0.0_f64 }) } });
                item.view(ids!(tts_lang_badge)).apply_over(cx, live! { draw_bg: { is_chinese: (is_zh) } });
                item.label(ids!(tts_lang_badge.tts_lang_label)).apply_over(cx, live! { draw_text: { is_chinese: (is_zh) } });
                item.draw_all(cx, scope);
            }
        }
    }

    // ── Initialisation ───────────────────────────────────────────────────────

    fn initialize(&mut self, cx: &mut Cx) {
        self.initialized = true;
        ::log::info!("ModelHubApp::initialize — hub_category = {}", self.hub_category);
        // Lock filter to the configured category (set by the shell via live_design)
        self.filter = match self.hub_category as u32 {
            1 => Filter::Cat(RegistryCategory::Llm),
            2 => Filter::Cat(RegistryCategory::Vlm),
            3 => Filter::Cat(RegistryCategory::Asr),
            4 => Filter::Cat(RegistryCategory::Tts),
            5 => Filter::Cat(RegistryCategory::ImageGen),
            6 => Filter::Cat(RegistryCategory::VideoGen),
            _ => Filter::All,
        };
        let registry = ModelRegistry::load();
        ModelRegistry::fetch_updates_async();
        for model in &registry.models {
            self.model_states.insert(model.id.clone(), scan_state(model));
        }
        self.registry = Some(registry);
        self.rebuild_list();
        ::log::info!("ModelHubApp::initialize — filter={:?}, flat_list has {} items", self.filter, self.flat_list.len());
        // Sync load states from the server immediately
        self.poll_server_status();
        // Hide "Open in Chat" button and loading label (Label doesn't support visible: false in live_design)
        self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_chat_btn)).set_visible(cx, false);
        self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_chat_btn)).set_visible(cx, false);
        self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_loading_label)).set_visible(cx, false);
        self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_loading_label)).set_visible(cx, false);
        self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_loading_label)).set_visible(cx, false);
        self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_loading_label)).set_visible(cx, false);
        self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_loading_label)).set_visible(cx, false);
        self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_loading_label)).set_visible(cx, false);
        self.view.widget(ids!(hub_voice_panel)).set_visible(cx, false);
        // Init voice defaults
        self.voice_quality  = "standard".to_string();
        self.voice_language = "auto".to_string();
        self.voice_denoise  = true;
        self.view.redraw(cx);
    }

    // ── List building ─────────────────────────────────────────────────────────

    fn rebuild_list(&mut self) {
        let Some(registry) = &self.registry else { return };
        let q = self.search_query.to_lowercase();

        const CATS: [RegistryCategory; 6] = [
            RegistryCategory::Llm, RegistryCategory::Vlm, RegistryCategory::Asr,
            RegistryCategory::Tts, RegistryCategory::ImageGen, RegistryCategory::VideoGen,
        ];
        self.flat_list.clear();
        self.subfolder_names.clear();

        let single_category = matches!(self.filter, Filter::Cat(_));
        for cat in CATS {
            if let Filter::Cat(fc) = self.filter { if fc != cat { continue; } }
            let models: Vec<usize> = registry.models.iter().enumerate()
                .filter(|(_, m)| m.category == cat)
                .filter(|(_, m)| q.is_empty()
                    || m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q)))
                .map(|(i, _)| i)
                .collect();
            if models.is_empty() { continue; }
            // Skip category header when locked to a single category
            if !single_category {
                self.flat_list.push(ListRow::Header(cat));
            }
            // Emit subfolder headers when the subfolder name changes.
            let mut last_subfolder = String::new();
            for gi in models {
                let sf = registry.models[gi].subfolder.clone();
                if !sf.is_empty() && sf != last_subfolder {
                    last_subfolder = sf.clone();
                    let idx = self.subfolder_names.len();
                    self.subfolder_names.push(sf);
                    self.flat_list.push(ListRow::SubHeader(idx));
                }
                self.flat_list.push(ListRow::Model(gi));
            }
        }
        // Voice Studio: show in the All hub (hub_category 0) and the TTS hub (hub_category 4)
        let in_voice_hub = self.hub_category == 0.0 || self.hub_category as u32 == 4;
        if in_voice_hub && q.is_empty() {
            self.flat_list.push(ListRow::VoiceStudio);
        }
    }

    // ── Panel visibility ──────────────────────────────────────────────────────

    fn show_panel(&mut self, cx: &mut Cx, panel: ActivePanel) {
        self.active_panel = panel;
        self.view.widget(ids!(hub_empty_state)).set_visible(cx, panel == ActivePanel::None);
        self.view.widget(ids!(hub_llm_panel)).set_visible(cx, panel == ActivePanel::Llm);
        self.view.widget(ids!(hub_vlm_panel)).set_visible(cx, panel == ActivePanel::Vlm);
        self.view.widget(ids!(hub_asr_panel)).set_visible(cx, panel == ActivePanel::Asr);
        self.view.widget(ids!(hub_tts_panel)).set_visible(cx, panel == ActivePanel::Tts);
        self.view.widget(ids!(hub_image_panel)).set_visible(cx, panel == ActivePanel::Image);
        self.view.widget(ids!(hub_image_edit_panel)).set_visible(cx, panel == ActivePanel::ImageEdit);
        self.view.widget(ids!(hub_video_panel)).set_visible(cx, panel == ActivePanel::Video);
        self.view.widget(ids!(hub_voice_panel)).set_visible(cx, panel == ActivePanel::Voice);
    }

    // ── Model selection ───────────────────────────────────────────────────────

    /// Called by the shell after loading a model via the model-selector bar.
    /// Auto-selects the model and shows its inference panel.
    pub fn focus_model(&mut self, cx: &mut Cx, model_id: &str) {
        if !self.initialized { self.initialize(cx); }
        self.selected_id = Some(model_id.to_string());
        self.on_model_selected(cx, model_id);
        self.view.redraw(cx);
    }

    fn on_model_selected(&mut self, cx: &mut Cx, model_id: &str) {
        self.reset_remove_confirmation(cx);
        let model = self.registry.as_ref()
            .and_then(|r| r.models.iter().find(|m| m.id == model_id))
            .cloned();
        let Some(model) = model else { return };

        let panel = match model.category {
            RegistryCategory::Llm      => ActivePanel::Llm,
            RegistryCategory::Vlm      => ActivePanel::Vlm,
            RegistryCategory::Asr      => ActivePanel::Asr,
            RegistryCategory::Tts      => ActivePanel::Tts,
            RegistryCategory::ImageGen => {
                // Use panel_type to distinguish image gen vs image edit
                if model.ui.panel_type == PanelType::ImageEdit {
                    ActivePanel::ImageEdit
                } else {
                    ActivePanel::Image
                }
            }
            RegistryCategory::VideoGen => ActivePanel::Video,
        };

        self.show_panel(cx, panel);
        self.refresh_header_for(cx, model_id);

        // TTS: lazily load available voices
        if panel == ActivePanel::Tts
            && self.tts_state.voices.is_empty()
            && self.tts_state.voices_rx.is_none()
        {
            self.load_tts_voices();
        }

        // Refresh server status for accurate Load/Unload button state
        if self.server_status_rx.is_none() {
            self.poll_server_status();
        }
    }

    // ── Panel header refresh ─────────────────────────────────────────────────

    fn refresh_header_for(&mut self, cx: &mut Cx, model_id: &str) {
        let model = self.registry.as_ref()
            .and_then(|r| r.models.iter().find(|m| m.id == model_id))
            .cloned();
        let Some(model) = model else { return };

        let dl       = self.model_states.get(model_id).copied().unwrap_or(ModelUiState::NotDownloaded);
        let load     = self.load_states.get(model_id).copied().unwrap_or_default();
        ::log::info!("refresh_header_for: model={}, dl={:?}, load={:?}, panel={:?}", model_id, dl, load, self.active_panel);
        let is_dl    = dl == ModelUiState::Downloading;
        let is_done  = dl == ModelUiState::Downloaded;
        let is_manual = model.source.kind == SourceKind::Manual;

        // Download buttons
        let show_dl   = !is_dl && !is_done && !is_manual;
        let show_can  = is_dl;
        let show_rm   = is_done;
        let show_prog = is_dl;
        let show_download_btn = (show_dl || is_dl) && !is_manual;
        let download_text = if is_dl { "Downloading..." } else { "Download" };

        // Load / Unload buttons (not applicable to ImageEdit — sd.cpp runs directly)
        let is_image_edit = self.active_panel == ActivePanel::ImageEdit;
        let show_load    = is_done && load == ModelLoadState::Unloaded && !is_image_edit;
        let show_unload  = is_done && load == ModelLoadState::Loaded && !is_image_edit;
        let show_loading = is_done && load == ModelLoadState::Loading && !is_image_edit;
        let loading_text = if show_loading { "Loading model..." } else { "" };

        let dot      = combined_dot_value(dl, load);
        let st_label = combined_status_label(dl, load);
        let status_text = if show_download_btn { "" } else { st_label };
        let name     = model.name.clone();
        let desc     = model.description.clone();
        let size     = model.storage.size_display.clone();
        let mem      = format!("{:.1} GB", model.runtime.memory_gb);

        // Status message
        let msg = if is_manual {
            format!("Manual install: {}", model.storage.local_path)
        } else if load == ModelLoadState::LoadError {
            "Load failed. Check logs — ominix-api may be missing or model files incomplete.".to_string()
        } else if show_load {
            "Ready to load into memory.".to_string()
        } else if is_image_edit && is_done {
            "Ready for image editing.".to_string()
        } else {
            String::new()
        };

        let dl_state = self.download_states.get(model_id).cloned();
        let pct = dl_state.as_ref().map(|d| d.fraction());
        let txt = dl_state.as_ref().map(|d| d.progress_text());

        // Memory guard warning: check if another model of same category is Loaded
        let cat = model.category;

        // "Open in Chat" button — only for LLM/VLM when loaded
        let show_chat = load == ModelLoadState::Loaded
            && (cat == RegistryCategory::Llm || cat == RegistryCategory::Vlm);
        let blocker_name = if show_load {
            self.registry.as_ref().and_then(|r| {
                r.models.iter().find(|m| {
                    m.category == cat
                        && m.id != model_id
                        && self.load_states.get(&m.id).copied() == Some(ModelLoadState::Loaded)
                }).map(|m| m.name.clone())
            })
        } else {
            None
        };
        let msg = if let Some(ref blocker) = blocker_name {
            format!("Unload '{}' first — only one {} model can be loaded at a time.", blocker, cat.label())
        } else {
            msg
        };
        let show_status_msg = !msg.is_empty();
        // Disable Load button if another model is blocking
        let show_load = show_load && blocker_name.is_none();
        let show_action_row = show_can;
        let show_runtime_row = show_load || show_unload || show_loading || show_chat || show_rm;

        // ids!() is compile-time — each panel's paths must be written explicitly.
        match self.active_panel {
            ActivePanel::Llm => {
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_llm_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_chat_btn)).set_visible(cx, show_chat);
                self.view.widget(ids!(hub_llm_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_llm_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::Vlm => {
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_vlm_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_chat_btn)).set_visible(cx, show_chat);
                self.view.widget(ids!(hub_vlm_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_vlm_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::Asr => {
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_asr_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_asr_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_asr_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_asr_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::Tts => {
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_tts_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_tts_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_tts_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_tts_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::Image => {
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_image_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_image_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_image_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_image_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_image_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_image_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::ImageEdit => {
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_image_edit_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_image_edit_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_image_edit_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_image_edit_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::Video => {
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_model_name)).set_text(cx, &name);
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_model_desc)).set_text(cx, &desc);
                self.view.view(ids!(hub_video_panel.hub_panel_header.panel_status_dot))
                    .apply_over(cx, live! { draw_bg: { status: (dot) } });
                self.view.button(ids!(hub_video_panel.hub_panel_header.panel_download_btn)).set_text(cx, download_text);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_download_btn)).set_visible(cx, show_download_btn);
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_status_text)).set_text(cx, status_text);
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_size_text)).set_text(cx, &size);
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_mem_text)).set_text(cx, &mem);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_action_row)).set_visible(cx, show_action_row);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_cancel_btn)).set_visible(cx, show_can);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_remove_btn)).set_visible(cx, show_rm);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_progress_section)).set_visible(cx, show_prog);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_runtime_row)).set_visible(cx, show_runtime_row);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_load_btn)).set_visible(cx, show_load);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_unload_btn)).set_visible(cx, show_unload);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_loading_label)).set_visible(cx, show_loading);
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_loading_label)).set_text(cx, loading_text);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_chat_btn)).set_visible(cx, false);
                self.view.widget(ids!(hub_video_panel.hub_panel_header.panel_status_msg)).set_visible(cx, show_status_msg);
                self.view.label(ids!(hub_video_panel.hub_panel_header.panel_status_msg)).set_text(cx, &msg);
                if show_prog {
                    if let Some(p) = pct {
                        self.view.view(ids!(hub_video_panel.hub_panel_header.panel_progress_fill))
                            .apply_over(cx, live! { draw_bg: { progress: (p) } });
                    }
                    if let Some(ref t) = txt {
                        self.view.label(ids!(hub_video_panel.hub_panel_header.panel_progress_text)).set_text(cx, t);
                    }
                }
            }
            ActivePanel::Voice | ActivePanel::Info => {}
            ActivePanel::None => {}
        }
        self.view.redraw(cx);
    }
}

// ─── Event handlers ───────────────────────────────────────────────────────────

impl ModelHubApp {
    fn handle_search(&mut self, actions: &Actions, cx: &mut Cx) {
        if let Some(txt) = self.view.text_input(ids!(search_input)).changed(actions) {
            self.search_query = txt.to_string();
            self.rebuild_list();
            self.view.redraw(cx);
        }
    }

    fn handle_list_clicks(&mut self, cx: &mut Cx, actions: &Actions) {
        let list = self.view.portal_list(ids!(hub_model_list));
        for (item_id, item) in list.items_with_actions(actions) {
            let row = self.flat_list.get(item_id).copied();
            if let Some(ListRow::Model(gi)) = row {
                if let Some(fd) = item.as_view().finger_down(actions) {
                    if fd.tap_count == 1 {
                        if let Some(id) = self.registry.as_ref()
                            .and_then(|r| r.models.get(gi))
                            .map(|m| m.id.clone())
                        {
                            self.selected_id = Some(id.clone());
                            self.on_model_selected(cx, &id);
                            self.view.redraw(cx);
                        }
                    }
                }
            } else if let Some(ListRow::VoiceStudio) = row {
                if let Some(fd) = item.as_view().finger_down(actions) {
                    if fd.tap_count == 1 {
                        self.selected_id = None;
                        self.on_voice_studio_selected(cx);
                        self.view.redraw(cx);
                    }
                }
            }
        }
    }

    /// Handle Download / Cancel / Remove buttons in the active panel header.
    fn handle_panel_header_buttons(&mut self, cx: &mut Cx, actions: &Actions) {
        let sel = match self.selected_id.clone() { Some(s) => s, None => return };

        let (dl, cancel, rm) = match self.active_panel {
            ActivePanel::Llm => (
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::Vlm => (
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::Asr => (
                self.view.button(ids!(hub_asr_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_asr_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_asr_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::Tts => (
                self.view.button(ids!(hub_tts_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_tts_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_tts_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::Image => (
                self.view.button(ids!(hub_image_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_image_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_image_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::ImageEdit => (
                self.view.button(ids!(hub_image_edit_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_image_edit_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_image_edit_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::Video => (
                self.view.button(ids!(hub_video_panel.hub_panel_header.panel_download_btn)).clicked(actions),
                self.view.button(ids!(hub_video_panel.hub_panel_header.panel_cancel_btn)).clicked(actions),
                self.view.button(ids!(hub_video_panel.hub_panel_header.panel_remove_btn)).clicked(actions),
            ),
            ActivePanel::Voice | ActivePanel::Info | ActivePanel::None => return,
        };

        if dl { self.start_download(cx, &sel); }
        if cancel {
            if let Some(ds) = self.download_states.get(&sel) {
                ds.cancel_requested.store(true, Ordering::SeqCst);
            }
        }
        if rm {
            if self.pending_remove_id.as_deref() == Some(sel.as_str()) {
                // Second click: confirmed — actually remove the model
                self.pending_remove_id = None;
                if let Some(model) = self.registry.as_ref()
                    .and_then(|r| r.models.iter().find(|m| m.id == sel))
                {
                    let path = expand_tilde(&model.storage.local_path);
                    if std::fs::remove_dir_all(&path).is_ok() {
                        self.model_states.insert(sel.clone(), ModelUiState::NotDownloaded);
                        self.load_states.remove(&sel);
                        self.refresh_header_for(cx, &sel);
                        self.view.redraw(cx);
                        ::log::info!("Removed model {}", sel);
                    }
                }
            } else {
                // First click: show confirmation
                self.pending_remove_id = Some(sel.clone());
                self.set_remove_btn_text(cx, "Are you sure?");
                self.view.redraw(cx);
            }
        }
    }

    /// Set the remove button text on the active panel
    fn set_remove_btn_text(&mut self, cx: &mut Cx, text: &str) {
        let btn_id = match self.active_panel {
            ActivePanel::Llm       => ids!(hub_llm_panel.hub_panel_header.panel_remove_btn),
            ActivePanel::Vlm       => ids!(hub_vlm_panel.hub_panel_header.panel_remove_btn),
            ActivePanel::Asr       => ids!(hub_asr_panel.hub_panel_header.panel_remove_btn),
            ActivePanel::Tts       => ids!(hub_tts_panel.hub_panel_header.panel_remove_btn),
            ActivePanel::Image     => ids!(hub_image_panel.hub_panel_header.panel_remove_btn),
            ActivePanel::ImageEdit => ids!(hub_image_edit_panel.hub_panel_header.panel_remove_btn),
            ActivePanel::Video     => ids!(hub_video_panel.hub_panel_header.panel_remove_btn),
            _ => return,
        };
        self.view.button(btn_id).set_text(cx, text);
    }

    /// Reset the remove confirmation state (e.g. when switching models)
    fn reset_remove_confirmation(&mut self, cx: &mut Cx) {
        if self.pending_remove_id.take().is_some() {
            self.set_remove_btn_text(cx, "Remove");
        }
    }

    /// Handle Load / Unload buttons in the active panel header.
    fn handle_load_buttons(&mut self, cx: &mut Cx, actions: &Actions) {
        let sel = match self.selected_id.clone() { Some(s) => s, None => return };

        let (load_clicked, unload_clicked) = match self.active_panel {
            ActivePanel::Llm => (
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::Vlm => (
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::Asr => (
                self.view.button(ids!(hub_asr_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_asr_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::Tts => (
                self.view.button(ids!(hub_tts_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_tts_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::Image => (
                self.view.button(ids!(hub_image_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_image_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::ImageEdit => (
                self.view.button(ids!(hub_image_edit_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_image_edit_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::Video => (
                self.view.button(ids!(hub_video_panel.hub_panel_header.panel_load_btn)).clicked(actions),
                self.view.button(ids!(hub_video_panel.hub_panel_header.panel_unload_btn)).clicked(actions),
            ),
            ActivePanel::Voice | ActivePanel::Info | ActivePanel::None => return,
        };

        if load_clicked   { self.start_load(cx, &sel); }
        if unload_clicked { self.start_unload(cx, &sel); }
    }

    /// Handle "Open in Chat" button — dispatch OpenChatWithModel to open a fresh chat session.
    fn handle_chat_button(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let chat_clicked = match self.active_panel {
            ActivePanel::Llm =>
                self.view.button(ids!(hub_llm_panel.hub_panel_header.panel_chat_btn)).clicked(actions),
            ActivePanel::Vlm =>
                self.view.button(ids!(hub_vlm_panel.hub_panel_header.panel_chat_btn)).clicked(actions),
            _ => false,
        };

        if !chat_clicked { return; }

        let sel = match self.selected_id.clone() { Some(s) => s, None => return };

        let model = self.registry.as_ref()
            .and_then(|r| r.models.iter().find(|m| m.id == sel))
            .cloned();
        let Some(model) = model else { return };

        let api_model_id = model.runtime.api_model_id.clone();
        let category = model.category;

        cx.action(StoreAction::OpenChatWithModel {
            model_id: api_model_id,
            category,
        });
    }

    fn handle_input_changes(&mut self, actions: &Actions) {
        if let Some(t) = self.view.text_input(ids!(hub_llm_panel.llm_system)).changed(actions)       { self.llm_state.system = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_llm_panel.llm_user)).changed(actions)         { self.llm_state.user = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_vlm_panel.vlm_image_path)).changed(actions)   { self.vlm_state.image_path = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_vlm_panel.vlm_user)).changed(actions)         { self.vlm_state.user = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_asr_panel.asr_audio_path)).changed(actions)   { self.asr_state.audio_path = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_tts_panel.tts_text_input)).changed(actions)   { self.tts_state.text = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_image_panel.img_prompt)).changed(actions)     { self.image_state.prompt = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_image_panel.img_neg_prompt)).changed(actions) { self.image_state.neg_prompt = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_image_edit_panel.img_edit_image_path)).changed(actions) { self.image_edit_state.image_path = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_image_edit_panel.img_edit_prompt)).changed(actions)     { self.image_edit_state.prompt = t.to_string(); }
        if let Some(t) = self.view.text_input(ids!(hub_video_panel.vid_prompt)).changed(actions)               { self.video_state.prompt = t.to_string(); }
    }

    fn handle_llm_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.view.button(ids!(hub_llm_panel.llm_generate_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let system = self.llm_state.system.clone();
                let user   = self.llm_state.user.clone();
                self.call_llm(cx, sel, system, user);
            }
        }
    }
    fn handle_vlm_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.view.button(ids!(hub_vlm_panel.vlm_browse_btn)).clicked(actions) {
            if let Some(path) = FileDialog::new()
                .add_filter("Image", &["jpg", "jpeg", "png", "bmp", "gif", "webp"])
                .pick_file()
            {
                let s = path.to_string_lossy().to_string();
                self.vlm_state.image_path = s.clone();
                self.view.text_input(ids!(hub_vlm_panel.vlm_image_path)).set_text(cx, &s);
                self.view.redraw(cx);
            }
        }
        if self.view.button(ids!(hub_vlm_panel.vlm_generate_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let img  = self.vlm_state.image_path.clone();
                let user = self.vlm_state.user.clone();
                self.call_vlm(cx, sel, img, user);
            }
        }
    }
    fn handle_asr_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.view.button(ids!(hub_asr_panel.asr_browse_btn)).clicked(actions) {
            if let Some(path) = FileDialog::new()
                .add_filter("Audio", &["wav", "mp3", "m4a", "flac", "ogg", "aac"])
                .pick_file()
            {
                let s = path.to_string_lossy().to_string();
                self.asr_state.audio_path = s.clone();
                self.view.text_input(ids!(hub_asr_panel.asr_audio_path)).set_text(cx, &s);
                self.view.redraw(cx);
            }
        }
        if self.view.button(ids!(hub_asr_panel.asr_transcribe_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let load = self.load_states.get(&sel).copied().unwrap_or_default();
                if load != ModelLoadState::Loaded {
                    self.view.label(ids!(hub_asr_panel.asr_status)).set_text(cx, "Model not loaded — click Load first.");
                    return;
                }
                let path = self.asr_state.audio_path.clone();
                self.call_asr(cx, sel, path);
            }
        }
    }
    fn handle_tts_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // TTS voice selector clicks
        let tts_voice_list = self.view.portal_list(ids!(hub_tts_panel.tts_voice_list));
        for (item_id, item) in tts_voice_list.items_with_actions(actions) {
            if let Some(fd) = item.as_view().finger_down(actions) {
                if fd.tap_count == 1 {
                    self.selected_tts_voice_idx = item_id;
                    self.tts_state.voice_id = TTS_PRESET_VOICES.get(item_id)
                        .map(|v| v.id.to_string())
                        .unwrap_or_else(|| "vivian".to_string());
                    self.view.redraw(cx);
                }
            }
        }

        // Save audio to Downloads
        if self.view.button(ids!(hub_tts_panel.tts_result_row.tts_save_btn)).clicked(actions) {
            let src = self.tts_state.output_path.clone();
            if !src.is_empty() {
                let downloads = dirs::download_dir()
                    .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0);
                let dest = downloads.join(format!("ominix-tts-{}.wav", ts));
                match std::fs::copy(&src, &dest) {
                    Ok(_) => {
                        self.tts_state.output_path = dest.to_string_lossy().to_string();
                        self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, &format!("Saved to {}", dest.display()));
                        self.view.button(ids!(hub_tts_panel.tts_result_row.tts_save_btn)).set_visible(cx, false);
                        self.view.button(ids!(hub_tts_panel.tts_result_row.tts_finder_btn)).set_visible(cx, true);
                        self.view.redraw(cx);
                    }
                    Err(e) => {
                        self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, &format!("Save failed: {}", e));
                        self.view.redraw(cx);
                    }
                }
            }
        }

        // Show saved audio in Finder
        if self.view.button(ids!(hub_tts_panel.tts_result_row.tts_finder_btn)).clicked(actions) {
            let path = self.tts_state.output_path.clone();
            if !path.is_empty() {
                let _ = std::process::Command::new("open").args(["-R", &path]).spawn();
            }
        }

        if self.view.button(ids!(hub_tts_panel.tts_generate_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let load = self.load_states.get(&sel).copied().unwrap_or_default();
                if load != ModelLoadState::Loaded {
                    self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, "Model not loaded — click Load first.");
                    return;
                }
                let voice = self.tts_state.voice_id.clone();
                let text  = self.tts_state.text.clone();
                self.call_tts(cx, sel, voice, text);
            }
        }
    }
    fn handle_image_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.view.button(ids!(hub_image_panel.img_generate_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let load = self.load_states.get(&sel).copied().unwrap_or_default();
                if load != ModelLoadState::Loaded {
                    self.view.label(ids!(hub_image_panel.img_status)).set_text(cx, "Model not loaded — click Load first.");
                    return;
                }
                let prompt = self.image_state.prompt.clone();
                let neg    = self.image_state.neg_prompt.clone();
                self.call_image(cx, sel, prompt, neg);
            }
        }

        if self.view.button(ids!(hub_image_panel.img_result_row.img_open_finder_btn)).clicked(actions) {
            let path = self.image_state.output_path.clone();
            if !path.is_empty() {
                let _ = std::process::Command::new("open").args(["-R", &path]).spawn();
            }
        }
    }

    fn handle_image_edit_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Browse reference image
        if self.view.button(ids!(hub_image_edit_panel.img_edit_browse_btn)).clicked(actions) {
            if let Some(path) = FileDialog::new()
                .add_filter("Image", &["jpg", "jpeg", "png", "bmp", "gif", "webp"])
                .pick_file()
            {
                let s = path.to_string_lossy().to_string();
                self.image_edit_state.image_path = s.clone();
                self.view.text_input(ids!(hub_image_edit_panel.img_edit_image_path)).set_text(cx, &s);
                self.view.redraw(cx);
            }
        }

        // Edit button — sd.cpp models don't need API loading, just download
        if self.view.button(ids!(hub_image_edit_panel.img_edit_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let dl = self.model_states.get(&sel).copied().unwrap_or(ModelUiState::NotDownloaded);
                if dl != ModelUiState::Downloaded {
                    self.view.label(ids!(hub_image_edit_panel.img_edit_status)).set_text(cx, "Model not downloaded yet — click Download first.");
                    return;
                }
                let image_path = self.image_edit_state.image_path.clone();
                let prompt     = self.image_edit_state.prompt.clone();
                self.call_image_edit(cx, sel, image_path, prompt);
            }
        }

        // Show in Finder button
        if self.view.button(ids!(hub_image_edit_panel.img_edit_result_row.img_edit_open_finder_btn)).clicked(actions) {
            let path = self.image_edit_state.output_path.clone();
            if !path.is_empty() {
                let _ = std::process::Command::new("open").args(["-R", &path]).spawn();
            }
        }
    }

    fn handle_video_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.view.button(ids!(hub_video_panel.vid_generate_btn)).clicked(actions) {
            if let Some(sel) = self.selected_id.clone() {
                let load = self.load_states.get(&sel).copied().unwrap_or_default();
                if load != ModelLoadState::Loaded {
                    self.view.label(ids!(hub_video_panel.vid_status)).set_text(cx, "Model not loaded — click Load first.");
                    return;
                }
                let prompt = self.video_state.prompt.clone();
                self.call_video(cx, sel, prompt);
            }
        }

        if self.view.button(ids!(hub_video_panel.vid_result_row.vid_play_btn)).clicked(actions) {
            let path = self.video_state.output_path.clone();
            if !path.is_empty() {
                let _ = std::process::Command::new("open").arg(&path).spawn();
            }
        }

        if self.view.button(ids!(hub_video_panel.vid_result_row.vid_open_finder_btn)).clicked(actions) {
            let path = self.video_state.output_path.clone();
            if !path.is_empty() {
                let _ = std::process::Command::new("open").args(["-R", &path]).spawn();
            }
        }
    }

    // ── Voice Studio event handlers ───────────────────────────────────────────

    fn on_voice_studio_selected(&mut self, cx: &mut Cx) {
        self.show_panel(cx, ActivePanel::Voice);
        // Initialize voice defaults if not done yet
        if self.voice_quality.is_empty() {
            self.voice_quality  = "standard".to_string();
            self.voice_language = "auto".to_string();
            self.voice_denoise  = true;
        }
        // Fetch voice list if not already done
        if self.voices.is_empty() && self.voice_list_rx.is_none() {
            self.fetch_voice_list();
        }
    }

    fn handle_voice_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.active_panel != ActivePanel::Voice { return; }

        // + New Voice button — clear form
        if self.view.button(ids!(hub_voice_panel.voice_new_btn)).clicked(actions) {
            self.selected_voice_idx = None;
            self.view.text_input(ids!(hub_voice_panel.voice_name_input)).set_text(cx, "");
            self.view.text_input(ids!(hub_voice_panel.voice_audio_path_input)).set_text(cx, "");
            self.view.text_input(ids!(hub_voice_panel.voice_transcript_input)).set_text(cx, "");
            self.view.redraw(cx);
        }

        // Voice list clicks
        let voices_list = self.view.portal_list(ids!(hub_voice_panel.voice_list));
        for (item_id, item) in voices_list.items_with_actions(actions) {
            if let Some(fd) = item.as_view().finger_down(actions) {
                if fd.tap_count == 1 {
                    self.selected_voice_idx = Some(item_id);
                    self.view.redraw(cx);
                }
            }
        }

        // Quality buttons
        if self.view.button(ids!(hub_voice_panel.voice_quality_fast)).clicked(actions) {
            self.voice_quality = "fast".to_string();
            self.view.redraw(cx);
        }
        if self.view.button(ids!(hub_voice_panel.voice_quality_standard)).clicked(actions) {
            self.voice_quality = "standard".to_string();
            self.view.redraw(cx);
        }
        if self.view.button(ids!(hub_voice_panel.voice_quality_high)).clicked(actions) {
            self.voice_quality = "high".to_string();
            self.view.redraw(cx);
        }

        // Browse audio file for training
        if self.view.button(ids!(hub_voice_panel.voice_audio_browse_btn)).clicked(actions) {
            if let Some(path) = FileDialog::new()
                .add_filter("Audio", &["wav", "mp3", "m4a", "flac", "ogg", "aac"])
                .pick_file()
            {
                let s = path.to_string_lossy().to_string();
                self.view.text_input(ids!(hub_voice_panel.voice_audio_path_input)).set_text(cx, &s);
                self.view.redraw(cx);
            }
        }

        // Train button
        if self.view.button(ids!(hub_voice_panel.voice_train_btn)).clicked(actions) {
            let name       = self.view.text_input(ids!(hub_voice_panel.voice_name_input)).text();
            let audio_path = self.view.text_input(ids!(hub_voice_panel.voice_audio_path_input)).text();
            let transcript = self.view.text_input(ids!(hub_voice_panel.voice_transcript_input)).text();
            if name.trim().is_empty() {
                self.view.label(ids!(hub_voice_panel.voice_train_status)).set_text(cx, "Please enter a voice name.");
            } else if audio_path.trim().is_empty() {
                self.view.label(ids!(hub_voice_panel.voice_train_status)).set_text(cx, "Please enter the path to a WAV file.");
            } else {
                self.start_voice_training(cx, name, audio_path, transcript);
            }
        }

        // Cancel training button
        if self.view.button(ids!(hub_voice_panel.voice_cancel_train_btn)).clicked(actions) {
            if let Some(cancel) = &self.voice_cancel {
                cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            let task_id = self.voice_task_id.clone();
            std::thread::spawn(move || {
                let _ = reqwest::blocking::Client::new()
                    .post("http://localhost:8080/v1/voices/train/cancel")
                    .json(&serde_json::json!({ "task_id": task_id }))
                    .send();
            });
            self.voice_training_state = VoiceTrainingState::Idle;
            self.voice_training_rx = None;
            self.view.redraw(cx);
        }

        // Generate/synthesize button
        if self.view.button(ids!(hub_voice_panel.voice_generate_btn)).clicked(actions) {
            let text = self.view.text_input(ids!(hub_voice_panel.voice_synth_text)).text();
            let speed_str = self.view.text_input(ids!(hub_voice_panel.voice_speed_input)).text();
            let speed: f32 = speed_str.parse().unwrap_or(1.0);
            if text.trim().is_empty() {
                self.view.label(ids!(hub_voice_panel.voice_synth_status)).set_text(cx, "Please enter text to synthesize.");
            } else if let Some(idx) = self.selected_voice_idx {
                if idx < self.voices.len() {
                    let voice_name = self.voices[idx].name.clone();
                    self.start_voice_synthesis(cx, text, voice_name, speed);
                } else {
                    self.view.label(ids!(hub_voice_panel.voice_synth_status)).set_text(cx, "Selected voice is no longer available.");
                }
            } else {
                self.view.label(ids!(hub_voice_panel.voice_synth_status)).set_text(cx, "Please select a voice from the list.");
            }
        }

        // Play button
        if self.view.button(ids!(hub_voice_panel.voice_play_btn)).clicked(actions) {
            std::process::Command::new("afplay")
                .arg("/tmp/ominix-voice-out.wav")
                .spawn()
                .ok();
        }
    }

    fn poll_voice_channels(&mut self, cx: &mut Cx) {
        let mut need_next_frame = false;

        // Voice list fetch
        if let Some(rx) = &self.voice_list_rx {
            if let Ok(update) = rx.try_recv() {
                match update {
                    VoicesUpdate::Loaded(voices) => { self.voices = voices; }
                    VoicesUpdate::Error(e) => { ::log::warn!("Voice list fetch failed: {}", e); }
                }
                self.voice_list_rx = None;
                self.view.redraw(cx);
            }
        }

        // Training updates
        if let Some(rx) = &self.voice_training_rx {
            match rx.try_recv() {
                Ok(VoiceTrainingUpdate::Progress { stage, progress }) => {
                    self.voice_training_state = VoiceTrainingState::Training {
                        task_id: self.voice_task_id.clone(),
                        stage,
                        progress,
                    };
                    need_next_frame = true;
                    self.view.redraw(cx);
                }
                Ok(VoiceTrainingUpdate::Done) => {
                    self.voice_training_state = VoiceTrainingState::Done;
                    self.voice_training_rx = None;
                    self.voice_cancel = None;
                    self.view.label(ids!(hub_voice_panel.voice_train_status)).set_text(cx, "Training complete!");
                    self.fetch_voice_list();
                    self.view.redraw(cx);
                }
                Ok(VoiceTrainingUpdate::Error(e)) => {
                    let msg = format!("Training failed: {}", e);
                    self.voice_training_state = VoiceTrainingState::Error(e);
                    self.voice_training_rx = None;
                    self.voice_cancel = None;
                    self.view.label(ids!(hub_voice_panel.voice_train_status)).set_text(cx, &msg);
                    self.view.redraw(cx);
                }
                Err(mpsc::TryRecvError::Empty) => { need_next_frame = true; }
                Err(mpsc::TryRecvError::Disconnected) => { self.voice_training_rx = None; }
            }
        }

        // Synthesis updates
        if let Some(rx) = &self.voice_synthesis_rx {
            match rx.try_recv() {
                Ok(VoiceSynthesisUpdate::Done { duration_secs }) => {
                    self.voice_synthesis_state = VoiceSynthesisState::Done { duration_secs };
                    self.voice_synthesis_rx = None;
                    let msg = format!("Ready — {:.1}s generated", duration_secs);
                    self.view.label(ids!(hub_voice_panel.voice_synth_status)).set_text(cx, &msg);
                    self.view.redraw(cx);
                }
                Ok(VoiceSynthesisUpdate::Error(e)) => {
                    let msg = format!("Synthesis failed: {}", e);
                    self.voice_synthesis_state = VoiceSynthesisState::Error(e);
                    self.voice_synthesis_rx = None;
                    self.view.label(ids!(hub_voice_panel.voice_synth_status)).set_text(cx, &msg);
                    self.view.redraw(cx);
                }
                Err(mpsc::TryRecvError::Empty) => { need_next_frame = true; }
                Err(mpsc::TryRecvError::Disconnected) => { self.voice_synthesis_rx = None; }
            }
        }

        if need_next_frame { cx.new_next_frame(); }
    }

    fn fetch_voice_list(&mut self) {
        let (tx, rx) = mpsc::channel::<VoicesUpdate>();
        self.voice_list_rx = Some(rx);
        std::thread::spawn(move || {
            match reqwest::blocking::get("http://localhost:8080/v1/voices") {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>() {
                        let voices = json.as_array()
                            .map(|arr| arr.iter().filter_map(|v| {
                                let name     = v["name"].as_str()?.to_string();
                                let is_ready = v["status"].as_str().map(|s| s == "ready").unwrap_or(false);
                                Some(VoiceEntry { name, is_ready })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        let _ = tx.send(VoicesUpdate::Loaded(voices));
                    } else {
                        let _ = tx.send(VoicesUpdate::Error("Invalid JSON response".to_string()));
                    }
                }
                Err(e) => { let _ = tx.send(VoicesUpdate::Error(e.to_string())); }
            }
        });
    }

    fn start_voice_training(&mut self, cx: &mut Cx, name: String, audio_path: String, transcript: String) {
        let quality  = self.voice_quality.clone();
        let language = self.voice_language.clone();
        let denoise  = self.voice_denoise;

        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.voice_cancel = Some(cancel.clone());
        let (tx, rx) = mpsc::channel::<VoiceTrainingUpdate>();
        self.voice_training_rx = Some(rx);
        self.voice_training_state = VoiceTrainingState::Training {
            task_id: String::new(),
            stage:   "Starting...".to_string(),
            progress: 0.0,
        };

        std::thread::spawn(move || {
            // Read and base64-encode audio file
            let audio_bytes = match std::fs::read(&audio_path) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(VoiceTrainingUpdate::Error(e.to_string()));
                    return;
                }
            };
            let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);

            let payload = serde_json::json!({
                "name": name,
                "audio_data": audio_b64,
                "transcript": transcript,
                "quality": quality,
                "language": language,
                "denoise": denoise,
            });

            let resp = match reqwest::blocking::Client::new()
                .post("http://localhost:8080/v1/voices/train")
                .json(&payload)
                .send()
            {
                Ok(r) => r,
                Err(e) => { let _ = tx.send(VoiceTrainingUpdate::Error(e.to_string())); return; }
            };

            let task_id = match resp.json::<serde_json::Value>() {
                Ok(v) => v["task_id"].as_str().unwrap_or("").to_string(),
                Err(e) => { let _ = tx.send(VoiceTrainingUpdate::Error(e.to_string())); return; }
            };

            // Poll for status
            loop {
                if cancel.load(std::sync::atomic::Ordering::SeqCst) { return; }
                std::thread::sleep(std::time::Duration::from_millis(800));
                let status_url = format!("http://localhost:8080/v1/voices/train/status?task_id={}", task_id);
                let status = match reqwest::blocking::get(&status_url) {
                    Ok(r) => match r.json::<serde_json::Value>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };
                let state    = status["state"].as_str().unwrap_or("").to_string();
                let stage    = status["stage"].as_str().unwrap_or("").to_string();
                let progress = status["progress"].as_f64().unwrap_or(0.0) as f32;
                match state.as_str() {
                    "done"  => { let _ = tx.send(VoiceTrainingUpdate::Done); return; }
                    "error" => {
                        let msg = status["error"].as_str().unwrap_or("Unknown error").to_string();
                        let _ = tx.send(VoiceTrainingUpdate::Error(msg));
                        return;
                    }
                    _ => { let _ = tx.send(VoiceTrainingUpdate::Progress { stage, progress }); }
                }
            }
        });

        self.view.redraw(cx);
    }

    fn start_voice_synthesis(&mut self, cx: &mut Cx, text: String, voice_name: String, speed: f32) {
        let (tx, rx) = mpsc::channel::<VoiceSynthesisUpdate>();
        self.voice_synthesis_rx = Some(rx);
        self.voice_synthesis_state = VoiceSynthesisState::Generating;
        self.view.label(ids!(hub_voice_panel.voice_synth_status)).set_text(cx, "Generating...");

        std::thread::spawn(move || {
            let payload = serde_json::json!({
                "model": "gpt-so-vits",
                "input": text,
                "voice": voice_name,
                "speed": speed,
                "response_format": "wav",
            });
            let t0 = std::time::Instant::now();
            match reqwest::blocking::Client::new()
                .post("http://localhost:8080/v1/audio/speech")
                .json(&payload)
                .send()
            {
                Ok(mut resp) => {
                    let mut buf = Vec::new();
                    match resp.copy_to(&mut buf) {
                        Ok(_) => {
                            let _ = std::fs::write("/tmp/ominix-voice-out.wav", &buf);
                            let duration_secs = t0.elapsed().as_secs_f32();
                            let _ = tx.send(VoiceSynthesisUpdate::Done { duration_secs });
                        }
                        Err(e) => { let _ = tx.send(VoiceSynthesisUpdate::Error(e.to_string())); }
                    }
                }
                Err(e) => { let _ = tx.send(VoiceSynthesisUpdate::Error(e.to_string())); }
            }
        });

        self.view.redraw(cx);
    }
}

// ─── Load / Unload operations ─────────────────────────────────────────────────

impl ModelHubApp {
    fn start_load(&mut self, cx: &mut Cx, model_id: &str) {
        if self.load_rxs.contains_key(model_id) { return; } // already in flight

        // Must be downloaded first
        if self.model_states.get(model_id).copied() != Some(ModelUiState::Downloaded) {
            return;
        }

        let model = match self.registry.as_ref()
            .and_then(|r| r.models.iter().find(|m| m.id == model_id)).cloned()
        { Some(m) => m, None => return };

        self.load_states.insert(model_id.to_string(), ModelLoadState::Loading);
        self.refresh_header_for(cx, model_id);

        let api_id = model.runtime.api_model_id.clone();
        let model_type = match model.category {
            RegistryCategory::Llm      => "llm",
            RegistryCategory::Vlm      => "vlm",
            RegistryCategory::Asr      => "asr",
            RegistryCategory::Tts      => "tts",
            RegistryCategory::ImageGen => "image",
            RegistryCategory::VideoGen => "video",
        }.to_string();
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.load_rxs.insert(model_id.to_string(), rx);

        std::thread::spawn(move || {
            // Auto-start ominix-api if it isn't running yet
            let result = ensure_server_running()
                .and_then(|()| ModelRuntimeClient::localhost().load_model(&api_id, &model_type));
            let _ = tx.send(result);
        });

        cx.new_next_frame();
        ::log::info!("Load started for {}", model_id);
    }

    fn start_unload(&mut self, cx: &mut Cx, model_id: &str) {
        if self.unload_rxs.contains_key(model_id) { return; }

        let model = match self.registry.as_ref()
            .and_then(|r| r.models.iter().find(|m| m.id == model_id)).cloned()
        { Some(m) => m, None => return };

        // Optimistic update
        self.load_states.insert(model_id.to_string(), ModelLoadState::Unloaded);
        self.refresh_header_for(cx, model_id);

        let model_type = match model.category {
            RegistryCategory::Llm      => "llm",
            RegistryCategory::Vlm      => "vlm",
            RegistryCategory::Asr      => "asr",
            RegistryCategory::Tts      => "tts",
            RegistryCategory::ImageGen => "image",
            RegistryCategory::VideoGen => "video",
        }.to_string();
        let model_id_owned = model_id.to_string();
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.unload_rxs.insert(model_id.to_string(), rx);

        std::thread::spawn(move || {
            let result = ModelRuntimeClient::localhost().unload_model(&model_type);
            let _ = tx.send(result);
            ::log::info!("Unload thread done for {}", model_id_owned);
        });

        self.view.redraw(cx);
    }

    // ── Status poll (GET /v1/models) ─────────────────────────────────────────

    fn poll_server_status(&mut self) {
        if self.server_status_rx.is_some() { return; } // already in flight

        let (tx, rx) = mpsc::channel::<Result<Vec<ServerModelInfo>, String>>();
        self.server_status_rx = Some(rx);

        std::thread::spawn(move || {
            let result = ModelRuntimeClient::localhost().list_models();
            let _ = tx.send(result);
        });
    }

    fn check_server_status_result(&mut self, cx: &mut Cx) {
        let done = if let Some(rx) = &self.server_status_rx {
            match rx.try_recv() {
                Ok(Ok(infos)) => {
                    let mut changed = false;
                    // Build set of loaded IDs reported by server
                    let loaded_api_ids: HashMap<String, ServerModelStatus> = infos.iter()
                        .map(|i| (i.api_id.clone(), i.status))
                        .collect();

                    if let Some(registry) = &self.registry {
                        for model in &registry.models {
                            let server_status = loaded_api_ids
                                .get(&model.runtime.api_model_id)
                                .copied()
                                .unwrap_or(ServerModelStatus::Unloaded);

                            let new_load = match server_status {
                                ServerModelStatus::Loaded   => ModelLoadState::Loaded,
                                ServerModelStatus::Loading  => ModelLoadState::Loading,
                                ServerModelStatus::Error    => ModelLoadState::LoadError,
                                ServerModelStatus::Unloaded => ModelLoadState::Unloaded,
                            };

                            let old = self.load_states.get(&model.id).copied().unwrap_or_default();
                            if old != new_load {
                                self.load_states.insert(model.id.clone(), new_load);
                                changed = true;
                            }
                        }
                    }

                    if changed {
                        if let Some(sel) = self.selected_id.clone() {
                            self.refresh_header_for(cx, &sel);
                        }
                        self.view.redraw(cx);
                    }
                    true
                }
                Ok(Err(e)) => {
                    ::log::warn!("Server status poll failed: {}", e);
                    true
                }
                Err(mpsc::TryRecvError::Empty)        => false,
                Err(mpsc::TryRecvError::Disconnected) => true,
            }
        } else { false };

        if done { self.server_status_rx = None; }
    }

    // ── Poll load / unload channel results ───────────────────────────────────

    fn poll_load_channels(&mut self, cx: &mut Cx) {
        // --- Load results ---
        let load_ids: Vec<String> = self.load_rxs.keys().cloned().collect();
        let mut load_done:   Vec<String>         = Vec::new();
        let mut load_failed: Vec<(String, String)> = Vec::new();

        for id in &load_ids {
            if let Some(rx) = self.load_rxs.get(id) {
                match rx.try_recv() {
                    Ok(Ok(()))    => load_done.push(id.clone()),
                    Ok(Err(e))    => load_failed.push((id.clone(), e)),
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => load_done.push(id.clone()),
                }
            }
        }

        for id in load_done {
            self.load_states.insert(id.clone(), ModelLoadState::Loaded);
            self.load_rxs.remove(&id);
            if self.selected_id.as_deref() == Some(id.as_str()) {
                self.refresh_header_for(cx, &id);
            }
            // Notify shell so the top model selector bar updates
            if let Some(registry) = &self.registry {
                if let Some(model) = registry.models.iter().find(|m| m.id == id) {
                    cx.action(StoreAction::HubModelLoaded {
                        model_id: id.clone(),
                        model_name: strip_quant_suffix(&model.name),
                        category: model.category,
                    });
                }
            }
            self.view.redraw(cx);
            ::log::info!("Model loaded: {}", id);
        }
        for (id, err) in load_failed {
            self.load_states.insert(id.clone(), ModelLoadState::LoadError);
            self.load_rxs.remove(&id);
            if self.selected_id.as_deref() == Some(id.as_str()) {
                self.refresh_header_for(cx, &id);
            }
            self.view.redraw(cx);
            ::log::error!("Load failed for {}: {}", id, err);
        }

        // --- Unload results ---
        let unload_ids: Vec<String> = self.unload_rxs.keys().cloned().collect();
        let mut unload_done:   Vec<String>         = Vec::new();
        let mut unload_failed: Vec<(String, String)> = Vec::new();

        for id in &unload_ids {
            if let Some(rx) = self.unload_rxs.get(id) {
                match rx.try_recv() {
                    Ok(Ok(()))    => unload_done.push(id.clone()),
                    Ok(Err(e))    => unload_failed.push((id.clone(), e)),
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => unload_done.push(id.clone()),
                }
            }
        }

        for id in unload_done {
            self.unload_rxs.remove(&id);
            // State was already set to Unloaded optimistically; confirm it
            self.load_states.insert(id.clone(), ModelLoadState::Unloaded);
            if self.selected_id.as_deref() == Some(id.as_str()) {
                self.refresh_header_for(cx, &id);
            }
            // Notify shell so the top model selector bar clears
            cx.action(StoreAction::HubModelUnloaded { model_id: id.clone() });
            self.view.redraw(cx);
            ::log::info!("Model unloaded: {}", id);
        }
        for (id, err) in unload_failed {
            // Unload failed — revert to Loaded
            self.load_states.insert(id.clone(), ModelLoadState::Loaded);
            self.unload_rxs.remove(&id);
            if self.selected_id.as_deref() == Some(id.as_str()) {
                self.refresh_header_for(cx, &id);
            }
            self.view.redraw(cx);
            ::log::error!("Unload failed for {}: {}", id, err);
        }

        // Keep the frame loop going while operations are in flight
        if !self.load_rxs.is_empty() || !self.unload_rxs.is_empty() {
            cx.new_next_frame();
        }
    }
}

// ─── Inference API calls ──────────────────────────────────────────────────────

impl ModelHubApp {
    fn call_llm(&mut self, cx: &mut Cx, model_id: String, system: String, user: String) {
        if self.llm_state.is_running { return; }
        self.llm_state.is_running = true;
        self.view.label(ids!(hub_llm_panel.llm_status)).set_text(cx, "Generating...");
        self.view.label(ids!(hub_llm_panel.llm_response.output_label)).set_text(cx, "");
        self.view.redraw(cx);

        let (tx, rx) = mpsc::channel();
        self.llm_state.rx = Some(rx);
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120)).build().unwrap();
            let body = serde_json::json!({
                "model": model_id,
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user",   "content": user}
                ]
            });
            let result = client.post("http://localhost:8080/v1/chat/completions")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()))
                .and_then(|v| v["choices"][0]["message"]["content"]
                    .as_str().map(|s| s.to_string())
                    .ok_or_else(|| "No content in response".to_string()));
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn call_vlm(&mut self, cx: &mut Cx, model_id: String, image_path: String, user: String) {
        if self.vlm_state.is_running { return; }
        self.vlm_state.is_running = true;
        self.view.label(ids!(hub_vlm_panel.vlm_status)).set_text(cx, "Generating...");
        self.view.label(ids!(hub_vlm_panel.vlm_response.output_label)).set_text(cx, "");
        self.view.redraw(cx);

        let (tx, rx) = mpsc::channel();
        self.vlm_state.rx = Some(rx);
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120)).build().unwrap();
            let img_b64 = if !image_path.is_empty() {
                std::fs::read(&image_path).ok()
                    .map(|b| base64::engine::general_purpose::STANDARD.encode(&b))
            } else { None };
            let mut content = vec![serde_json::json!({"type": "text", "text": user})];
            if let Some(b64) = img_b64 {
                content.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": {"url": format!("data:image/jpeg;base64,{}", b64)}
                }));
            }
            let body = serde_json::json!({"model": model_id, "messages": [{"role": "user", "content": content}]});
            let result = client.post("http://localhost:8080/v1/chat/completions")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()))
                .and_then(|v| v["choices"][0]["message"]["content"]
                    .as_str().map(|s| s.to_string())
                    .ok_or_else(|| "No content in response".to_string()));
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn call_asr(&mut self, cx: &mut Cx, model_id: String, audio_path: String) {
        if self.asr_state.is_running { return; }
        if audio_path.is_empty() {
            self.view.label(ids!(hub_asr_panel.asr_status)).set_text(cx, "Enter an audio file path.");
            return;
        }
        self.asr_state.is_running = true;
        let is_wav = audio_path.to_lowercase().ends_with(".wav");
        let status_msg = if is_wav { "Transcribing..." } else { "Converting + transcribing..." };
        self.view.label(ids!(hub_asr_panel.asr_status)).set_text(cx, status_msg);
        self.view.label(ids!(hub_asr_panel.asr_transcript.output_label)).set_text(cx, "");
        self.view.redraw(cx);

        let (tx, rx) = mpsc::channel();
        self.asr_state.rx = Some(rx);
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(1800)).build().unwrap();

            // OminiX-API only accepts WAV. Convert non-WAV files using afconvert (macOS built-in).
            let (wav_path, is_temp) = if !audio_path.to_lowercase().ends_with(".wav") {
                let tmp = format!("/tmp/ominix_asr_{}.wav",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().as_millis());
                let output = std::process::Command::new("afconvert")
                    .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1", &audio_path, &tmp])
                    .output();
                match output {
                    Ok(o) if o.status.success() => (tmp, true),
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        let _ = tx.send(Err(format!("Format conversion failed: {}", stderr.trim())));
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(format!("afconvert not available: {}. Please convert to WAV first.", e)));
                        return;
                    }
                }
            } else {
                (audio_path.clone(), false)
            };

            // Send the WAV file path directly — OminiX-API reads it from disk (no size limit)
            let body = serde_json::json!({ "file": wav_path, "model": model_id });
            let result = client.post("http://localhost:8080/v1/audio/transcriptions")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| {
                    let status = r.status();
                    let text = r.text().map_err(|e| e.to_string())?;
                    if !status.is_success() {
                        return Err(format!("HTTP {}: {}", status, text.chars().take(300).collect::<String>()));
                    }
                    serde_json::from_str::<serde_json::Value>(&text)
                        .map_err(|e| format!("Bad JSON ({}): {}", e, text.chars().take(200).collect::<String>()))
                })
                .and_then(|v| v["text"].as_str().map(|s| s.to_string())
                    .ok_or_else(|| format!("No 'text' field in response: {}", v)));
            // Clean up temp WAV after the request completes
            if is_temp { let _ = std::fs::remove_file(&wav_path); }
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn call_tts(&mut self, cx: &mut Cx, model_id: String, voice_id: String, text: String) {
        if self.tts_state.is_running { return; }
        if text.is_empty() {
            self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, "Enter text to synthesize.");
            return;
        }
        self.tts_state.is_running = true;
        self.tts_state.output_path.clear();
        self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, "Generating audio...");
        self.view.view(ids!(hub_tts_panel.tts_result_row)).set_visible(cx, false);
        self.view.redraw(cx);

        let (tx, rx) = mpsc::channel();
        self.tts_state.rx = Some(rx);
        let voice = if voice_id.is_empty() { "vivian".to_string() } else { voice_id };
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120)).build().unwrap();
            let body = serde_json::json!({"model": model_id, "input": text, "voice": voice});
            let result = client.post("http://localhost:8080/v1/audio/speech")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| {
                    if !r.status().is_success() { return Err(format!("HTTP {}", r.status())); }
                    r.bytes().map_err(|e| e.to_string())
                })
                .and_then(|b| {
                    let out = "/tmp/ominix-hub-tts.wav";
                    std::fs::write(out, &b).map_err(|e| e.to_string())?;
                    std::process::Command::new("afplay").arg(out).spawn().map_err(|e| e.to_string())?;
                    Ok(())
                });
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn call_image(&mut self, cx: &mut Cx, model_id: String, prompt: String, neg_prompt: String) {
        if self.image_state.is_running { return; }
        if prompt.is_empty() {
            self.view.label(ids!(hub_image_panel.img_status)).set_text(cx, "Enter a prompt.");
            return;
        }
        self.image_state.is_running = true;
        self.view.label(ids!(hub_image_panel.img_status)).set_text(cx, "Generating image...");
        self.view.view(ids!(hub_image_panel.img_result_row)).set_visible(cx, false);
        self.view.image(ids!(hub_image_panel.img_preview)).set_visible(cx, false);
        self.view.redraw(cx);

        // Build a filename slug from the prompt (first 40 chars, alphanumeric + dash)
        let slug: String = prompt.chars()
            .take(40)
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let slug = if slug.is_empty() { "image".to_string() } else { slug };
        // Collapse runs of dashes
        let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
        let out_path = format!("/tmp/ominix-{}.png", slug);

        let (tx, rx) = mpsc::channel();
        self.image_state.rx = Some(rx);
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(300)).build().unwrap();
            let mut body = serde_json::json!({
                "model": model_id, "prompt": prompt,
                "n": 1, "size": "512x512", "response_format": "b64_json"
            });
            if !neg_prompt.is_empty() { body["negative_prompt"] = serde_json::Value::String(neg_prompt); }
            let result = client.post("http://localhost:8080/v1/images/generations")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()))
                .and_then(|v| {
                    let b64 = v["data"][0]["b64_json"].as_str()
                        .ok_or_else(|| "No image data".to_string())?;
                    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
                        .map_err(|e| e.to_string())?;
                    std::fs::write(&out_path, &bytes).map_err(|e| e.to_string())?;
                    Ok(out_path)
                });
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn call_image_edit(&mut self, cx: &mut Cx, model_id: String, image_path: String, prompt: String) {
        if self.image_edit_state.is_running { return; }
        if prompt.is_empty() {
            self.view.label(ids!(hub_image_edit_panel.img_edit_status)).set_text(cx, "Enter an edit instruction.");
            return;
        }
        if image_path.is_empty() {
            self.view.label(ids!(hub_image_edit_panel.img_edit_status)).set_text(cx, "Select a reference image.");
            return;
        }
        self.image_edit_state.is_running = true;
        self.view.label(ids!(hub_image_edit_panel.img_edit_status)).set_text(cx, "Starting server & editing image...");
        self.view.view(ids!(hub_image_edit_panel.img_edit_result_row)).set_visible(cx, false);
        self.view.image(ids!(hub_image_edit_panel.img_edit_preview)).set_visible(cx, false);
        self.view.redraw(cx);

        let slug: String = prompt.chars().take(40)
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>().trim_matches('-').to_string();
        let slug = if slug.is_empty() { "edit".to_string() } else { slug };
        let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
        let out_path = format!("/tmp/ominix-edit-{}.png", slug);

        let (tx, rx) = mpsc::channel();
        self.image_edit_state.rx = Some(rx);
        std::thread::spawn(move || {
            // Ensure ominix-api is running (sd.cpp models don't go through load_model)
            if let Err(e) = ensure_server_running() {
                let _ = tx.send(Err(format!("Failed to start server: {}", e)));
                return;
            }

            // Read reference image and encode as base64
            let image_b64 = match std::fs::read(&image_path) {
                Ok(bytes) => base64::engine::general_purpose::STANDARD.encode(&bytes),
                Err(e) => { let _ = tx.send(Err(format!("Failed to read image: {}", e))); return; }
            };

            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(600)).build().unwrap();
            let body = serde_json::json!({
                "model": model_id,
                "prompt": prompt,
                "image": image_b64,
                "n": 1, "size": "512x512", "response_format": "b64_json"
            });
            let result = client.post("http://localhost:8080/v1/images/generations")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| {
                    let status = r.status();
                    let text = r.text().map_err(|e| e.to_string())?;
                    if !status.is_success() {
                        return Err(format!("HTTP {}: {}", status, text));
                    }
                    serde_json::from_str::<serde_json::Value>(&text).map_err(|e| e.to_string())
                })
                .and_then(|v| {
                    // Check for API error response
                    if let Some(err) = v.get("error") {
                        return Err(format!("API error: {}", err));
                    }
                    let b64 = v["data"][0]["b64_json"].as_str()
                        .ok_or_else(|| format!("No image data in response: {}", v))?;
                    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
                        .map_err(|e| e.to_string())?;
                    std::fs::write(&out_path, &bytes).map_err(|e| e.to_string())?;
                    Ok(out_path)
                });
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn call_video(&mut self, cx: &mut Cx, model_id: String, prompt: String) {
        if self.video_state.is_running { return; }
        if prompt.is_empty() {
            self.view.label(ids!(hub_video_panel.vid_status)).set_text(cx, "Enter a prompt.");
            return;
        }
        self.video_state.is_running = true;
        self.view.label(ids!(hub_video_panel.vid_status)).set_text(cx, "Generating video — this may take a few minutes...");
        self.view.view(ids!(hub_video_panel.vid_result_row)).set_visible(cx, false);
        self.view.redraw(cx);

        let slug: String = prompt.chars()
            .take(40)
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let slug = if slug.is_empty() { "video".to_string() } else { slug };
        let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
        let out_path = format!("/tmp/ominix-{}.mp4", slug);

        let (tx, rx) = mpsc::channel();
        self.video_state.rx = Some(rx);
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(600)).build().unwrap();
            let body = serde_json::json!({
                "model": model_id,
                "prompt": prompt,
                "response_format": "b64_json"
            });
            let result = client.post("http://localhost:8080/v1/videos/generations")
                .json(&body).send()
                .map_err(|e| e.to_string())
                .and_then(|r| {
                    let status = r.status();
                    let text = r.text().map_err(|e| e.to_string())?;
                    if !status.is_success() {
                        return Err(format!("HTTP {}: {}", status, text));
                    }
                    serde_json::from_str::<serde_json::Value>(&text).map_err(|e| e.to_string())
                })
                .and_then(|v| {
                    if let Some(err) = v.get("error") {
                        return Err(format!("API error: {}", err));
                    }
                    let b64 = v["data"][0]["b64_json"].as_str()
                        .ok_or_else(|| format!("No video data in response: {}", v))?;
                    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
                        .map_err(|e| e.to_string())?;
                    std::fs::write(&out_path, &bytes).map_err(|e| e.to_string())?;
                    Ok(out_path)
                });
            let _ = tx.send(result);
        });
        cx.new_next_frame();
    }

    fn load_tts_voices(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.tts_state.voices_rx = Some(rx);
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(10)).build().unwrap();
            let result = client.get("http://localhost:8080/v1/voices").send()
                .map_err(|e| e.to_string())
                .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()))
                .map(|v| v["voices"].as_array()
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                    .unwrap_or_default());
            let _ = tx.send(result);
        });
    }
}

// ─── Download operations ──────────────────────────────────────────────────────

impl ModelHubApp {
    fn start_download(&mut self, cx: &mut Cx, model_id: &str) {
        if self.model_states.get(model_id).copied() == Some(ModelUiState::Downloading) {
            return;
        }

        let Some(model) = self.registry.as_ref()
            .and_then(|r| r.models.iter().find(|m| m.id == model_id)).cloned()
        else { return };
        if model.source.kind == SourceKind::Manual { return; }

        let ds = self.download_states
            .entry(model_id.to_string()).or_insert_with(ModelDownloadState::new).clone();
        ds.reset();
        ds.is_downloading.store(true, Ordering::SeqCst);

        self.model_states.insert(model_id.to_string(), ModelUiState::Downloading);
        self.refresh_header_for(cx, model_id);
        cx.new_next_frame();

        let model_id_owned = model_id.to_string();
        let local_path     = expand_tilde(&model.storage.local_path);
        let source_kind    = model.source.kind;
        let source_url     = model.source.url.clone().unwrap_or_default();
        let repo_id        = model.source.repo_id.clone().unwrap_or_default();
        let revision       = model.source.revision.clone();
        // Collect extra sources for multi-file models
        let extra_sources: Vec<(String, String)> = model.extra_sources.iter()
            .filter_map(|es| {
                es.source.url.as_ref().map(|url| {
                    (url.clone(), expand_tilde(&es.storage.local_path))
                })
            })
            .collect();

        std::thread::spawn(move || {
            let client = match reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(3600)).build()
            {
                Ok(c) => c,
                Err(e) => {
                    *ds.error_msg.lock().unwrap() = e.to_string();
                    ds.failed.store(true, Ordering::SeqCst);
                    ds.is_downloading.store(false, Ordering::SeqCst);
                    return;
                }
            };
            // Download primary source
            let result = match source_kind {
                SourceKind::HuggingFace => download_hf(&client, &repo_id, &revision, &local_path, &ds),
                SourceKind::ModelScope  => download_ms(&client, &repo_id, &revision, &local_path, &ds),
                SourceKind::DirectUrl   => download_direct_url(&client, &source_url, &local_path, &ds),
                _                       => Err("Source not supported".to_string()),
            };
            if let Err(e) = result {
                *ds.error_msg.lock().unwrap() = e;
                ds.failed.store(true, Ordering::SeqCst);
                ds.is_downloading.store(false, Ordering::SeqCst);
                return;
            }
            // Download extra sources sequentially
            for (url, dir) in extra_sources {
                if ds.cancel_requested.load(Ordering::SeqCst) { break; }
                if let Err(e) = download_direct_url(&client, &url, &dir, &ds) {
                    *ds.error_msg.lock().unwrap() = e;
                    ds.failed.store(true, Ordering::SeqCst);
                    ds.is_downloading.store(false, Ordering::SeqCst);
                    return;
                }
            }
            ds.completed.store(true, Ordering::SeqCst);
            ds.is_downloading.store(false, Ordering::SeqCst);
            ::log::info!("Download finished: {}", model_id_owned);
        });
    }

    fn poll_downloads(&mut self, cx: &mut Cx) {
        let mut keep = false;
        let mut done:   Vec<String>         = Vec::new();
        let mut failed: Vec<(String, String)> = Vec::new();

        for (id, ds) in &self.download_states {
            if ds.is_downloading.load(Ordering::SeqCst) { keep = true; }
            if ds.completed.load(Ordering::SeqCst) { done.push(id.clone()); }
            else if ds.failed.load(Ordering::SeqCst) {
                failed.push((id.clone(), ds.error_msg.lock().unwrap().clone()));
            }
        }

        let has_status_updates = !done.is_empty() || !failed.is_empty();

        for id in done {
            self.model_states.insert(id.clone(), ModelUiState::Downloaded);
            self.download_states.remove(&id);
            if self.selected_id.as_deref() == Some(id.as_str()) {
                self.refresh_header_for(cx, &id);
            }
        }
        for (id, err) in failed {
            self.model_states.insert(id.clone(), ModelUiState::Error);
            self.download_states.remove(&id);
            if self.selected_id.as_deref() == Some(id.as_str()) {
                self.refresh_header_for(cx, &id);
            }
            ::log::error!("Download error for {}: {}", id, err);
        }

        if has_status_updates {
            self.rebuild_list();
            self.view.redraw(cx);
        }

        // Live progress for the selected model
        if let Some(sel) = self.selected_id.clone() {
            if let Some(ds) = self.download_states.get(sel.as_str()) {
                if ds.is_downloading.load(Ordering::SeqCst) {
                    let pct = ds.fraction();
                    let txt = ds.progress_text();
                    let panel = self.active_panel;
                    match panel {
                        ActivePanel::Llm => {
                            self.view.view(ids!(hub_llm_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_llm_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::Vlm => {
                            self.view.view(ids!(hub_vlm_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_vlm_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::Asr => {
                            self.view.view(ids!(hub_asr_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_asr_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::Tts => {
                            self.view.view(ids!(hub_tts_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_tts_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::Image => {
                            self.view.view(ids!(hub_image_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_image_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::ImageEdit => {
                            self.view.view(ids!(hub_image_edit_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_image_edit_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::Video => {
                            self.view.view(ids!(hub_video_panel.hub_panel_header.panel_progress_fill)).apply_over(cx, live! { draw_bg: { progress: (pct) } });
                            self.view.label(ids!(hub_video_panel.hub_panel_header.panel_progress_text)).set_text(cx, &txt);
                        }
                        ActivePanel::Voice | ActivePanel::Info | ActivePanel::None => {}
                    }
                    self.view.redraw(cx);
                }
            }
        }

        if keep { cx.new_next_frame(); }
    }

    // ── Panel channel poll ────────────────────────────────────────────────────

    fn poll_panel_channels(&mut self, cx: &mut Cx) {
        let mut redraw = false;

        macro_rules! poll_string_rx {
            ($state:expr, $label:expr, $status:expr) => {
                if $state.is_running {
                    if let Some(rx) = &$state.rx {
                        if let Ok(result) = rx.try_recv() {
                            match result {
                                Ok(t)  => { self.view.label($label).set_text(cx, &t);
                                            self.view.label($status).set_text(cx, "Done."); }
                                Err(e) => { self.view.label($status).set_text(cx, &format!("Error: {}", e)); }
                            }
                            $state.is_running = false;
                            $state.rx = None;
                            redraw = true;
                        } else { cx.new_next_frame(); }
                    }
                }
            };
        }

        poll_string_rx!(self.llm_state,
            ids!(hub_llm_panel.llm_response.output_label),
            ids!(hub_llm_panel.llm_status));
        poll_string_rx!(self.vlm_state,
            ids!(hub_vlm_panel.vlm_response.output_label),
            ids!(hub_vlm_panel.vlm_status));
        poll_string_rx!(self.asr_state,
            ids!(hub_asr_panel.asr_transcript.output_label),
            ids!(hub_asr_panel.asr_status));
        // Image gen: custom poll so we can show preview + result row
        if self.image_state.is_running {
            if let Some(rx) = &self.image_state.rx {
                if let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(path) => {
                            self.view.label(ids!(hub_image_panel.img_status)).set_text(cx, "Done.");
                            self.view.label(ids!(hub_image_panel.img_output_path)).set_text(cx, &path);
                            self.view.view(ids!(hub_image_panel.img_result_row)).set_visible(cx, true);
                            // Load image into preview widget
                            let img_ref = self.view.image(ids!(hub_image_panel.img_preview));
                            if img_ref.load_image_file_by_path(cx, std::path::Path::new(&path)).is_ok() {
                                img_ref.set_visible(cx, true);
                            }
                            self.image_state.output_path = path;
                        }
                        Err(e) => {
                            self.view.label(ids!(hub_image_panel.img_status)).set_text(cx, &format!("Error: {}", e));
                        }
                    }
                    self.image_state.is_running = false;
                    self.image_state.rx = None;
                    redraw = true;
                } else { cx.new_next_frame(); }
            }
        }
        // Image edit: custom poll so we can show preview + result row
        if self.image_edit_state.is_running {
            if let Some(rx) = &self.image_edit_state.rx {
                if let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(path) => {
                            self.view.label(ids!(hub_image_edit_panel.img_edit_status)).set_text(cx, "Done.");
                            self.view.label(ids!(hub_image_edit_panel.img_edit_output_path)).set_text(cx, &path);
                            self.view.view(ids!(hub_image_edit_panel.img_edit_result_row)).set_visible(cx, true);
                            let img_ref = self.view.image(ids!(hub_image_edit_panel.img_edit_preview));
                            if img_ref.load_image_file_by_path(cx, std::path::Path::new(&path)).is_ok() {
                                img_ref.set_visible(cx, true);
                            }
                            self.image_edit_state.output_path = path;
                        }
                        Err(e) => {
                            self.view.label(ids!(hub_image_edit_panel.img_edit_status)).set_text(cx, &format!("Error: {}", e));
                        }
                    }
                    self.image_edit_state.is_running = false;
                    self.image_edit_state.rx = None;
                    redraw = true;
                } else { cx.new_next_frame(); }
            }
        }

        // Video gen: show result row on success
        if self.video_state.is_running {
            if let Some(rx) = &self.video_state.rx {
                if let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(path) => {
                            self.view.label(ids!(hub_video_panel.vid_status)).set_text(cx, "Done.");
                            self.view.label(ids!(hub_video_panel.vid_output_path)).set_text(cx, &path);
                            self.view.view(ids!(hub_video_panel.vid_result_row)).set_visible(cx, true);
                            self.video_state.output_path = path;
                        }
                        Err(e) => {
                            self.view.label(ids!(hub_video_panel.vid_status)).set_text(cx, &format!("Error: {}", e));
                        }
                    }
                    self.video_state.is_running = false;
                    self.video_state.rx = None;
                    redraw = true;
                } else { cx.new_next_frame(); }
            }
        }

        // TTS (returns ())
        if self.tts_state.is_running {
            if let Some(rx) = &self.tts_state.rx {
                if let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(()) => {
                            self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, "Playing...");
                            self.tts_state.output_path = "/tmp/ominix-hub-tts.wav".to_string();
                            self.view.view(ids!(hub_tts_panel.tts_result_row)).set_visible(cx, true);
                            self.view.button(ids!(hub_tts_panel.tts_result_row.tts_save_btn)).set_visible(cx, true);
                            self.view.button(ids!(hub_tts_panel.tts_result_row.tts_finder_btn)).set_visible(cx, false);
                        }
                        Err(e) => { self.view.label(ids!(hub_tts_panel.tts_status)).set_text(cx, &format!("Error: {}", e)); }
                    }
                    self.tts_state.is_running = false;
                    self.tts_state.rx = None;
                    redraw = true;
                } else { cx.new_next_frame(); }
            }
        }

        // TTS voices
        let voices_done = if let Some(rx) = &self.tts_state.voices_rx {
            match rx.try_recv() {
                Ok(Ok(voices)) => {
                    self.tts_state.voices = voices;
                    redraw = true;
                    true
                }
                Ok(Err(_)) => true,
                Err(_)     => false,
            }
        } else { false };
        if voices_done { self.tts_state.voices_rx = None; }

        if redraw { self.view.redraw(cx); }
    }
}

// ─── Filesystem helpers ───────────────────────────────────────────────────────

fn scan_state(model: &RegistryModel) -> ModelUiState {
    let p = expand_tilde(&model.storage.local_path);
    let path = Path::new(&p);
    if !path.exists() { return ModelUiState::NotDownloaded; }

    // For multi-source models, verify all required files are present
    if !model.extra_sources.is_empty() {
        // Check primary file from URL
        if let Some(url) = &model.source.url {
            let filename = url.split('/').last().and_then(|f| f.split('?').next()).unwrap_or("");
            if !filename.is_empty() && !path.join(filename).exists() {
                return ModelUiState::NotDownloaded;
            }
        }
        // Check each extra source file
        for extra in &model.extra_sources {
            if let Some(url) = &extra.source.url {
                let filename = url.split('/').last().and_then(|f| f.split('?').next()).unwrap_or("");
                let dir = expand_tilde(&extra.storage.local_path);
                if !filename.is_empty() && !Path::new(&dir).join(filename).exists() {
                    return ModelUiState::NotDownloaded;
                }
            }
        }
        return ModelUiState::Downloaded;
    }

    // For models with a known size > 100 MB, require at least one weight file
    // (.safetensors, .bin, or .gguf) to exist — prevents partial downloads (e.g. only
    // config.json fetched by the HF library) from showing as "Downloaded".
    // For GGUF models, also check file size against expected to avoid false positives
    // when multiple quant variants share the same HuggingFace repo directory.
    if model.storage.size_bytes > 100 * 1024 * 1024 {
        return if has_weight_files(path, model.storage.size_bytes) { ModelUiState::Downloaded } else { ModelUiState::NotDownloaded };
    }
    let n = std::fs::read_dir(path)
        .map(|e| e.filter_map(|x| x.ok())
             .filter(|x| !x.file_name().to_string_lossy().starts_with('.')).count())
        .unwrap_or(0);
    if n > 0 { ModelUiState::Downloaded } else { ModelUiState::NotDownloaded }
}

/// Recursively search for weight files (.safetensors, .bin, .gguf) under `dir`.
/// For .gguf files, compares actual size against `expected_size` (5% tolerance)
/// to avoid false positives when multiple quant variants share a directory.
fn has_weight_files(dir: &Path, expected_size: u64) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false; };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            if name.ends_with(".gguf") {
                if expected_size > 0 {
                    let actual = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    let tolerance = expected_size / 20; // 5%
                    if (actual as i64 - expected_size as i64).unsigned_abs() <= tolerance {
                        return true;
                    }
                } else {
                    return true;
                }
            } else if name.ends_with(".safetensors") || name.ends_with(".bin") {
                return true;
            }
        } else if p.is_dir() && has_weight_files(&p, expected_size) {
            return true;
        }
    }
    false
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn hf_token() -> Option<String> {
    let home = dirs::home_dir()?;
    // Check both token locations: newer ~/.cache/huggingface/token and legacy ~/.huggingface/hub/token
    for p in [
        home.join(".cache").join("huggingface").join("token"),
        home.join(".huggingface").join("hub").join("token"),
    ] {
        if let Ok(t) = std::fs::read_to_string(&p) {
            let t = t.trim().to_string();
            if !t.is_empty() { return Some(t); }
        }
    }
    None
}

// ─── HuggingFace download ─────────────────────────────────────────────────────

fn download_hf(
    client: &reqwest::blocking::Client,
    repo_id: &str, revision: &str, local_path: &str,
    ds: &ModelDownloadState,
) -> Result<(), String> {
    // Use ?blobs=true to get all files recursively (including subdirectories) with sizes
    let url = format!("https://huggingface.co/api/models/{}?blobs=true", repo_id);
    let mut req = client.get(&url);
    if let Some(tok) = hf_token() { req = req.header("Authorization", format!("Bearer {}", tok)); }
    let resp = req.send().map_err(|e| e.to_string())?;
    if resp.status() == 401 {
        return Err("Access denied — model requires HuggingFace authentication. Accept the license at huggingface.co and add your token to ~/.huggingface/hub/token".to_string());
    }
    if !resp.status().is_success() { return Err(format!("HF API {}", resp.status())); }
    let body: HfBlobsResponse = resp.json().map_err(|e| e.to_string())?;
    let files: Vec<(String, u64)> = body.siblings.into_iter()
        .filter(|s| !s.rfilename.starts_with('.'))
        .map(|s| (s.rfilename, s.size.unwrap_or(0)))
        .collect();
    if files.is_empty() { return Err("No files in repo".to_string()); }

    ds.total_bytes.store(files.iter().map(|(_, s)| s).sum(), Ordering::SeqCst);
    let mut done = 0u64;
    for (path, _) in &files {
        if ds.cancel_requested.load(Ordering::SeqCst) { return Err("Cancelled".to_string()); }
        let file_url = format!("https://huggingface.co/{}/resolve/{}/{}", repo_id, revision, path);
        let dest = PathBuf::from(local_path).join(path);
        // Create parent directories for nested paths (e.g. transformer/model.safetensors)
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        *ds.current_file.lock().unwrap() = path.clone();
        done += stream_download(client, &file_url, &dest, &ds.cancel_requested, &ds.progress_bytes, done)?;
    }
    Ok(())
}

// ─── Direct URL download (single file) ────────────────────────────────────────

/// Download a single file from `url` into `local_dir`, using the URL filename.
fn download_direct_url(
    client: &reqwest::blocking::Client,
    url: &str,
    local_dir: &str,
    ds: &ModelDownloadState,
) -> Result<(), String> {
    let filename = url.split('/').last()
        .filter(|s| !s.is_empty())
        .unwrap_or("file");
    // Strip query params if any
    let filename = filename.split('?').next().unwrap_or(filename);
    let dest = PathBuf::from(local_dir).join(filename);
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    *ds.current_file.lock().unwrap() = filename.to_string();
    // HEAD request to get Content-Length for progress tracking
    let mut req = client.head(url);
    if let Some(tok) = hf_token() { req = req.header("Authorization", format!("Bearer {}", tok)); }
    if let Ok(resp) = req.send() {
        if let Some(len) = resp.headers().get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            let current_total = ds.total_bytes.load(Ordering::SeqCst);
            ds.total_bytes.store(current_total + len, Ordering::SeqCst);
        }
    }
    let done_before = ds.progress_bytes.load(Ordering::SeqCst);
    stream_download(client, url, &dest, &ds.cancel_requested, &ds.progress_bytes, done_before)?;
    Ok(())
}

// ─── ModelScope download ──────────────────────────────────────────────────────

fn download_ms(
    client: &reqwest::blocking::Client,
    repo_id: &str, revision: &str, local_path: &str,
    ds: &ModelDownloadState,
) -> Result<(), String> {
    let url = format!(
        "https://modelscope.cn/api/v1/models/{}/repo/files?Revision={}&Recursive=true",
        repo_id, revision
    );
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    let ms: MsResponse = resp.json().map_err(|e| e.to_string())?;
    if ms.code != 200 { return Err(format!("ModelScope code {}", ms.code)); }
    let data = ms.data.ok_or_else(|| "empty data".to_string())?;
    let files: Vec<(String, u64)> = data.files.into_iter()
        .filter(|f| f.file_type == "blob").map(|f| (f.path, f.size)).collect();

    ds.total_bytes.store(files.iter().map(|(_, s)| s).sum(), Ordering::SeqCst);
    let mut done = 0u64;
    for (path, _) in &files {
        if ds.cancel_requested.load(Ordering::SeqCst) { return Err("Cancelled".to_string()); }
        let file_url = format!(
            "https://modelscope.cn/api/v1/models/{}/repo?Revision={}&FilePath={}",
            repo_id, revision, path
        );
        let dest = PathBuf::from(local_path).join(path);
        *ds.current_file.lock().unwrap() = path.clone();
        done += stream_download(client, &file_url, &dest, &ds.cancel_requested, &ds.progress_bytes, done)?;
    }
    Ok(())
}

// ─── Stream helper ────────────────────────────────────────────────────────────

fn stream_download(
    client: &reqwest::blocking::Client,
    url: &str, dest: &Path, cancel: &Arc<AtomicBool>,
    progress_bytes: &Arc<AtomicU64>, base_done: u64,
) -> Result<u64, String> {
    if let Some(p) = dest.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let mut req = client.get(url);
    if let Some(tok) = hf_token() { req = req.header("Authorization", format!("Bearer {}", tok)); }
    let mut resp = req.send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() { return Err(format!("HTTP {}", resp.status())); }

    let file_name = dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    ::log::info!("stream_download: starting {} from {}", file_name, url);
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut buf  = [0u8; 65536];
    let mut total = 0u64;
    let mut last_log = 0u64;
    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(file); let _ = std::fs::remove_file(dest);
            return Err("Cancelled".to_string());
        }
        match resp.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                total += n as u64;
                progress_bytes.store(base_done + total, Ordering::SeqCst);
                // Log every 50MB to verify streaming works
                if total / (50 * 1_048_576) > last_log / (50 * 1_048_576) {
                    ::log::info!("stream_download: {} — {} MB downloaded", file_name, total / 1_048_576);
                    last_log = total;
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(total)
}
