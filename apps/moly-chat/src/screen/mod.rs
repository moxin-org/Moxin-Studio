//! Chat Screen Widget Implementation

pub mod design;

use makepad_widgets::*;
use moly_kit::prelude::*;
use moly_kit::aitk::controllers::chat::{ChatStateMutation, ChatTask};
use moly_kit::aitk::protocol::{Bot, BotId, EntityAvatar};
use moly_kit::widgets::a2ui_client::A2uiClient;
use moly_kit::widgets::model_selector::BotGroup;
use moly_kit::widgets::prompt_input::PromptInputAction;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};

use moly_data::{ChatId, Store};
use moly_data::model_registry::RegistryCategory;

static TTS_VOICE_IDS: &[&str] = &[
    "vivian", "serena", "ryan", "aiden", "english_man",
    "uncle_fu", "chinese_woman", "chinese_man", "dialect",
];

/// Which mode the chat UI is in, based on the loaded model category.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
enum ChatMode {
    #[default]
    Llm,
    Vlm,
    Asr,
    Tts,
    ImageGen,
    VideoGen,
}

// Actions emitted by ChatHistoryPanel
#[derive(Clone, Debug, DefaultNone)]
pub enum ChatHistoryAction {
    None,
    NewChat,
    ChatCreated,
    SelectChat(ChatId),
    DeleteChat(ChatId),
}

/// ChatHistoryItem Widget - handles its own click events
#[derive(Live, LiveHook, Widget)]
pub struct ChatHistoryItem {
    #[deref]
    view: View,

    #[rust]
    chat_id: Option<ChatId>,
}

impl Widget for ChatHistoryItem {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ChatHistoryItem {
    pub fn set_chat_id(&mut self, id: ChatId) {
        self.chat_id = Some(id);
    }

    /// Check if this item was clicked (but not the delete button)
    pub fn clicked(&self, actions: &Actions) -> bool {
        // Don't count as clicked if delete button was clicked
        if self.delete_clicked(actions) {
            return false;
        }
        if let Some(item) = actions.find_widget_action(self.view.widget_uid()) {
            if let ViewAction::FingerDown(fd) = item.cast() {
                return fd.tap_count == 1;
            }
        }
        false
    }

    /// Check if the delete button was clicked
    pub fn delete_clicked(&self, actions: &Actions) -> bool {
        if let Some(item) = actions.find_widget_action(self.view.view(ids!(delete_button)).widget_uid()) {
            if let ViewAction::FingerDown(fd) = item.cast() {
                return fd.tap_count == 1;
            }
        }
        false
    }

    pub fn get_chat_id(&self) -> Option<ChatId> {
        self.chat_id
    }
}

impl ChatHistoryItemRef {
    pub fn set_chat_id(&self, id: ChatId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_chat_id(id);
        }
    }

    pub fn clicked(&self, actions: &Actions) -> bool {
        if let Some(inner) = self.borrow() {
            inner.clicked(actions)
        } else {
            false
        }
    }

    pub fn delete_clicked(&self, actions: &Actions) -> bool {
        if let Some(inner) = self.borrow() {
            inner.delete_clicked(actions)
        } else {
            false
        }
    }

    pub fn get_chat_id(&self) -> Option<ChatId> {
        if let Some(inner) = self.borrow() {
            inner.get_chat_id()
        } else {
            None
        }
    }
}

/// Separate widget for chat history panel - handles its own PortalList drawing
#[derive(Live, LiveHook, Widget)]
pub struct ChatHistoryPanel {
    #[deref]
    view: View,

    #[rust]
    chat_count: usize,

    #[rust]
    current_chat_id: Option<ChatId>,
}

impl Widget for ChatHistoryPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // Delegate events directly to view (like moly-ai pattern)
        self.view.handle_event(cx, event, scope);

        // Use WidgetMatchEvent pattern for handling actions
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Get data from store
        if let Some(store) = scope.data.get::<Store>() {
            self.chat_count = store.chats.saved_chats.len();
        }

        // Get the history_list PortalList
        let history_list = self.view.portal_list(ids!(history_list));
        let history_list_uid = history_list.widget_uid();

        // Draw with PortalList handling
        while let Some(widget) = self.view.draw_walk(cx, scope, walk).step() {
            if widget.widget_uid() == history_list_uid {
                if let Some(mut list) = widget.as_portal_list().borrow_mut() {
                    list.set_item_range(cx, 0, self.chat_count);

                    while let Some(item_id) = list.next_visible_item(cx) {
                        if item_id < self.chat_count {
                            // Get chat data
                            let (chat_id, title, date_str, is_selected, category) = if let Some(store) = scope.data.get::<Store>() {
                                if let Some(chat) = store.chats.saved_chats.get(item_id) {
                                    let id = chat.id;
                                    let title = chat.title.clone();
                                    let date = chat.accessed_at.format("%b %d").to_string();
                                    let selected = self.current_chat_id == Some(chat.id);
                                    let cat = chat.model_category;
                                    (id, title, date, selected, cat)
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            };

                            // Draw the item - get as ChatHistoryItem widget
                            let item_widget = list.item(cx, item_id, live_id!(ChatHistoryItem));

                            // Set the chat_id on the item so we can retrieve it in handle_actions
                            item_widget.as_chat_history_item().set_chat_id(chat_id);

                            let selected_value = if is_selected { 1.0 } else { 0.0 };

                            item_widget.apply_over(cx, live! {
                                draw_bg: {
                                    selected: (selected_value)
                                }
                            });

                            item_widget.label(ids!(content.title_label)).set_text(cx, &title);

                            item_widget.label(ids!(content.category_row.date_label)).set_text(cx, &date_str);

                            if let Some(cat) = category {
                                let tag = item_widget.view(ids!(content.category_row.category_tag));
                                tag.set_visible(cx, true);
                                item_widget.label(ids!(content.category_row.category_tag.tag_label))
                                    .set_text(cx, cat.label());
                                let hex = cat.color().trim_start_matches('#');
                                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
                                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
                                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;
                                tag.apply_over(cx, live! { draw_bg: { color: (vec4(r as f32, g as f32, b as f32, 1.0)) } });
                            } else {
                                item_widget.view(ids!(content.category_row.category_tag))
                                    .set_visible(cx, false);
                            }

                            item_widget.draw_all(cx, scope);
                        }
                    }
                }
            }
        }

        DrawStep::done()
    }
}

impl ChatHistoryPanel {
    pub fn set_current_chat(&mut self, chat_id: Option<ChatId>) {
        self.current_chat_id = chat_id;
    }
}

impl WidgetMatchEvent for ChatHistoryPanel {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // Handle new chat button click
        let btn = self.button(ids!(new_chat_button));
        if btn.clicked(actions) {
            ::log::info!("New chat button clicked");
            cx.action(ChatHistoryAction::NewChat);
        }

        // Handle chat history item clicks from PortalList
        // Use the ChatHistoryItem widget's clicked() method (like moly-ai's EntityButton pattern)
        let history_list = self.portal_list(ids!(history_list));
        for (_item_id, item) in history_list.items_with_actions(actions) {
            let history_item = item.as_chat_history_item();

            // Check for delete button click first
            if history_item.delete_clicked(actions) {
                if let Some(chat_id) = history_item.get_chat_id() {
                    ::log::info!("Delete button clicked for chat: {:?}", chat_id);
                    cx.action(ChatHistoryAction::DeleteChat(chat_id));
                }
            }
            // Then check for item click (select chat)
            else if history_item.clicked(actions) {
                if let Some(chat_id) = history_item.get_chat_id() {
                    ::log::info!("Chat history item clicked: {:?}", chat_id);
                    cx.action(ChatHistoryAction::SelectChat(chat_id));
                }
            }
        }
    }
}

impl ChatHistoryPanelRef {
    pub fn set_current_chat(&self, chat_id: Option<ChatId>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_current_chat(chat_id);
        }
    }
}

#[derive(Live, Widget)]
pub struct ChatApp {
    #[deref]
    pub view: View,

    /// Provider icons loaded from live_design for use in model selector and chat messages
    #[live]
    provider_icons: Vec<LiveDependency>,

    // We create our own controller and set it on the Chat widget
    #[rust(ChatController::new_arc())]
    chat_controller: Arc<Mutex<ChatController>>,

    #[rust]
    controller_set_on_widget: bool,

    #[rust]
    providers_configured: bool,

    #[rust]
    current_provider_id: Option<String>,

    /// Track which providers we've already fetched models from
    #[rust]
    fetched_provider_ids: Vec<String>,

    /// List of providers to fetch models from (in order)
    #[rust]
    providers_to_fetch: Vec<String>,

    /// Index of the provider currently being fetched
    #[rust]
    fetch_index: usize,

    /// Whether we're currently waiting for a model fetch to complete
    #[rust]
    fetch_in_progress: bool,

    /// Number of bots we last saw from the current fetch
    #[rust]
    last_bots_count: usize,

    /// Track the last saved bot_id to detect changes
    #[rust]
    last_saved_bot_id: Option<String>,

    /// Whether we've restored the saved model selection
    #[rust]
    restored_saved_model: bool,

    /// Whether we need to force re-set the controller (after models load or visibility change)
    #[rust]
    needs_controller_reset: bool,

    /// Whether we need to create a new chat (set by request_new_chat, handled in handle_event)
    #[rust]
    needs_new_chat: bool,

    /// Current chat ID being edited
    #[rust]
    current_chat_id: Option<ChatId>,

    /// Last message count we synced (to detect changes)
    #[rust]
    last_synced_message_count: usize,

    /// A2UI client wrapper - wraps the actual client to inject A2UI tools when enabled
    #[rust]
    a2ui_client: Option<A2uiClient>,

    /// Whether there was a message being written in the last sync check
    #[rust]
    had_writing_message: bool,

    /// Content length of last message at last sync (to detect streaming content)
    #[rust]
    last_synced_content_len: usize,

    /// Whether we've initialized the chat from persistence
    #[rust]
    chat_initialized: bool,

    /// Whether we're in welcome mode (centered input) - only changes on explicit send
    /// Defaults to true so new chats start in welcome mode
    #[rust(true)]
    in_welcome_mode: bool,

    /// Whether we've set the controller on the welcome prompt
    #[rust]
    welcome_prompt_controller_set: bool,

    /// Tracks the last active_local_model value we handled, to detect changes
    #[rust]
    last_active_local_model: Option<String>,

    // ── Mode-specific state (ASR / TTS / Image) ────────────────────────
    /// Current chat mode based on loaded model category
    #[rust]
    chat_mode: ChatMode,

    /// ASR: path to the selected audio file
    #[rust]
    asr_file_path: String,

    /// ASR/TTS/Image: receiver for async operation results
    #[rust]
    mode_rx: Option<mpsc::Receiver<Result<String, String>>>,

    /// ASR: receiver for file picker result
    #[rust]
    file_picker_rx: Option<mpsc::Receiver<Result<String, String>>>,

    /// Whether a mode-specific operation is in progress
    #[rust]
    mode_busy: bool,

    /// VLM: path to the selected image file
    #[rust]
    vlm_image_path: String,

    /// VLM: base64-encoded image (ready for API)
    #[rust]
    vlm_image_b64: Option<String>,

    /// Image edit: base64-encoded reference image (set by file picker)
    #[rust]
    image_ref_b64: Option<String>,

    /// Image edit: path of the selected reference image
    #[rust]
    image_ref_path: String,

    /// TTS: selected voice index (maps to TTS_VOICE_IDS)
    #[rust]
    tts_voice_idx: usize,

    /// TTS: path to the last generated audio file
    #[rust]
    tts_audio_path: Option<String>,

    /// TTS: whether audio is currently playing
    #[rust]
    tts_playing: bool,

    /// TTS: child process handle for afplay (so we can kill it on stop)
    #[rust]
    tts_play_process: Option<std::process::Child>,

    /// TTS: audio duration in seconds
    #[rust]
    tts_duration_secs: f64,

    /// TTS: playback start instant
    #[rust]
    tts_play_start: Option<std::time::Instant>,

    /// Tracks the last message count we checked for mode-specific interception
    #[rust]
    last_mode_msg_count: usize,

    /// Skip drawing the Chat widget for N frames after a client switch
    /// to let the controller process mutations before draw
    #[rust]
    skip_chat_draw_frames: u32,
}

impl LiveHook for ChatApp {
    fn after_new_from_doc(&mut self, _cx: &mut Cx) {
        // Initialize the controller with basic spawner
        let mut controller = self.chat_controller.lock().unwrap();
        controller.set_basic_spawner();
    }
}

impl ChatApp {
    /// Get provider icon LiveDependency from the loaded list
    fn get_provider_icon(&self, provider_id: &str) -> Option<&LiveDependency> {
        let index = match provider_id {
            "openai" | "openai-realtime" => Some(0),
            "anthropic" => Some(1),
            "gemini" => Some(2),
            "ollama" => Some(3),
            "deepseek" => Some(4),
            "openrouter" => Some(5),
            "siliconflow" => Some(6),
            "nvidia" => Some(7),
            "groq" => Some(8),
            "zhipu" => Some(9),
            "ominix-image" => Some(3),
            "ominix-local" => Some(3), // local model: use the ollama/local icon
            _ => None,
        };
        index.and_then(|i| self.provider_icons.get(i))
    }

    /// Get provider icon path string from the loaded LiveDependency list
    fn get_provider_icon_path(&self, provider_id: &str) -> Option<String> {
        self.get_provider_icon(provider_id).map(|dep| dep.as_str().to_string())
    }

    /// Get provider display name
    fn get_provider_display_name(provider_id: &str) -> &'static str {
        match provider_id {
            "openai" => "OpenAI",
            "openai-realtime" => "OpenAI Realtime",
            "anthropic" => "Anthropic",
            "gemini" => "Google Gemini",
            "ollama" => "Ollama",
            "deepseek" => "DeepSeek",
            "groq" => "Groq",
            "nvidia" => "NVIDIA",
            "openrouter" => "OpenRouter",
            "siliconflow" => "SiliconFlow",
            "zhipu" => "Zhipu AI",
            "ominix-image" => "OminiX Image",
            "ominix-local" => "OminiX Local",
            _ => "Unknown",
        }
    }

    /// Set up the grouping function for the model selector
    fn setup_model_selector_grouping(&mut self, scope: &mut Scope) {
        let Some(store) = scope.data.get::<Store>() else { return };

        // Build lookup table: BotId -> BotGroup
        let mut bot_groups: HashMap<BotId, BotGroup> = HashMap::new();

        for bot in store.providers_manager.get_all_bots() {
            // Get provider ID from ProvidersManager
            let (_, fallback_provider) = Self::parse_bot_id_string(bot.id.as_str());
            let provider_id = store.providers_manager.get_provider_for_bot(&bot.id)
                .map(|s| s.to_string())
                .unwrap_or(fallback_provider); // fallback to parsed provider if not found
            let provider_id = provider_id.as_str();

            let icon = self.get_provider_icon_path(provider_id)
                .map(|path| EntityAvatar::Image(path));
            let label = Self::get_provider_display_name(provider_id).to_string();

            bot_groups.insert(
                bot.id.clone(),
                BotGroup {
                    id: provider_id.to_string(),
                    label,
                    icon,
                },
            );
        }

        // Create grouping function that looks up bot groups by bot ID
        let grouping_fn = move |bot: &Bot| -> BotGroup {
            bot_groups.get(&bot.id).cloned().unwrap_or_else(|| BotGroup {
                id: "unknown".to_string(),
                label: "Unknown".to_string(),
                icon: None,
            })
        };

        // Set grouping on the ModelSelector inside PromptInput
        let chat = self.view.chat(ids!(main_content.chat));
        chat.read()
            .prompt_input_ref()
            .widget(ids!(model_selector))
            .as_model_selector()
            .set_grouping(grouping_fn);
    }

    /// Set our controller on the Chat widget if not already done
    fn maybe_set_controller_on_widget(&mut self, cx: &mut Cx) {
        if self.controller_set_on_widget {
            return;
        }

        let mut chat_ref = self.view.chat(ids!(main_content.chat));
        chat_ref.write().set_chat_controller(cx, Some(self.chat_controller.clone()));
        self.controller_set_on_widget = true;
    }

    /// Force re-set the controller on the Chat widget
    /// This handles visibility changes and ensures bots are properly propagated
    fn force_reset_controller_on_widget(&mut self, cx: &mut Cx) {
        let mut chat_ref = self.view.chat(ids!(main_content.chat));
        // Set to None first to bypass the same-pointer check
        chat_ref.write().set_chat_controller(cx, None);
        chat_ref.write().set_chat_controller(cx, Some(self.chat_controller.clone()));
    }

    /// Called by the parent App when this view becomes visible
    /// This triggers a controller reset to ensure the model list is populated
    pub fn on_become_visible(&mut self) {
        self.needs_controller_reset = true;
    }

    /// Request creation of a new chat. Called directly from parent App.
    /// This sets a flag that will be processed in handle_event.
    pub fn request_new_chat(&mut self) {
        self.needs_new_chat = true;
    }

    /// Load a chat by ID. Called from App when selecting a chat from history.
    pub fn load_chat(&mut self, chat_id: ChatId) {
        // Store the chat_id to be loaded - we'll handle it in handle_event
        // when we have access to Cx and Scope
        self.current_chat_id = Some(chat_id);
        self.chat_initialized = false; // Force re-initialization
        self.last_synced_message_count = 0;
        self.had_writing_message = false;
        self.last_synced_content_len = 0;
        self.last_mode_msg_count = 0;
    }

    /// Initialize the chat from persistence (load or create the current chat)
    fn maybe_initialize_chat(&mut self, cx: &mut Cx, scope: &mut Scope) {
        if self.chat_initialized {
            return;
        }
        ::log::info!("maybe_initialize_chat: RUNNING (chat_initialized was false)");

        let Some(store) = scope.data.get_mut::<Store>() else { return };

        // Get or create the current chat
        let chat_id = if let Some(id) = store.chats.current_chat_id {
            id
        } else {
            // No current chat, create one
            let current_bot_id = {
                let ctrl = self.chat_controller.lock().unwrap();
                ctrl.state().bot_id.clone()
            };
            ::log::info!("Creating new chat");
            store.chats.create_chat(current_bot_id)
        };

        self.current_chat_id = Some(chat_id);

        // Load messages from the chat into the controller
        if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
            let messages = chat.messages.clone();
            let message_count = messages.len();

            if !messages.is_empty() {
                ::log::info!("Loading {} messages from chat {}", message_count, chat_id);
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(VecMutation::Set(messages));
                // Has messages, not in welcome mode
                self.in_welcome_mode = false;
            } else {
                // No messages, show welcome mode
                self.in_welcome_mode = true;
            }

            self.last_synced_message_count = message_count;
            self.last_mode_msg_count = message_count;

            // Also restore the bot_id if it was saved with the chat
            if let Some(ref bot_id) = chat.bot_id {
                ::log::info!("Chat {} has saved bot_id: {}", chat_id, bot_id.as_str());
                // We'll let restore_saved_model handle the bot selection
            }
        } else {
            // No chat found, show welcome mode
            self.in_welcome_mode = true;
        }

        self.chat_initialized = true;
        self.view.redraw(cx);
    }

    /// Sync messages from controller to persistence when they change
    fn sync_messages_to_persistence(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(chat_id) = self.current_chat_id else { return };

        // Get current messages from controller
        let (messages, message_count, has_writing_message, last_msg_content_len) = {
            let ctrl = self.chat_controller.lock().unwrap();
            let msgs = ctrl.state().messages.clone();
            let count = msgs.len();
            // Check if any message is still being written
            let writing = msgs.iter().any(|m| m.metadata.is_writing);
            // Get the content length of the last message (to detect content changes)
            let last_len = msgs.last().map(|m| m.content.text.len()).unwrap_or(0);
            (msgs, count, writing, last_len)
        };

        // Sync if:
        // 1. Message count changed (new message added)
        // 2. OR there was a writing message that just finished (content now complete)
        // 3. OR the last message content has grown (streaming in progress or just finished)
        let count_changed = message_count != self.last_synced_message_count;
        let writing_finished = self.had_writing_message && !has_writing_message;
        let content_changed = last_msg_content_len != self.last_synced_content_len;

        if !count_changed && !writing_finished && !content_changed {
            return;
        }

        if count_changed {
            ::log::debug!("Messages count changed: {} -> {}, syncing to persistence",
                self.last_synced_message_count, message_count);
        }
        if writing_finished {
            ::log::debug!("Message finished streaming, syncing to persistence");
        }
        if content_changed {
            ::log::debug!("Message content changed: {} -> {} bytes, syncing to persistence",
                self.last_synced_content_len, last_msg_content_len);
        }

        // Update the chat in persistence
        if let Some(store) = scope.data.get_mut::<Store>() {
            store.chats.update_chat_messages(chat_id, messages);
        }

        // Notify shell to refresh sidebar when chat gets its first messages (title updates)
        if self.last_synced_message_count == 0 && message_count > 0 {
            cx.action(ChatHistoryAction::ChatCreated);
        }

        self.last_synced_message_count = message_count;
        self.had_writing_message = has_writing_message;
        self.last_synced_content_len = last_msg_content_len;
    }

    /// Sync the current bot_id to the chat when it changes
    fn sync_bot_to_chat(&mut self, scope: &mut Scope) {
        let Some(chat_id) = self.current_chat_id else { return };

        // Get current bot_id from controller
        let current_bot_id = {
            let ctrl = self.chat_controller.lock().unwrap();
            ctrl.state().bot_id.clone()
        };

        // Update the chat's bot_id; only set category if not already set
        // (category is assigned at session creation and should not change)
        if let Some(store) = scope.data.get_mut::<Store>() {
            let current_cat = store.get_active_local_model_category();
            let (needs_bot_update, needs_cat_set) = if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
                (chat.bot_id != current_bot_id, chat.model_category.is_none() && current_cat.is_some())
            } else {
                (false, false)
            };
            if needs_bot_update {
                store.chats.update_chat_bot(chat_id, current_bot_id);
            }
            if needs_cat_set {
                store.chats.update_chat_category(chat_id, current_cat);
            }
        }
    }

    /// Create a new chat session
    pub fn create_new_chat(&mut self, cx: &mut Cx, scope: &mut Scope) {
        // Skip if already in an empty welcome session (no messages sent yet)
        if self.in_welcome_mode {
            if let Some(chat_id) = self.current_chat_id {
                let ctrl = self.chat_controller.lock().unwrap();
                if ctrl.state().messages.is_empty() {
                    ::log::info!("Already in empty session {} — skipping new session creation", chat_id);
                    return;
                }
            }
        }

        let Some(store) = scope.data.get_mut::<Store>() else { return };

        // Get current bot_id and all bots to use for new chat
        let (current_bot_id, all_bots) = {
            let ctrl = self.chat_controller.lock().unwrap();
            (ctrl.state().bot_id.clone(), ctrl.state().bots.clone())
        };

        // Create new chat in store with the current model category
        let chat_id = store.chats.create_chat(current_bot_id.clone());
        let current_cat = store.get_active_local_model_category();
        if current_cat.is_some() {
            store.chats.update_chat_category(chat_id, current_cat);
        }
        self.current_chat_id = Some(chat_id);

        ::log::info!("=== NEW CHAT CREATED: {} (category: {:?}) ===", chat_id, current_cat);

        // Clear messages in controller - this triggers the Chat widget's plugin to redraw
        {
            let mut ctrl = self.chat_controller.lock().unwrap();
            let old_count = ctrl.state().messages.len();
            ::log::info!("Clearing {} messages from controller", old_count);
            ctrl.dispatch_mutation(VecMutation::<Message>::Set(vec![]));
            ctrl.dispatch_mutation(VecMutation::Set(all_bots));
            if let Some(bot_id) = current_bot_id {
                ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(bot_id)));
            }
        }

        // Reset sync tracking state
        self.last_synced_message_count = 0;
        self.had_writing_message = false;
        self.last_synced_content_len = 0;
        self.last_mode_msg_count = 0;

        // Mark as initialized to prevent maybe_initialize_chat from overwriting our state
        self.chat_initialized = true;

        // Enter welcome mode for new chat (centered input)
        self.in_welcome_mode = true;

        // Verify state after clearing
        let msg_count = {
            let ctrl = self.chat_controller.lock().unwrap();
            ctrl.state().messages.len()
        };
        ::log::info!("New chat {} ready - controller has {} messages", chat_id, msg_count);

        // Notify the shell to refresh sidebar chat list
        cx.action(ChatHistoryAction::ChatCreated);

        // Force redraw the entire view
        self.view.redraw(cx);
    }

    /// Switch to a different chat
    pub fn switch_to_chat(&mut self, cx: &mut Cx, scope: &mut Scope, chat_id: ChatId) {
        if self.current_chat_id == Some(chat_id) {
            return;
        }

        let Some(store) = scope.data.get_mut::<Store>() else { return };

        // Set as current chat in persistence
        store.chats.set_current_chat(Some(chat_id));
        self.current_chat_id = Some(chat_id);

        // Load the chat's messages into controller
        if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
            // Clone messages and reset is_writing flag on all of them
            // This is needed because in-memory messages may still have is_writing: true
            // from when they were being streamed, even though it's not persisted to disk
            let mut messages = chat.messages.clone();
            for msg in &mut messages {
                msg.metadata.is_writing = false;
            }
            let message_count = messages.len();
            let last_content_len = messages.last().map(|m| m.content.text.len()).unwrap_or(0);

            ::log::info!("Switching to chat {} with {} messages", chat_id, message_count);

            {
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(VecMutation::Set(messages));

                // Also restore the bot if saved with the chat
                if let Some(ref bot_id) = chat.bot_id {
                    ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(bot_id.clone())));
                }
            }

            // Reset all sync tracking state for the loaded chat
            self.last_synced_message_count = message_count;
            self.had_writing_message = false;
            self.last_synced_content_len = last_content_len;

            // Set welcome mode based on message count
            let new_welcome_mode = message_count == 0;
            ::log::info!("switch_to_chat: setting in_welcome_mode={} (message_count={})", new_welcome_mode, message_count);
            self.in_welcome_mode = new_welcome_mode;

            // Reset the scroll position to bottom to avoid PortalList first_id > range_end errors
            // This is needed because switching from a chat with many messages to one with fewer
            // can leave the scroll position pointing to a non-existent message index
            self.view.chat(ids!(main_content.chat)).write().messages_ref().write().instant_scroll_to_bottom(cx);
        }

        self.view.redraw(cx);
    }

    /// Delete a chat session
    pub fn delete_chat(&mut self, cx: &mut Cx, scope: &mut Scope, chat_id: ChatId) {
        let Some(store) = scope.data.get_mut::<Store>() else { return };

        // Check if we're deleting the current chat
        let is_current = self.current_chat_id == Some(chat_id);

        // Delete from storage (this also updates current_chat_id if needed)
        store.chats.delete_chat(chat_id);

        ::log::info!("Deleted chat {}", chat_id);

        // If we deleted the current chat, we need to switch to another chat or create a new one
        if is_current {
            if let Some(next_chat) = store.chats.saved_chats.first() {
                // Switch to the next available chat
                let next_id = next_chat.id;
                self.current_chat_id = Some(next_id);
                store.chats.set_current_chat(Some(next_id));

                // Load the chat's messages into controller
                if let Some(chat) = store.chats.get_chat_by_id(next_id) {
                    let mut messages = chat.messages.clone();
                    for msg in &mut messages {
                        msg.metadata.is_writing = false;
                    }
                    let message_count = messages.len();
                    let last_content_len = messages.last().map(|m| m.content.text.len()).unwrap_or(0);

                    {
                        let mut ctrl = self.chat_controller.lock().unwrap();
                        ctrl.dispatch_mutation(VecMutation::Set(messages));

                        if let Some(ref bot_id) = chat.bot_id {
                            ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(bot_id.clone())));
                        }
                    }

                    self.last_synced_message_count = message_count;
                    self.had_writing_message = false;
                    self.last_synced_content_len = last_content_len;
                }
            } else {
                // No chats left, create a new one
                self.create_new_chat(cx, scope);
                return; // create_new_chat handles redraw
            }

            // Reset scroll position
            self.view.chat(ids!(main_content.chat)).write().messages_ref().write().instant_scroll_to_bottom(cx);
        }

        self.view.redraw(cx);
    }
}

impl Widget for ChatApp {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // Log state at start of handle_event for debugging
        if matches!(event, Event::KeyDown(_) | Event::TextInput(_)) {
            ::log::info!("handle_event START: in_welcome_mode={}, chat_initialized={}",
                self.in_welcome_mode, self.chat_initialized);
        }

        // Set controller on Chat widget early (required for Messages widget)
        self.maybe_set_controller_on_widget(cx);

        // Set controller on welcome prompt for centered input (only once)
        if !self.welcome_prompt_controller_set {
            self.view.prompt_input(ids!(main_content.welcome_overlay.welcome_prompt))
                .write()
                .set_chat_controller(Some(self.chat_controller.clone()));
            self.welcome_prompt_controller_set = true;
        }

        // Handle pending controller reset (e.g., after models load or view becomes visible)
        // This ensures the model list is properly populated after visibility changes
        // Also re-filters bots in case model settings changed in Settings
        if self.needs_controller_reset {
            if let Some(store) = scope.data.get::<Store>() {
                let all_bots = store.providers_manager.get_all_bots();
                if !all_bots.is_empty() {
                    let enabled_bots = Self::filter_enabled_bots(all_bots, store);

                    self.force_reset_controller_on_widget(cx);
                    self.skip_chat_draw_frames = 2;

                    // Preserve existing messages (including writing indicator) if busy
                    let mut ctrl = self.chat_controller.lock().unwrap();
                    let existing_msgs: Vec<_> = ctrl.state().messages.clone();
                    ctrl.dispatch_mutation(VecMutation::Set(enabled_bots));
                    if !existing_msgs.is_empty() {
                        for msg in existing_msgs {
                            ctrl.dispatch_mutation(VecMutation::Push(msg));
                        }
                    }
                    drop(ctrl);

                    self.view.redraw(cx);
                    self.view.chat(ids!(main_content.chat)).redraw(cx);
                }
            }
            self.needs_controller_reset = false;
        }

        // Handle pending new chat request (set by request_new_chat from parent App)
        if self.needs_new_chat {
            self.needs_new_chat = false;
            self.create_new_chat(cx, scope);
            // Consume pending_chat_model if set by StoreAction::OpenChatWithModel
            // (model injection happens via maybe_inject_local_model detecting active_local_model change)
            if let Some(store) = scope.data.get_mut::<Store>() {
                let _ = store.take_pending_chat_model();
            }
        }

        // Sync chat mode from Store's loaded model category
        self.sync_chat_mode(scope);

        // Poll mode-specific async results (ASR/TTS/Image)
        self.poll_mode_result(cx);
        self.poll_file_picker(cx, scope);

        // Strip stale error messages from ChatTask::Send in non-chat modes.
        // The Chat widget dispatches ChatTask::Send async; the error arrives after
        // our interception, so we clean it up on every frame.
        self.strip_mode_errors(cx);

        // Keep requesting frames while a mode operation is in progress
        // so we can poll the result and keep the typing animation running.
        if self.mode_busy {
            cx.new_next_frame();
            self.view.redraw(cx);
        }

        // Check and configure providers from Store
        self.maybe_configure_providers(cx, scope);

        // React to active_local_model changes (set by Model Hub "Open in Chat")
        self.maybe_inject_local_model(cx, scope);

        // Check for loaded bots from the ChatController
        self.check_for_loaded_bots(cx, scope);

        // Initialize chat from persistence (load or create)
        self.maybe_initialize_chat(cx, scope);

        // Track model selection changes and save to preferences
        self.track_model_selection(scope);

        // Sync messages to persistence when they change
        self.sync_messages_to_persistence(cx, scope);

        // Sync bot selection to current chat
        self.sync_bot_to_chat(scope);

        // Handle events for Chat widget + mode controls
        self.view.chat(ids!(main_content.chat)).handle_event(cx, event, scope);
        self.view.view(ids!(header)).handle_event(cx, event, scope);
        self.view.view(ids!(mode_controls)).handle_event(cx, event, scope);

        // ── VLM image drag-and-drop ──────────────────────────────────────────
        if self.chat_mode == ChatMode::Vlm {
            let drop_zone_area = self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone)).area();
            match event.drag_hits(cx, drop_zone_area) {
                DragHit::Drag(e) => {
                    match e.state {
                        DragState::In | DragState::Over => {
                            *e.response.lock().unwrap() = DragResponse::Copy;
                            self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone))
                                .apply_over(cx, live! { draw_bg: { hover: (1.0) } });
                            self.view.redraw(cx);
                        }
                        DragState::Out => {
                            self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone))
                                .apply_over(cx, live! { draw_bg: { hover: (0.0) } });
                            self.view.redraw(cx);
                        }
                    }
                }
                DragHit::Drop(e) => {
                    self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone))
                        .apply_over(cx, live! { draw_bg: { hover: (0.0) } });
                    for item in e.items.iter() {
                        if let DragItem::FilePath { path, .. } = item {
                            let lower = path.to_lowercase();
                            if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
                                || lower.ends_with(".bmp") || lower.ends_with(".gif") || lower.ends_with(".webp") {
                                if let Ok(bytes) = std::fs::read(path) {
                                    use base64::Engine;
                                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                    self.vlm_image_b64 = Some(b64);
                                    self.vlm_image_path = path.clone();
                                    let filename = std::path::Path::new(path)
                                        .file_name()
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path.clone());
                                    self.view.label(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_file_label))
                                        .set_text(cx, &filename);
                                    let preview = self.view.image(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_preview));
                                    preview.set_visible(cx, true);
                                    let _ = preview.load_image_file_by_path(cx, std::path::Path::new(path));
                                    self.view.redraw(cx);
                                }
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // ── ASR audio drag-and-drop ──────────────────────────────────────────
        if self.chat_mode == ChatMode::Asr {
            let drop_zone_area = self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone)).area();
            match event.drag_hits(cx, drop_zone_area) {
                DragHit::Drag(e) => {
                    match e.state {
                        DragState::In | DragState::Over => {
                            *e.response.lock().unwrap() = DragResponse::Copy;
                            self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone))
                                .apply_over(cx, live! { draw_bg: { hover: (1.0) } });
                            self.view.redraw(cx);
                        }
                        DragState::Out => {
                            self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone))
                                .apply_over(cx, live! { draw_bg: { hover: (0.0) } });
                            self.view.redraw(cx);
                        }
                    }
                }
                DragHit::Drop(e) => {
                    self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone))
                        .apply_over(cx, live! { draw_bg: { hover: (0.0) } });
                    for item in e.items.iter() {
                        if let DragItem::FilePath { path, .. } = item {
                            let lower = path.to_lowercase();
                            if lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.ends_with(".m4a")
                                || lower.ends_with(".flac") || lower.ends_with(".ogg") || lower.ends_with(".aac") {
                                self.asr_file_path = path.clone();
                                let filename = std::path::Path::new(path)
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.clone());
                                self.view.label(ids!(mode_controls.asr_controls.asr_file_row.asr_file_label))
                                    .set_text(cx, &filename);
                                self.start_asr_transcribe(cx, scope);
                                self.view.redraw(cx);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Detect new user messages from Chat widget for non-LLM modes
        self.maybe_handle_mode_message(cx, scope);

        // Use WidgetMatchEvent pattern for handling actions
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Update chat title from current chat (empty if no saved chats)
        if let Some(store) = scope.data.get::<Store>() {
            // Only show title if there are saved chats with messages
            if store.chats.saved_chats.is_empty() {
                // No saved chats - set empty title
                self.view.label(ids!(title_label)).set_text(cx, "");
            } else if let Some(chat) = store.chats.get_current_chat() {
                self.view.label(ids!(title_label)).set_text(cx, &chat.title);
            } else {
                self.view.label(ids!(title_label)).set_text(cx, "");
            }
        }

        // Update greeting text based on loaded model
        if let Some(store) = scope.data.get::<Store>() {
            if let Some(model_id) = store.get_active_local_model() {
                let greeting = format!("{} is ready", model_id);
                self.view.label(ids!(main_content.welcome_overlay.greeting_label))
                    .set_text(cx, &greeting);
            } else {
                self.view.label(ids!(main_content.welcome_overlay.greeting_label))
                    .set_text(cx, "What can I help you with?");
            }
        }

        // Always show the Chat widget, never the welcome overlay.
        // This keeps the UI consistent across all model types and avoids flashing.
        self.in_welcome_mode = false;
        if self.skip_chat_draw_frames > 0 {
            self.skip_chat_draw_frames -= 1;
            self.view.redraw(cx);
        }
        self.view.view(ids!(main_content.welcome_overlay)).set_visible(cx, false);
        self.view.chat(ids!(main_content.chat)).set_visible(cx, true);

        // Show mode_controls bar for non-LLM modes
        let show_controls = matches!(self.chat_mode, ChatMode::Vlm | ChatMode::Tts | ChatMode::ImageGen | ChatMode::Asr | ChatMode::VideoGen);
        self.view.view(ids!(mode_controls)).set_visible(cx, show_controls);
        self.view.view(ids!(mode_controls.vlm_controls)).set_visible(cx, self.chat_mode == ChatMode::Vlm);
        self.view.view(ids!(mode_controls.tts_controls)).set_visible(cx, self.chat_mode == ChatMode::Tts);
        self.view.view(ids!(mode_controls.image_controls)).set_visible(cx, self.chat_mode == ChatMode::ImageGen);
        self.view.view(ids!(mode_controls.asr_controls)).set_visible(cx, self.chat_mode == ChatMode::Asr);

        // Show VLM image preview/clear when an image is selected
        if self.chat_mode == ChatMode::Vlm {
            let has_image = self.vlm_image_b64.is_some();
            self.view.view(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_preview))
                .set_visible(cx, has_image);
            self.view.view(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_clear_btn))
                .set_visible(cx, has_image);
            if has_image {
                self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone))
                    .set_visible(cx, false);
            } else {
                self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone))
                    .set_visible(cx, true);
            }
        }

        // Show ASR clear button when audio file is selected
        if self.chat_mode == ChatMode::Asr {
            let has_audio = !self.asr_file_path.is_empty();
            self.view.view(ids!(mode_controls.asr_controls.asr_file_row.asr_clear_btn))
                .set_visible(cx, has_audio);
            if has_audio {
                self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone))
                    .set_visible(cx, false);
            } else {
                self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone))
                    .set_visible(cx, true);
            }
        }

        // Show reference image section only for image editing models
        if self.chat_mode == ChatMode::ImageGen {
            let supports_images = if let Some(store) = scope.data.get::<Store>() {
                store.active_local_model_supports_images
            } else { false };
            self.view.view(ids!(mode_controls.image_controls.image_ref_section))
                .set_visible(cx, supports_images);
        }

        // Show TTS audio controls when there's a generated file
        if self.chat_mode == ChatMode::Tts {
            let has_audio = self.tts_audio_path.is_some();
            self.view.view(ids!(mode_controls.tts_controls.tts_audio_controls))
                .set_visible(cx, has_audio);
            if has_audio {
                let label = if self.tts_playing { "⏸ Stop" } else { "▶ Play" };
                self.view.label(ids!(mode_controls.tts_controls.tts_audio_controls.tts_play_btn.tts_play_label))
                    .set_text(cx, label);
            }
        }

        // Check if afplay process finished
        if self.tts_playing {
            if let Some(ref mut proc) = self.tts_play_process {
                if let Ok(Some(_)) = proc.try_wait() {
                    self.tts_playing = false;
                    self.tts_play_process = None;
                    self.tts_play_start = None;
                }
            }
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for ChatApp {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, scope: &mut Scope) {
        // Hide model_selector inside Chat's PromptInput when a local model is loaded
        let has_local_model = if let Some(store) = scope.data.get::<Store>() {
            store.active_local_model_category.is_some()
        } else {
            false
        };
        if has_local_model {
            self.view.widget(ids!(main_content.chat.prompt.model_selector))
                .set_visible(cx, false);
        }

        let was_welcome = self.in_welcome_mode;

        // Handle ChatHistoryPanel actions
        for action in actions.iter() {
            if let ChatHistoryAction::NewChat = action.cast() {
                ::log::info!("ACTION: NewChat triggered");
                self.create_new_chat(cx, scope);
            }
            if let ChatHistoryAction::SelectChat(chat_id) = action.cast() {
                ::log::info!("ACTION: SelectChat({}) triggered, in_welcome_mode={}", chat_id, self.in_welcome_mode);
                self.switch_to_chat(cx, scope, chat_id);
            }
            if let ChatHistoryAction::DeleteChat(chat_id) = action.cast() {
                ::log::info!("ACTION: DeleteChat({}) triggered", chat_id);
                self.delete_chat(cx, scope, chat_id);
            }

            // Handle A2UI toggle from PromptInput (direct action for welcome prompt)
            if let PromptInputAction::A2uiToggled(enabled) = action.cast() {
                eprintln!("[ChatApp] A2UI toggled (from PromptInput): {}", enabled);
                if let Some(ref a2ui_client) = self.a2ui_client {
                    a2ui_client.set_a2ui_enabled(enabled);
                    eprintln!("[ChatApp] A2uiClient enabled state set to: {}", enabled);
                } else {
                    eprintln!("[ChatApp] Warning: No A2uiClient available");
                }
            }

            // Handle A2UI toggle from Chat widget (forwarded from its internal PromptInput)
            if let ChatAction::A2uiToggled(enabled) = action.cast() {
                eprintln!("[ChatApp] A2UI toggled (from Chat): {}", enabled);
                if let Some(ref a2ui_client) = self.a2ui_client {
                    a2ui_client.set_a2ui_enabled(enabled);
                    eprintln!("[ChatApp] A2uiClient enabled state set to: {}", enabled);
                } else {
                    eprintln!("[ChatApp] Warning: No A2uiClient available");
                }
            }
        }

        // Also directly check the Chat widget's PromptInput for A2UI toggle
        let chat = self.view.chat(ids!(main_content.chat));
        if let Some(a2ui_enabled) = chat.read().prompt_input_ref().a2ui_toggled(actions) {
            eprintln!("[ChatApp] A2UI toggled (direct check): {}", a2ui_enabled);
            if let Some(ref a2ui_client) = self.a2ui_client {
                a2ui_client.set_a2ui_enabled(a2ui_enabled);
                eprintln!("[ChatApp] A2uiClient enabled state set to: {}", a2ui_enabled);
            } else {
                eprintln!("[ChatApp] Warning: No A2uiClient available");
            }
        }

        // Handle welcome prompt send action - use submitted() to detect actual user send action
        // (has_send_task() just checks if the button is in "send" state, not if user clicked)
        let mut welcome_prompt = self.view.prompt_input(ids!(main_content.welcome_overlay.welcome_prompt));
        if welcome_prompt.read().submitted(actions) {
            ::log::info!("WELCOME PROMPT SUBMITTED: user pressed Enter or clicked Send");
            let text = welcome_prompt.text();

            if !text.is_empty() {
                use moly_kit::aitk::protocol::{EntityId, Message, MessageContent};

                {
                    let mut ctrl = self.chat_controller.lock().unwrap();
                    ctrl.dispatch_mutation(VecMutation::Push(Message {
                        from: EntityId::User,
                        content: MessageContent {
                            text,
                            ..Default::default()
                        },
                        ..Default::default()
                    }));
                    ctrl.dispatch_task(ChatTask::Send);
                }

                // Transfer A2UI toggle state to Chat's PromptInput before reset
                let a2ui_state = welcome_prompt.read().is_a2ui_enabled();
                if a2ui_state {
                    let mut chat = self.view.chat(ids!(main_content.chat));
                    chat.write().prompt_input_ref().write()
                        .set_a2ui_enabled(cx, true);
                }

                // Reset the welcome prompt
                welcome_prompt.write().reset(cx);

                // Exit welcome mode - switch to normal Chat view
                ::log::info!("handle_actions: user sent from welcome prompt, setting in_welcome_mode=false");
                self.in_welcome_mode = false;

                // Redraw to switch to Chat widget view
                self.view.redraw(cx);
            }
        }

        // ── Mode controls bar handlers ────────────────────────────────────

        // VLM: Browse image button
        if self.view.view(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_browse_btn))
            .finger_down(&actions).is_some()
        {
            self.handle_vlm_browse(cx);
        }

        // VLM: Clear image button
        if self.view.view(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_clear_btn))
            .finger_down(&actions).is_some()
        {
            self.vlm_image_path.clear();
            self.vlm_image_b64 = None;
            self.view.label(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_file_label))
                .set_text(cx, "");
            self.view.image(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_preview))
                .set_visible(cx, false);
            self.view.view(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_clear_btn))
                .set_visible(cx, false);
            self.view.view(ids!(mode_controls.vlm_controls.vlm_drop_zone))
                .set_visible(cx, true);
            self.view.redraw(cx);
        }

        // ASR: Browse/upload audio button
        if self.view.view(ids!(mode_controls.asr_controls.asr_file_row.asr_browse_btn))
            .finger_down(&actions).is_some()
        {
            self.handle_asr_browse(cx);
        }

        // ASR: Clear audio button
        if self.view.view(ids!(mode_controls.asr_controls.asr_file_row.asr_clear_btn))
            .finger_down(&actions).is_some()
        {
            self.asr_file_path.clear();
            self.view.label(ids!(mode_controls.asr_controls.asr_file_row.asr_file_label))
                .set_text(cx, "");
            self.view.view(ids!(mode_controls.asr_controls.asr_file_row.asr_clear_btn))
                .set_visible(cx, false);
            self.view.view(ids!(mode_controls.asr_controls.asr_drop_zone))
                .set_visible(cx, true);
            self.view.redraw(cx);
        }

        // TTS: Voice selection buttons (in mode_controls bar)
        for i in 0..9usize {
            let voice_id = match i {
                0 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_0),
                1 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_1),
                2 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_2),
                3 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_3),
                4 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_4),
                5 => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_5),
                6 => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_6),
                7 => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_7),
                _ => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_8),
            };
            if self.view.view(voice_id).finger_down(&actions).is_some() {
                self.tts_voice_idx = i;
                for j in 0..9usize {
                    let vid = match j {
                        0 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_0),
                        1 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_1),
                        2 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_2),
                        3 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_3),
                        4 => ids!(mode_controls.tts_controls.tts_voice_row.tts_voice_4),
                        5 => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_5),
                        6 => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_6),
                        7 => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_7),
                        _ => ids!(mode_controls.tts_controls.tts_voice_row_zh.tts_voice_8),
                    };
                    let sel = if j == i { 1.0_f64 } else { 0.0_f64 };
                    self.view.view(vid).apply_over(cx, live! { draw_bg: { selected: (sel) } });
                }
                self.view.redraw(cx);
                break;
            }
        }

        // TTS: Play button
        if self.view.view(ids!(mode_controls.tts_controls.tts_audio_controls.tts_play_btn))
            .finger_down(&actions).is_some()
        {
            self.handle_audio_play_toggle(cx);
        }

        // TTS: Save button
        if self.view.view(ids!(mode_controls.tts_controls.tts_audio_controls.tts_save_btn))
            .finger_down(&actions).is_some()
        {
            self.handle_audio_download(cx);
        }

        // Image: Browse reference image button
        if self.view.view(ids!(mode_controls.image_controls.image_ref_section.image_ref_browse_btn))
            .finger_down(&actions).is_some()
        {
            self.handle_image_ref_browse(cx);
        }


        // Log if welcome mode changed during handle_actions
        if was_welcome != self.in_welcome_mode {
            ::log::info!("handle_actions: in_welcome_mode changed from {} to {}", was_welcome, self.in_welcome_mode);
        }
    }
}

impl ChatApp {
    /// Sync chat_mode from the Store's active model category,
    /// falling back to the current session's saved category for older sessions.
    fn sync_chat_mode(&mut self, scope: &mut Scope) {
        let new_mode = if let Some(store) = scope.data.get::<Store>() {
            // First try: use the active local model's category
            let from_active = match store.get_active_local_model_category() {
                Some(RegistryCategory::Vlm)      => Some(ChatMode::Vlm),
                Some(RegistryCategory::Asr)      => Some(ChatMode::Asr),
                Some(RegistryCategory::Tts)      => Some(ChatMode::Tts),
                Some(RegistryCategory::ImageGen) => Some(ChatMode::ImageGen),
                Some(RegistryCategory::VideoGen) => Some(ChatMode::VideoGen),
                Some(RegistryCategory::Llm)      => Some(ChatMode::Llm),
                _                                => None,
            };
            // If no active model, use the current session's saved category
            from_active.unwrap_or_else(|| {
                if let Some(chat_id) = self.current_chat_id {
                    if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
                        match chat.model_category {
                            Some(RegistryCategory::Vlm)      => return ChatMode::Vlm,
                            Some(RegistryCategory::Asr)      => return ChatMode::Asr,
                            Some(RegistryCategory::Tts)      => return ChatMode::Tts,
                            Some(RegistryCategory::ImageGen) => return ChatMode::ImageGen,
                            Some(RegistryCategory::VideoGen) => return ChatMode::VideoGen,
                            _ => {}
                        }
                    }
                }
                ChatMode::Llm
            })
        } else {
            ChatMode::Llm
        };
        if new_mode != self.chat_mode {
            ::log::info!("Chat mode changed: {:?} -> {:?}", self.chat_mode, new_mode);
            self.chat_mode = new_mode;
            // Reset mode-specific state
            self.asr_file_path.clear();
            self.vlm_image_path.clear();
            self.vlm_image_b64 = None;
            self.image_ref_b64 = None;
            self.image_ref_path.clear();
            self.mode_rx = None;
            self.mode_busy = false;
            // Sync mode msg tracking to current message count
            let ctrl = self.chat_controller.lock().unwrap();
            self.last_mode_msg_count = ctrl.state().messages.len();
        }
    }

    /// Remove error/system messages injected by ChatTask::Send in non-chat modes.
    /// The Chat widget fires ChatTask::Send automatically, which hits the wrong
    /// endpoint for TTS/ImageGen/VideoGen/ASR and returns an error. This method
    /// strips those stale errors every frame so they don't clutter the chat.
    fn strip_mode_errors(&mut self, cx: &mut Cx) {
        if !matches!(self.chat_mode, ChatMode::Vlm | ChatMode::Tts | ChatMode::ImageGen | ChatMode::VideoGen | ChatMode::Asr) {
            return;
        }
        use moly_kit::aitk::protocol::EntityId;
        let mut ctrl = self.chat_controller.lock().unwrap();
        let msgs = &ctrl.state().messages;
        let has_error = msgs.iter().any(|m| {
            matches!(m.from, EntityId::App | EntityId::System) && !m.metadata.is_writing
        });
        if has_error {
            let kept: Vec<_> = msgs.iter()
                .filter(|m| m.metadata.is_writing || !matches!(m.from, EntityId::App | EntityId::System))
                .cloned()
                .collect();
            ctrl.dispatch_mutation(VecMutation::Set(kept.clone()));
            self.last_mode_msg_count = kept.len();
            drop(ctrl);
            self.skip_chat_draw_frames = 2;
            cx.new_next_frame();
        }
    }

    /// Poll the mode-specific async result channel and inject as assistant message
    fn poll_mode_result(&mut self, cx: &mut Cx) {
        let result = self.mode_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(result) = result else { return };
        self.mode_rx = None;
        self.mode_busy = false;

        use moly_kit::aitk::protocol::{Attachment, EntityId, Message, MessageContent};

        let (response_text, response_attachments) = match self.chat_mode {
            ChatMode::Vlm => {
                match result {
                    Ok(text) => (text, vec![]),
                    Err(e) => (format!("VLM error: {}", e), vec![]),
                }
            }
            ChatMode::Asr => {
                match result {
                    Ok(text) => (text, vec![]),
                    Err(e) => (format!("Transcription error: {}", e), vec![]),
                }
            }
            ChatMode::Tts => {
                match result {
                    Ok(path) => {
                        self.tts_audio_path = Some(path.clone());
                        self.tts_playing = false;
                        self.tts_duration_secs = Self::get_wav_duration(&path);
                        let voice = TTS_VOICE_IDS.get(self.tts_voice_idx)
                            .copied().unwrap_or("vivian");
                        let dur = self.tts_duration_secs;
                        let mins = dur as u32 / 60;
                        let secs = dur as u32 % 60;
                        if let Ok(child) = std::process::Command::new("afplay").arg(&path).spawn() {
                            self.tts_play_process = Some(child);
                            self.tts_playing = true;
                            self.tts_play_start = Some(std::time::Instant::now());
                        }
                        (format!("Voice: {} | Duration: {}:{:02}", voice, mins, secs), vec![])
                    }
                    Err(e) => (format!("TTS error: {}", e), vec![]),
                }
            }
            ChatMode::ImageGen => {
                match result {
                    Ok(path) => {
                        if let Ok(bytes) = std::fs::read(&path) {
                            let attachment = Attachment::from_bytes(
                                "generated_image.png".to_string(),
                                Some("image/png".to_string()),
                                &bytes,
                            );
                            ("".to_string(), vec![attachment])
                        } else {
                            (format!("Image saved to: {}", path), vec![])
                        }
                    }
                    Err(e) => (format!("Image generation error: {}", e), vec![]),
                }
            }
            ChatMode::VideoGen => {
                match result {
                    Ok(path) => {
                        (format!("Video generated: {}", path), vec![])
                    }
                    Err(e) => (format!("Video generation error: {}", e), vec![]),
                }
            }
            _ => return,
        };

        // Replace the "writing" indicator with the real response
        {
            let mut ctrl = self.chat_controller.lock().unwrap();
            let bot_id = ctrl.state().bot_id.clone();
            let from = bot_id.map(EntityId::Bot).unwrap_or(EntityId::System);
            let response_msg = Message {
                from,
                content: MessageContent {
                    text: response_text,
                    attachments: response_attachments,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut msgs = ctrl.state().messages.clone();
            if let Some(pos) = msgs.iter().rposition(|m| m.metadata.is_writing) {
                msgs[pos] = response_msg;
                ctrl.dispatch_mutation(VecMutation::Set(msgs));
            } else {
                ctrl.dispatch_mutation(VecMutation::Push(response_msg));
            }
            self.last_mode_msg_count = ctrl.state().messages.len();
        }

        self.view.redraw(cx);
    }

    /// Detect new user messages from the Chat widget and trigger mode-specific operations.
    /// For all non-LLM modes, we intercept user messages and make our own API calls
    /// instead of relying on ChatTask::Send (which would call the wrong endpoint or
    /// format content parts in the wrong order for VLM).
    fn maybe_handle_mode_message(&mut self, cx: &mut Cx, scope: &mut Scope) {
        if self.mode_busy { return; }
        if !matches!(self.chat_mode, ChatMode::Vlm | ChatMode::Tts | ChatMode::ImageGen | ChatMode::VideoGen) { return; }

        use moly_kit::aitk::protocol::EntityId;

        let (msg_count, user_text) = {
            let ctrl = self.chat_controller.lock().unwrap();
            let msgs = &ctrl.state().messages;
            let count = msgs.len();
            if count <= self.last_mode_msg_count {
                return;
            }
            let mut user_text = None;
            for msg in msgs[self.last_mode_msg_count..].iter() {
                if matches!(msg.from, EntityId::User) {
                    user_text = Some(msg.content.text.clone());
                }
            }
            (count, user_text)
        };

        let Some(user_text) = user_text else {
            self.last_mode_msg_count = msg_count;
            return;
        };

        // Strip error/system messages from the failed ChatTask::Send
        {
            use moly_kit::aitk::protocol::{Message, MessageContent, MessageMetadata};
            let mut ctrl = self.chat_controller.lock().unwrap();
            let msgs = ctrl.state().messages.clone();
            let kept: Vec<_> = msgs.iter().enumerate().filter(|(i, m)| {
                if *i < self.last_mode_msg_count { return true; }
                matches!(m.from, EntityId::User)
            }).map(|(_, m)| m.clone()).collect();
            if kept.len() < msgs.len() {
                ctrl.dispatch_mutation(VecMutation::Set(kept.clone()));
            }
            // Inject a "writing" indicator message so users see the model is working
            let bot_id = ctrl.state().bot_id.clone();
            ctrl.dispatch_mutation(VecMutation::Push(Message {
                from: bot_id.map(EntityId::Bot).unwrap_or(EntityId::System),
                content: MessageContent {
                    text: String::new(),
                    ..Default::default()
                },
                metadata: MessageMetadata {
                    is_writing: true,
                    ..MessageMetadata::default()
                },
                ..Default::default()
            }));
            self.last_mode_msg_count = ctrl.state().messages.len();
        }

        self.view.redraw(cx);

        match self.chat_mode {
            ChatMode::Vlm => {
                self.start_vlm_generate(cx, scope, user_text);
            }
            ChatMode::Tts => {
                self.start_tts_generate(cx, scope, user_text);
            }
            ChatMode::ImageGen => {
                self.start_image_generate(cx, scope, user_text);
            }
            ChatMode::VideoGen => {
                self.start_video_generate(cx, scope, user_text);
            }
            _ => {}
        }
    }

    /// Audio player: toggle play/stop
    fn handle_audio_play_toggle(&mut self, cx: &mut Cx) {
        let Some(ref path) = self.tts_audio_path else { return };

        if self.tts_playing {
            if let Some(mut proc) = self.tts_play_process.take() {
                proc.kill().ok();
                proc.wait().ok();
            }
            self.tts_playing = false;
            self.tts_play_start = None;
        } else {
            if let Ok(child) = std::process::Command::new("afplay").arg(path).spawn() {
                self.tts_play_process = Some(child);
                self.tts_playing = true;
                self.tts_play_start = Some(std::time::Instant::now());
            }
        }
        self.view.redraw(cx);
    }

    /// Get WAV file duration in seconds from file header
    fn get_wav_duration(path: &str) -> f64 {
        let Ok(data) = std::fs::read(path) else { return 0.0 };
        if data.len() < 44 { return 0.0; }
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as f64;
        let byte_rate = u32::from_le_bytes([data[28], data[29], data[30], data[31]]) as f64;
        if byte_rate == 0.0 { return 0.0; }
        let data_size = (data.len() - 44) as f64;
        data_size / byte_rate
    }

    /// Audio player: save as MP3 via save dialog
    fn handle_audio_download(&mut self, _cx: &mut Cx) {
        let Some(ref wav_path) = self.tts_audio_path else { return };
        let wav_path = wav_path.clone();

        std::thread::spawn(move || {
            let output = std::process::Command::new("osascript")
                .args(["-e", "POSIX path of (choose file name with prompt \"Save audio as MP3\" default name \"speech.mp3\")"])
                .output();
            if let Ok(out) = output {
                let save_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if save_path.is_empty() { return; }

                if save_path.ends_with(".mp3") {
                    let _ = std::process::Command::new("afconvert")
                        .args(["-f", "mp4f", "-d", "aac", &wav_path, &save_path])
                        .status();
                } else {
                    let _ = std::fs::copy(&wav_path, &save_path);
                }
            }
        });
    }

    /// VLM: Open image file browser dialog
    fn handle_vlm_browse(&mut self, _cx: &mut Cx) {
        if self.file_picker_rx.is_some() { return; }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let output = std::process::Command::new("osascript")
                .args(["-e", "POSIX path of (choose file of type {\"public.image\"} with prompt \"Select image\")"])
                .output();
            match output {
                Ok(out) => {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        tx.send(Ok(path)).ok();
                    } else {
                        tx.send(Err("Cancelled".to_string())).ok();
                    }
                }
                Err(e) => { tx.send(Err(e.to_string())).ok(); }
            }
        });
        self.file_picker_rx = Some(rx);
    }

    /// ASR: Open file browser dialog
    fn handle_asr_browse(&mut self, _cx: &mut Cx) {
        if self.file_picker_rx.is_some() { return; } // Already picking
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let output = std::process::Command::new("osascript")
                .args(["-e", "POSIX path of (choose file of type {\"public.audio\"} with prompt \"Select audio file\")"])
                .output();
            match output {
                Ok(out) => {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        tx.send(Ok(path)).ok();
                    } else {
                        tx.send(Err("Cancelled".to_string())).ok();
                    }
                }
                Err(e) => { tx.send(Err(e.to_string())).ok(); }
            }
        });
        self.file_picker_rx = Some(rx);
    }

    /// Image Edit: Open file browser for reference image
    fn handle_image_ref_browse(&mut self, _cx: &mut Cx) {
        if self.file_picker_rx.is_some() { return; }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let output = std::process::Command::new("osascript")
                .args(["-e", "set f to choose file of type {\"public.image\"} with prompt \"Select reference image\""])
                .args(["-e", "POSIX path of f"])
                .output();
            match output {
                Ok(out) => {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        tx.send(Ok(path)).ok();
                    } else {
                        tx.send(Err("Cancelled".to_string())).ok();
                    }
                }
                Err(e) => { tx.send(Err(e.to_string())).ok(); }
            }
        });
        self.file_picker_rx = Some(rx);
    }

    /// Poll for file picker result
    fn poll_file_picker(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let result = self.file_picker_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(result) = result else { return };
        self.file_picker_rx = None;

        if let Ok(path) = result {
            let filename = std::path::Path::new(&path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());

            match self.chat_mode {
                ChatMode::Vlm => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        self.vlm_image_b64 = Some(b64);
                        self.vlm_image_path = path.clone();
                        self.view.label(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_file_label))
                            .set_text(cx, &filename);
                        let preview = self.view.image(ids!(mode_controls.vlm_controls.vlm_file_row.vlm_preview));
                        preview.set_visible(cx, true);
                        let _ = preview.load_image_file_by_path(cx, std::path::Path::new(&path));
                    }
                }
                ChatMode::ImageGen => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        self.image_ref_b64 = Some(b64);
                        self.image_ref_path = path.clone();
                        self.view.label(ids!(mode_controls.image_controls.image_ref_section.image_ref_file_label))
                            .set_text(cx, &filename);
                        let preview = self.view.image(ids!(mode_controls.image_controls.image_ref_section.image_ref_preview));
                        preview.set_visible(cx, true);
                        let _ = preview.load_image_file_by_path(cx, std::path::Path::new(&path));
                    }
                }
                ChatMode::Asr => {
                    self.asr_file_path = path;
                    self.view.label(ids!(mode_controls.asr_controls.asr_file_row.asr_file_label))
                        .set_text(cx, &filename);
                    self.start_asr_transcribe(cx, scope);
                }
                _ => {}
            }
            self.view.redraw(cx);
        }
    }

    /// ASR: Start transcription of the selected audio file, pushing a user message first
    fn start_asr_transcribe(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let model_id = if let Some(store) = scope.data.get::<Store>() {
            store.get_active_local_model().unwrap_or("").to_string()
        } else { return };

        let file_path = self.asr_file_path.clone();
        if file_path.is_empty() { return; }

        let filename = std::path::Path::new(&file_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());

        // Push a user message showing what we're transcribing, then a writing indicator
        {
            use moly_kit::aitk::protocol::{EntityId, Message, MessageContent, MessageMetadata};
            let mut ctrl = self.chat_controller.lock().unwrap();
            ctrl.dispatch_mutation(VecMutation::Push(Message {
                from: EntityId::User,
                content: MessageContent {
                    text: format!("Transcribe: {}", filename),
                    ..Default::default()
                },
                ..Default::default()
            }));
            let bot_id = ctrl.state().bot_id.clone();
            ctrl.dispatch_mutation(VecMutation::Push(Message {
                from: bot_id.map(EntityId::Bot).unwrap_or(EntityId::System),
                content: MessageContent {
                    text: String::new(),
                    ..Default::default()
                },
                metadata: MessageMetadata {
                    is_writing: true,
                    ..MessageMetadata::default()
                },
                ..Default::default()
            }));
            self.last_mode_msg_count = ctrl.state().messages.len();
        }

        // Exit welcome mode since we're now showing messages
        self.in_welcome_mode = false;

        self.mode_busy = true;
        self.view.label(ids!(mode_controls.asr_controls.asr_file_row.asr_file_label))
            .set_text(cx, "Transcribing...");
        self.view.redraw(cx);

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                // Convert non-WAV to WAV if needed
                let wav_path = if !file_path.to_lowercase().ends_with(".wav") {
                    let tmp = format!("/tmp/ominix_asr_{}.wav", std::process::id());
                    let status = std::process::Command::new("afconvert")
                        .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1", &file_path, &tmp])
                        .status()
                        .map_err(|e| format!("afconvert failed: {}", e))?;
                    if !status.success() {
                        return Err("Audio conversion failed".to_string());
                    }
                    tmp
                } else {
                    file_path.clone()
                };

                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(1800))
                    .build()
                    .map_err(|e| e.to_string())?;
                let body = serde_json::json!({ "file": wav_path, "model": model_id });
                let resp = client.post("http://localhost:8080/v1/audio/transcriptions")
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?;
                let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
                json["text"].as_str().map(|s| s.to_string())
                    .ok_or_else(|| format!("Unexpected response: {}", json))
            })();
            tx.send(result).ok();
        });
        self.mode_rx = Some(rx);
    }

    /// TTS: Start speech generation from the given text
    /// VLM: Make a direct API call with text + image (matching hub format)
    fn start_vlm_generate(&mut self, _cx: &mut Cx, scope: &mut Scope, user_text: String) {
        let model_id = if let Some(store) = scope.data.get::<Store>() {
            store.get_active_local_model().unwrap_or("").to_string()
        } else { return };

        if user_text.is_empty() { return; }

        let image_b64 = self.vlm_image_b64.clone();
        self.mode_busy = true;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .map_err(|e| e.to_string())?;
                let mut content = vec![serde_json::json!({"type": "text", "text": user_text})];
                if let Some(b64) = image_b64 {
                    content.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": {"url": format!("data:image/jpeg;base64,{}", b64)}
                    }));
                }
                let body = serde_json::json!({
                    "model": model_id,
                    "messages": [{"role": "user", "content": content}]
                });
                let resp = client.post("http://localhost:8080/v1/chat/completions")
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?;
                let status = resp.status();
                let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
                if !status.is_success() {
                    let err_msg = json["error"]["message"].as_str().unwrap_or("Unknown error");
                    return Err(format!("API error {}: {}", status, err_msg));
                }
                json["choices"][0]["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| "No content in response".to_string())
            })();
            tx.send(result).ok();
        });
        self.mode_rx = Some(rx);
    }

    fn start_tts_generate(&mut self, _cx: &mut Cx, scope: &mut Scope, text: String) {
        let model_id = if let Some(store) = scope.data.get::<Store>() {
            store.get_active_local_model().unwrap_or("").to_string()
        } else { return };

        if text.is_empty() { return; }

        let voice = TTS_VOICE_IDS.get(self.tts_voice_idx)
            .copied().unwrap_or("vivian").to_string();

        self.mode_busy = true;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(300))
                    .build()
                    .map_err(|e| e.to_string())?;
                let body = serde_json::json!({"model": model_id, "input": text, "voice": voice});
                let resp = client.post("http://localhost:8080/v1/audio/speech")
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?;
                let bytes = resp.bytes().map_err(|e| e.to_string())?;
                let out_path = "/tmp/ominix-chat-tts.wav";
                std::fs::write(out_path, &bytes).map_err(|e| e.to_string())?;
                Ok(out_path.to_string())
            })();
            tx.send(result).ok();
        });
        self.mode_rx = Some(rx);
    }

    /// Image: Start generation from the given prompt
    fn start_image_generate(&mut self, _cx: &mut Cx, scope: &mut Scope, prompt: String) {
        let model_id = if let Some(store) = scope.data.get::<Store>() {
            store.get_active_local_model().unwrap_or("").to_string()
        } else { return };

        if prompt.is_empty() { return; }
        let neg_prompt = self.view.text_input(ids!(mode_controls.image_controls.image_neg_row.image_neg_prompt_input)).text();
        let ref_image_b64 = self.image_ref_b64.clone();

        self.mode_busy = true;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(600))
                    .build()
                    .map_err(|e| e.to_string())?;
                let mut body = serde_json::json!({
                    "model": model_id,
                    "prompt": prompt,
                    "n": 1,
                    "size": "512x512",
                    "response_format": "b64_json"
                });
                if !neg_prompt.is_empty() {
                    body["negative_prompt"] = serde_json::Value::String(neg_prompt);
                }
                if let Some(ref_b64) = ref_image_b64 {
                    body["image"] = serde_json::Value::String(ref_b64);
                }
                let resp = client.post("http://localhost:8080/v1/images/generations")
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?;
                let status = resp.status();
                let resp_text = resp.text().map_err(|e| format!("reading response: {}", e))?;
                if !status.is_success() {
                    return Err(format!("API error {}: {}", status, &resp_text[..resp_text.len().min(500)]));
                }
                let json: serde_json::Value = serde_json::from_str(&resp_text)
                    .map_err(|e| format!("parsing JSON: {} (first 200 chars: {})", e, &resp_text[..resp_text.len().min(200)]))?;
                let b64 = json["data"][0]["b64_json"].as_str()
                    .ok_or_else(|| format!("Unexpected response: {}", json))?;
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
                    .map_err(|e| e.to_string())?;
                let slug = model_id.replace('/', "-").replace(' ', "_");
                let path = format!("/tmp/ominix-chat-{}.png", slug);
                std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
                Ok(path)
            })();
            tx.send(result).ok();
        });
        self.mode_rx = Some(rx);
    }

    /// Video: Start generation from the given prompt
    fn start_video_generate(&mut self, _cx: &mut Cx, scope: &mut Scope, prompt: String) {
        let model_id = if let Some(store) = scope.data.get::<Store>() {
            let registry_id = store.get_active_local_model().unwrap_or("").to_string();
            let registry = moly_data::ModelRegistry::load();
            registry.get(&registry_id)
                .map(|m| m.runtime.api_model_id.clone())
                .unwrap_or(registry_id)
        } else { return };

        if prompt.is_empty() { return; }

        self.mode_busy = true;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(1800))
                    .build()
                    .map_err(|e| e.to_string())?;
                let body = serde_json::json!({
                    "model": model_id,
                    "prompt": prompt,
                    "n": 1,
                    "response_format": "b64_json"
                });
                let resp = client.post("http://localhost:8080/v1/videos/generations")
                    .json(&body)
                    .send()
                    .map_err(|e| e.to_string())?;
                let status = resp.status();
                let resp_text = resp.text().map_err(|e| format!("reading response: {}", e))?;
                if !status.is_success() {
                    return Err(format!("API error {}: {}", status, &resp_text[..resp_text.len().min(500)]));
                }
                let json: serde_json::Value = serde_json::from_str(&resp_text)
                    .map_err(|e| format!("parsing JSON: {} (first 200 chars: {})", e, &resp_text[..resp_text.len().min(200)]))?;
                let b64 = json["data"][0]["b64_json"].as_str()
                    .ok_or_else(|| format!("Unexpected response: {}", json))?;
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
                    .map_err(|e| e.to_string())?;
                let slug = model_id.replace('/', "-").replace(' ', "_");
                let path = format!("/tmp/ominix-chat-video-{}.mp4", slug);
                std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
                Ok(path)
            })();
            tx.send(result).ok();
        });
        self.mode_rx = Some(rx);
    }

    /// Configure all enabled providers and start fetching models sequentially
    fn maybe_configure_providers(&mut self, cx: &mut Cx, scope: &mut Scope) {
        // If we're already fetching, don't restart
        if self.fetch_in_progress {
            return;
        }

        let Some(store) = scope.data.get_mut::<Store>() else { return };

        // Get all enabled providers with API keys - clone to avoid borrow issues
        let enabled_providers: Vec<_> = store.preferences.get_enabled_providers()
            .iter()
            .map(|p| (*p).clone())
            .collect();

        // Check if we need to reconfigure (new providers added or removed)
        let current_provider_ids: Vec<_> = enabled_providers.iter().map(|p| p.id.clone()).collect();
        let mut needs_reconfigure = false;

        // Check if the set of providers has changed
        if self.providers_configured {
            let old_set: std::collections::HashSet<_> = self.fetched_provider_ids.iter().collect();
            let new_set: std::collections::HashSet<_> = current_provider_ids.iter().collect();
            needs_reconfigure = old_set != new_set;
        }

        // Skip if already configured and no changes
        if self.providers_configured && !needs_reconfigure {
            return;
        }

        // Handle case when all providers are disabled
        if enabled_providers.is_empty() {
            if self.providers_configured {
                ::log::info!("All providers disabled, clearing models");
                // Clear all bots
                store.providers_manager.clear_all_bots();
                {
                    let mut ctrl = self.chat_controller.lock().unwrap();
                    ctrl.dispatch_mutation(VecMutation::<Bot>::Set(vec![]));
                    ctrl.dispatch_mutation(ChatStateMutation::SetBotId(None));
                }
                self.fetched_provider_ids.clear();
                self.providers_configured = false;
                self.restored_saved_model = false;
                self.last_saved_bot_id = None;
                self.view.redraw(cx);
            }
            return;
        }

        ::log::info!("Configuring {} providers for multi-provider support", enabled_providers.len());

        // Clear previous state if reconfiguring
        if needs_reconfigure {
            ::log::info!("Provider configuration changed, clearing existing models");
            store.providers_manager.clear_all_bots();
            self.restored_saved_model = false;  // Allow model selection after reload
        }
        self.fetched_provider_ids.clear();
        self.providers_to_fetch.clear();
        self.fetch_index = 0;

        // Configure all provider clients in ProvidersManager
        store.reconfigure_providers();

        // Build list of providers to fetch
        for provider in &enabled_providers {
            let api_key = provider.api_key.clone().unwrap_or_default();
            let api_key = api_key.trim().to_string();
            if api_key.is_empty() {
                ::log::warn!("API key is empty for provider {}", provider.id);
                continue;
            }

            // Debug: show key length and first/last chars
            let key_preview = if api_key.len() > 8 {
                format!("{}...{} (len={})", &api_key[..4], &api_key[api_key.len()-4..], api_key.len())
            } else {
                format!("(len={})", api_key.len())
            };
            ::log::info!("Will fetch models from provider {} with API key: {}", provider.id, key_preview);

            self.providers_to_fetch.push(provider.id.clone());
        }

        self.providers_configured = true;

        // Start fetching from the first provider
        if !self.providers_to_fetch.is_empty() {
            self.start_fetch_for_provider(cx, scope, 0);
        }
    }

    /// Start fetching models from a specific provider by index
    fn start_fetch_for_provider(&mut self, cx: &mut Cx, scope: &mut Scope, index: usize) {
        if index >= self.providers_to_fetch.len() {
            ::log::info!("Finished fetching from all {} providers", self.fetched_provider_ids.len());
            self.fetch_in_progress = false;
            self.view.redraw(cx);
            return;
        }

        let provider_id = &self.providers_to_fetch[index];
        ::log::info!("Starting fetch for provider {} (index {})", provider_id, index);

        let Some(store) = scope.data.get::<Store>() else { return };

        // Get client for this provider from ProvidersManager (supports all client types)
        let Some(client) = store.providers_manager.get_bot_client(provider_id) else {
            ::log::warn!("No client for provider {}, skipping", provider_id);
            // Skip to next provider
            self.start_fetch_for_provider(cx, scope, index + 1);
            return;
        };

        // Set up the ChatController with this provider's client wrapped in A2uiClient
        {
            // Create A2UI wrapper around the client
            let a2ui_client = A2uiClient::new(client);
            self.a2ui_client = Some(a2ui_client.clone());

            let mut ctrl = self.chat_controller.lock().unwrap();
            ctrl.set_client(Some(Box::new(a2ui_client)));
            self.skip_chat_draw_frames = 2;

            // Don't set a default bot_id here - we'll restore the saved model
            // or select first available after models are loaded

            // Dispatch Load task to fetch models
            ::log::info!("Dispatching ChatTask::Load for provider {}", provider_id);
            ctrl.dispatch_task(ChatTask::Load);
        }

        self.current_provider_id = Some(provider_id.clone());
        self.fetch_index = index;
        self.fetch_in_progress = true;
        self.last_bots_count = 0;

        self.view.redraw(cx);
    }

    /// Apply provider icon to all bots' avatars
    fn apply_provider_icon_to_bots(bots: &mut Vec<Bot>, icon_path: Option<String>) {
        if let Some(path) = icon_path {
            for bot in bots.iter_mut() {
                bot.avatar = EntityAvatar::Image(path.clone());
            }
        }
    }

    /// Check for loaded bots and continue sequential fetching
    fn check_for_loaded_bots(&mut self, cx: &mut Cx, scope: &mut Scope) {
        if !self.fetch_in_progress {
            return;
        }
        // Get the bots from the controller state
        let mut bots: Vec<Bot> = {
            let ctrl = self.chat_controller.lock().unwrap();
            ctrl.state().bots.clone()
        };

        // Check if we have new bots (fetch completed)
        if bots.is_empty() || bots.len() == self.last_bots_count {
            return;
        }

        self.last_bots_count = bots.len();

        // Update the ProvidersManager with the loaded bots
        let Some(store) = scope.data.get_mut::<Store>() else { return };

        // Store bots for current provider
        if let Some(ref current_provider) = self.current_provider_id {
            // Apply provider icon to bot avatars before storing
            let icon_path = self.get_provider_icon_path(current_provider);
            Self::apply_provider_icon_to_bots(&mut bots, icon_path);

            ::log::info!("Loaded {} bots from provider {}", bots.len(), current_provider);
            store.providers_manager.set_provider_bots(current_provider, bots.clone());

            if !self.fetched_provider_ids.contains(current_provider) {
                self.fetched_provider_ids.push(current_provider.clone());
            }
        }

        // Move to next provider
        let next_index = self.fetch_index + 1;
        if next_index < self.providers_to_fetch.len() {
            self.start_fetch_for_provider(cx, scope, next_index);
        } else {
            // All providers fetched - combine bots into ChatController
            ::log::info!("All providers fetched, {} total bots available", store.providers_manager.get_all_bots().len());
            self.fetch_in_progress = false;

            // Update ChatController with filtered bots (only enabled models)
            let all_bots = store.providers_manager.get_all_bots();
            let enabled_bots = Self::filter_enabled_bots(all_bots, store);
            let num_bots = enabled_bots.len();
            ::log::info!("Setting {} enabled bots on ChatController (out of {} total)", num_bots, all_bots.len());
            {
                let mut ctrl = self.chat_controller.lock().unwrap();
                // VecMutation::Set automatically converts to ChatStateMutation::MutateBots
                ctrl.dispatch_mutation(VecMutation::Set(enabled_bots.clone()));

                // Verify bots were set
                let controller_bots = ctrl.state().bots.len();
                ::log::info!("ChatController now has {} bots", controller_bots);
            }

            // Get filtered bots before restore (restore may clear them due to set_client)
            let all_bots_for_reset = enabled_bots;

            // Restore the saved model selection (this may switch client which clears bots)
            self.restore_saved_model(scope);

            // Force re-setting the controller on the Chat widget now that bots are loaded
            // The Chat widget's set_chat_controller has an early return if the Arc pointer
            // is the same, so we need to set it to None first to force re-propagation
            // IMPORTANT: Do this BEFORE dispatching mutations so the new plugin receives them
            {
                let mut chat_ref = self.view.chat(ids!(main_content.chat));
                // First set to None to clear the existing controller
                chat_ref.write().set_chat_controller(cx, None);
                // Then set to our controller again to force propagation to child widgets
                chat_ref.write().set_chat_controller(cx, Some(self.chat_controller.clone()));
            }

            // Re-set the bots after restore (set_client clears them)
            // Do this AFTER force re-setting controller so the new plugin sees the mutation
            {
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(VecMutation::Set(all_bots_for_reset.clone()));
            }

            // Set up grouping with provider icons for the model selector
            self.setup_model_selector_grouping(scope);

            // Update A2UI toggle visibility based on current provider
            self.update_a2ui_toggle_visibility(cx, scope);

            // Redraw both the view and explicitly the chat widget
            self.view.redraw(cx);
            self.view.chat(ids!(main_content.chat)).redraw(cx);
        }
    }

    /// Parse a BotId string into (model_name, provider) tuple
    /// BotId format: <id_len>;<model_id>@<provider>
    fn parse_bot_id_string(bot_id_str: &str) -> (String, String) {
        // Split on first ';' to get id_length and rest
        if let Some((id_length_str, rest)) = bot_id_str.split_once(';') {
            if let Ok(id_length) = id_length_str.parse::<usize>() {
                if rest.len() >= id_length + 1 {
                    let model_name = &rest[..id_length];
                    // Skip the '@' separator
                    let provider = &rest[id_length + 1..];
                    return (model_name.to_string(), provider.to_string());
                }
            }
        }
        // Fallback: return empty strings if parsing fails
        (String::new(), String::new())
    }

    /// Track model selection changes and save to preferences
    /// Only tracks changes after the saved model has been restored
    fn track_model_selection(&mut self, scope: &mut Scope) {
        // Don't track until we've restored the saved model
        // This prevents the initial load from overwriting the user's saved selection
        if !self.restored_saved_model {
            return;
        }

        // Get current bot_id from controller
        let current_bot_id: Option<BotId> = {
            let ctrl = self.chat_controller.lock().unwrap();
            ctrl.state().bot_id.clone()
        };

        let current_bot_id_str = current_bot_id.as_ref().map(|id| id.as_str().to_string());

        // Check if it changed from what we last saved
        if current_bot_id_str != self.last_saved_bot_id {
            if let Some(ref bot_id) = current_bot_id {
                let bot_id_str = bot_id.as_str().to_string();
                ::log::info!("Model selection changed to: {}", bot_id_str);

                // Switch to the correct provider's client for this model
                self.switch_to_provider_for_bot(bot_id, scope);

                // Save to preferences
                if let Some(store) = scope.data.get_mut::<Store>() {
                    store.preferences.set_current_chat_model(Some(bot_id_str.clone()));
                }

                self.last_saved_bot_id = Some(bot_id_str);
            } else {
                self.last_saved_bot_id = None;
            }
        }
    }

    /// Switch to the correct provider's client for a given bot
    fn switch_to_provider_for_bot(&mut self, bot_id: &BotId, scope: &mut Scope) {
        let Some(store) = scope.data.get::<Store>() else { return };

        // Find which provider this bot belongs to
        if let Some(provider_id) = store.providers_manager.get_provider_for_bot(bot_id) {
            // Only switch if it's a different provider
            if self.current_provider_id.as_deref() != Some(provider_id) {
                // Use get_bot_client to support all client types (text, realtime, image)
                if let Some(client) = store.providers_manager.get_bot_client(provider_id) {
                    let all_bots = store.providers_manager.get_all_bots();
                    let enabled_bots = Self::filter_enabled_bots(all_bots, store);

                    let a2ui_client = A2uiClient::new(client);
                    self.a2ui_client = Some(a2ui_client.clone());

                    {
                        let mut ctrl = self.chat_controller.lock().unwrap();
                        // Preserve bot_id — set_client clears all state
                        let saved_bot_id = ctrl.state().bot_id.clone();
                        ctrl.set_client(Some(Box::new(a2ui_client)));
                        // Restore bots + bot_id atomically so draw never sees empty state
                        ctrl.dispatch_mutation(VecMutation::Set(enabled_bots));
                        if let Some(bid) = saved_bot_id {
                            ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(bid)));
                        }
                    }

                    self.current_provider_id = Some(provider_id.to_string());
                    ::log::info!("Switched to provider: {} for model", provider_id);
                }
            }
        } else {
            ::log::warn!("Could not find provider for bot: {}", bot_id.as_str());
        }
    }

    /// Wrapper for switch_to_provider_for_bot that also updates A2UI toggle visibility
    /// Called when model selection changes (needs Cx for UI updates)
    fn switch_to_provider_for_bot_with_ui(&mut self, cx: &mut Cx, bot_id: &BotId, scope: &mut Scope) {
        self.switch_to_provider_for_bot(bot_id, scope);
        self.update_a2ui_toggle_visibility(cx, scope);
    }

    /// Filter bots based on enabled status in provider preferences
    /// Returns only bots that are either:
    /// 1. Not in the provider's models list (default to enabled)
    /// 2. Explicitly enabled in the provider's models list
    fn filter_enabled_bots(all_bots: &[Bot], store: &Store) -> Vec<Bot> {
        all_bots.iter()
            .filter(|bot| {
                // Find which provider this bot belongs to
                let provider_id = store.providers_manager.get_provider_for_bot(&bot.id);

                if let Some(provider_id) = provider_id {
                    // Get the provider preferences
                    let provider_id_string = provider_id.to_string();
                    if let Some(provider) = store.preferences.get_provider(&provider_id_string) {
                        // Check if this model is in the models list
                        let model_name = bot.id.id();

                        // Find the model in the provider's models list
                        if let Some((_, enabled)) = provider.models.iter()
                            .find(|(name, _)| name == model_name || name == &bot.name)
                        {
                            return *enabled;
                        }
                        // Model not in list - default to disabled
                        // (user must explicitly enable models via Settings)
                        return provider.models.is_empty();
                    }
                }
                // Provider not found - default to showing the bot
                true
            })
            .cloned()
            .collect()
    }

    /// Restore the saved model selection from preferences
    fn restore_saved_model(&mut self, scope: &mut Scope) {
        if self.restored_saved_model {
            return;
        }

        let Some(store) = scope.data.get::<Store>() else { return };

        // Get the saved model from preferences
        let saved_model = store.preferences.get_current_chat_model();
        let all_bots = store.providers_manager.get_all_bots();

        // Filter to only enabled bots
        let enabled_bots = Self::filter_enabled_bots(all_bots, store);

        if enabled_bots.is_empty() {
            self.restored_saved_model = true;
            return;
        }

        // If no saved model, select the first available enabled model
        if saved_model.is_none() {
            let first_bot_id = enabled_bots[0].id.clone();
            let first_bot_name = enabled_bots[0].name.clone();
            let _ = store;  // Release the borrow on store

            ::log::info!("No saved model, selecting first available: {}", first_bot_name);

            // Switch to the correct provider for this bot
            self.switch_to_provider_for_bot(&first_bot_id, scope);

            {
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(first_bot_id.clone())));
            }
            self.last_saved_bot_id = Some(first_bot_id.as_str().to_string());
            self.restored_saved_model = true;
            return;
        }

        let saved_model = saved_model.unwrap().to_string();
        ::log::info!("Restoring saved model: {}", saved_model);

        // Parse the saved model to extract model name and provider
        // BotId format: <id_len>;<model_id>@<provider>
        let (saved_model_name, saved_provider) = Self::parse_bot_id_string(&saved_model);

        // Check if this model exists in the enabled bots
        let all_bots = &enabled_bots;

        // First try exact match
        let mut matching_bot = all_bots.iter().find(|bot| bot.id.as_str() == saved_model);

        // If no exact match, try matching by model name (handling models/ prefix)
        if matching_bot.is_none() {
            matching_bot = all_bots.iter().find(|bot| {
                let (bot_model_name, bot_provider) = Self::parse_bot_id_string(bot.id.as_str());
                // Match if providers are the same and either:
                // 1. Model names match exactly
                // 2. Bot model is "models/<saved_model>"
                // 3. Saved model is "models/<bot_model>"
                bot_provider == saved_provider && (
                    bot_model_name == saved_model_name ||
                    bot_model_name == format!("models/{}", saved_model_name) ||
                    saved_model_name == format!("models/{}", bot_model_name)
                )
            });
        }

        if let Some(bot) = matching_bot {
            ::log::info!("Found saved model, selecting: {}", bot.name);

            let matched_bot_id = bot.id.clone();
            let matched_bot_id_str = bot.id.as_str().to_string();

            // Switch to the correct provider for this bot
            self.switch_to_provider_for_bot(&matched_bot_id, scope);

            // Set the bot_id on the controller
            {
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(matched_bot_id)));
            }

            // Update our tracking with the actual matched bot ID (for future exact matching)
            self.last_saved_bot_id = Some(matched_bot_id_str.clone());

            // Also save the correct ID to preferences for future exact matching
            if let Some(store) = scope.data.get_mut::<Store>() {
                if matched_bot_id_str != saved_model {
                    store.preferences.set_current_chat_model(Some(matched_bot_id_str));
                }
            }
        } else {
            // Saved model not found, select first available
            ::log::warn!("Saved model '{}' not found, selecting first available", saved_model);
            let first_bot_id = all_bots[0].id.clone();

            // Switch to the correct provider for this bot
            self.switch_to_provider_for_bot(&first_bot_id, scope);

            {
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(first_bot_id.clone())));
            }
            self.last_saved_bot_id = Some(first_bot_id.as_str().to_string());
        }

        self.restored_saved_model = true;
    }

    /// React when store.active_local_model changes — switch ChatController to ominix-local
    fn maybe_inject_local_model(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let current_local_model = {
            let Some(store) = scope.data.get::<Store>() else { return };
            store.active_local_model.clone()
        };

        if current_local_model == self.last_active_local_model {
            return;
        }

        self.last_active_local_model = current_local_model.clone();

        let Some(model_id) = current_local_model else {
            // Local model cleared — nothing extra needed; normal provider fetch will resume
            return;
        };

        // Get all bots (including newly injected local one) and switch client
        let client = {
            let Some(store) = scope.data.get::<Store>() else { return };
            store.providers_manager.get_bot_client("ominix-local")
        };

        if let Some(client) = client {
            let (all_bots, enabled_bots) = {
                let Some(store) = scope.data.get::<Store>() else { return };
                let all = store.providers_manager.get_all_bots().to_vec();
                let enabled = Self::filter_enabled_bots(&all, store);
                (all, enabled)
            };

            // Only call set_client if the provider actually changed.
            let provider_changed = self.current_provider_id.as_deref() != Some("ominix-local");
            if provider_changed {
                let a2ui_client = A2uiClient::new(client);
                self.a2ui_client = Some(a2ui_client.clone());
                {
                    let mut ctrl = self.chat_controller.lock().unwrap();
                    let saved_bot_id = ctrl.state().bot_id.clone();
                    ctrl.set_client(Some(Box::new(a2ui_client)));
                    ctrl.dispatch_mutation(VecMutation::Set(enabled_bots));
                    if let Some(bid) = saved_bot_id {
                        ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(bid)));
                    }
                }
                self.current_provider_id = Some("ominix-local".to_string());
                self.force_reset_controller_on_widget(cx);
                self.skip_chat_draw_frames = 2;
            } else {
                // Same provider — just update bots list without touching the client
                let mut ctrl = self.chat_controller.lock().unwrap();
                ctrl.dispatch_mutation(VecMutation::Set(enabled_bots));
            }
            let _ = all_bots;

            self.view.redraw(cx);
            self.view.chat(ids!(main_content.chat)).redraw(cx);
        }

        // Select the local model bot
        let bot_id = moly_kit::aitk::protocol::BotId::new(&model_id);
        {
            let mut ctrl = self.chat_controller.lock().unwrap();
            ctrl.dispatch_mutation(ChatStateMutation::SetBotId(Some(bot_id)));
        }

        self.last_saved_bot_id = Some(model_id.clone());
        self.restored_saved_model = true;
        self.providers_configured = true;

        // Update model selector grouping
        self.setup_model_selector_grouping(scope);

        self.view.redraw(cx);
        self.view.chat(ids!(main_content.chat)).redraw(cx);
    }

    /// Update the A2UI toggle visibility in PromptInput based on current provider support
    fn update_a2ui_toggle_visibility(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(store) = scope.data.get::<Store>() else { return };

        // Check if current provider supports and has A2UI enabled
        let a2ui_available = if let Some(ref provider_id) = self.current_provider_id {
            store.preferences.get_provider(provider_id)
                .map(|p| p.is_a2ui_ready())
                .unwrap_or(false)
        } else {
            false
        };

        // Update the PromptInput in the Chat widget
        let mut chat = self.view.chat(ids!(main_content.chat));
        chat.write().prompt_input_ref().write().set_a2ui_available(cx, a2ui_available);

        // Also update the welcome prompt
        self.view.prompt_input(ids!(main_content.welcome_overlay.welcome_prompt))
            .write()
            .set_a2ui_available(cx, a2ui_available);

        ::log::info!("A2UI toggle visibility updated: available={}", a2ui_available);
    }
}
