//! Settings Screen Widget Implementation

pub mod design;

use makepad_widgets::*;
use makepad_component::widgets::{MpSwitchWidgetExt, MpSwitchWidgetRefExt};
use moly_data::{Store, ProviderId, ProviderConnectionStatus, UpdateInfo, check_for_update};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::path::Path;
use serde::Deserialize;

/// Result from connection test stored in shared state
#[derive(Clone, Debug)]
struct ConnectionTestResult {
    provider_id: String,
    status: ProviderConnectionStatus,
    model_count: Option<usize>,
    models: Vec<String>,
}

/// Shared state for async connection testing
type ConnectionTestState = Arc<Mutex<Option<ConnectionTestResult>>>;

/// Response from OpenAI-compatible /models endpoint
#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ModelInfo {
    id: String,
}

#[derive(Live, LiveHook, Widget)]
pub struct SettingsApp {
    #[deref]
    pub view: View,

    /// Provider icons loaded from live_design
    #[live]
    provider_icons: Vec<LiveDependency>,

    #[rust]
    selected_provider_id: Option<ProviderId>,

    /// Shared state for connection test results
    #[rust]
    connection_test_state: ConnectionTestState,

    /// Whether a connection test is currently in progress
    #[rust]
    connection_test_in_progress: bool,

    /// Current connection status for selected provider
    #[rust]
    connection_status: ProviderConnectionStatus,

    /// Number of models found (if connected)
    #[rust]
    model_count: Option<usize>,

    /// List of models fetched from the provider (name, enabled)
    #[rust]
    fetched_models: Vec<(String, bool)>,

    /// Whether the Add Provider modal is visible
    #[rust]
    modal_visible: bool,

    /// Cached list of provider IDs for the PortalList
    #[rust]
    provider_ids: Vec<String>,

    /// Connection status per provider (persists after testing)
    #[rust]
    provider_statuses: HashMap<String, ProviderConnectionStatus>,

    /// Receiver for update check result
    #[rust]
    update_rx: Option<mpsc::Receiver<Option<UpdateInfo>>>,
}

impl Widget for SettingsApp {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // Initialize shared state if needed
        if Arc::strong_count(&self.connection_test_state) == 0 {
            self.connection_test_state = Arc::new(Mutex::new(None));
        }

        // Initialize with first provider selected (before handling events)
        if self.selected_provider_id.is_none() {
            self.selected_provider_id = Some("openai".to_string());
            self.connection_test_state = Arc::new(Mutex::new(None));
            self.load_provider_data(cx, scope);
            self.view.redraw(cx);

            // Log icon paths at startup for debugging (debug level)
            ::log::debug!("Provider icons count: {}", self.provider_icons.len());
        }

        // Check for connection test results
        self.check_connection_test_result(cx, scope);

        // Handle events
        let actions = cx.capture_actions(|cx| {
            self.view.handle_event(cx, event, scope);
        });

        // Handle provider list item clicks
        self.handle_provider_list_clicks(cx, scope, &actions);

        // Save button click
        if self.view.button(ids!(save_button)).clicked(&actions) {
            self.save_provider(cx, scope);
        }

        // Test Connection button click
        if self.view.button(ids!(test_button)).clicked(&actions) {
            self.test_connection(cx, scope);
        }

        // Add Provider button click
        if self.view.button(ids!(add_provider_button)).clicked(&actions) {
            self.open_add_provider_modal(cx);
        }

        // Close modal button clicks
        if self.view.button(ids!(close_modal_button)).clicked(&actions)
            || self.view.button(ids!(cancel_modal_button)).clicked(&actions) {
            self.close_add_provider_modal(cx);
        }

        // Save new provider button click
        if self.view.button(ids!(save_new_provider_button)).clicked(&actions) {
            self.save_new_provider(cx, scope);
        }

        // Delete provider button click
        if self.view.button(ids!(delete_provider_button)).clicked(&actions) {
            self.delete_provider(cx, scope);
        }

        // Handle model checkbox clicks
        self.handle_model_checkbox_clicks(cx, scope, &actions);

        // Handle Select All toggle
        self.handle_select_all_toggle(cx, scope, &actions);

        // Handle A2UI toggle
        self.handle_a2ui_toggle(cx, scope, &actions);

        // Check for Updates button
        if self.view.button(ids!(check_update_button)).clicked(&actions) {
            self.view.label(ids!(update_status_label)).set_text(cx, "Checking...");
            self.view.redraw(cx);
            let (tx, rx) = mpsc::channel();
            self.update_rx = Some(rx);
            std::thread::spawn(move || {
                let result = check_for_update(env!("CARGO_PKG_VERSION"));
                let _ = tx.send(match result {
                    Ok(info) => info,
                    Err(_) => None,
                });
            });
        }

        // Poll update check result
        if let Some(info) = self.update_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.update_rx = None;
            let msg = match info {
                Some(ref u) => format!("v{} available — click Download", u.version),
                None => "You're up to date!".to_string(),
            };
            self.view.label(ids!(update_status_label)).set_text(cx, &msg);
            self.view.redraw(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Update selection highlighting
        self.update_selection(cx);

        // Show/hide models section based on fetched models
        let has_models = !self.fetched_models.is_empty();
        self.view.view(ids!(models_section)).set_visible(cx, has_models);

        // Update select_all_toggle state: ON if all models enabled, OFF otherwise
        if has_models {
            let all_enabled = self.fetched_models.iter().all(|(_, enabled)| *enabled);
            self.view.mp_switch(ids!(select_all_toggle)).set_on(cx, all_enabled);
        }

        // Show/hide add provider modal
        self.view.view(ids!(add_provider_modal)).set_visible(cx, self.modal_visible);

        // Set version label
        self.view.label(ids!(version_label)).set_text(cx,
            &format!("Moxin Studio v{}", env!("CARGO_PKG_VERSION")));

        // Update provider list from store
        if let Some(store) = scope.data.get::<Store>() {
            self.provider_ids = store.preferences.providers_preferences
                .iter()
                .map(|p| p.id.clone())
                .collect();
        }

        // Get PortalList widget UIDs for step pattern
        let providers_list = self.view.portal_list(ids!(providers_list));
        let providers_list_uid = providers_list.widget_uid();
        let models_list = self.view.portal_list(ids!(models_list));
        let models_list_uid = models_list.widget_uid();

        // Draw with PortalList handling
        while let Some(widget) = self.view.draw_walk(cx, scope, walk).step() {
            // Draw providers list
            if widget.widget_uid() == providers_list_uid {
                self.draw_providers_list(cx, scope, widget);
            }
            // Draw models list
            else if widget.widget_uid() == models_list_uid {
                if let Some(mut list) = widget.as_portal_list().borrow_mut() {
                    list.set_item_range(cx, 0, self.fetched_models.len());

                    while let Some(item_id) = list.next_visible_item(cx) {
                        if item_id < self.fetched_models.len() {
                            let (model_name, enabled) = &self.fetched_models[item_id];
                            let item_widget = list.item(cx, item_id, live_id!(ModelItem));

                            // Set model name
                            item_widget.label(ids!(model_name)).set_text(cx, model_name);

                            // Set switch state
                            item_widget.mp_switch(ids!(model_enabled)).set_on(cx, *enabled);

                            item_widget.draw_all(cx, scope);
                        }
                    }
                }
            }
        }

        DrawStep::done()
    }
}

impl SettingsApp {
    /// Get provider icon from the loaded LiveDependency list
    fn get_provider_icon(&self, provider_id: &str) -> Option<&LiveDependency> {
        // Icons are stored in order: openai, anthropic, gemini, ollama, deepseek, nvidia, groq, kimi, zhipu
        let index = match provider_id {
            "openai" => Some(0),
            "anthropic" => Some(1),
            "gemini" => Some(2),
            "ollama" => Some(3),
            "deepseek" => Some(4),
            "nvidia" => Some(5),
            "groq" => Some(6),
            "kimi" => Some(7),
            "zhipu" => Some(8),
            _ => None,
        };
        index.and_then(|i| self.provider_icons.get(i))
    }

    fn select_provider(&mut self, cx: &mut Cx, scope: &mut Scope, id: &str) {
        self.selected_provider_id = Some(id.to_string());
        // Reset connection status when changing providers
        self.connection_status = ProviderConnectionStatus::NotConnected;
        self.model_count = None;
        self.fetched_models.clear();
        self.connection_test_in_progress = false;
        self.load_provider_data(cx, scope);
        self.view.redraw(cx);
    }

    fn load_provider_data(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(provider_id) = self.selected_provider_id.clone() else { return };

        if let Some(store) = scope.data.get::<Store>() {
            if let Some(provider) = store.preferences.get_provider(&provider_id) {
                ::log::info!("Loading provider data for {}: url={}, has_key={}, enabled={}",
                    provider_id, provider.url, provider.api_key.is_some(), provider.enabled);

                // Update title
                self.view.label(ids!(provider_title)).set_text(cx, &provider.name);

                // Update provider title icon using LiveDependency from live_design
                if let Some(icon_dep) = self.get_provider_icon(&provider_id) {
                    let icon_path = icon_dep.as_str();
                    let _ = self.view.image(ids!(provider_title_icon)).load_image_file_by_path(cx, Path::new(icon_path));
                }

                // Update URL input
                self.view.text_input(ids!(api_host_input)).set_text(cx, &provider.url);

                // Update API key input - show masked if exists
                let key_text = provider.api_key.clone().unwrap_or_default();
                ::log::info!("Setting API key input: len={}", key_text.len());
                self.view.text_input(ids!(api_key_input)).set_text(cx, &key_text);

                // Show/hide delete button based on whether provider was custom added
                self.view.button(ids!(delete_provider_button)).set_visible(cx, provider.was_customly_added);

                // Show/hide A2UI section based on provider type (only for OpenAI-compatible)
                let supports_a2ui = provider.supports_a2ui();
                self.view.view(ids!(a2ui_section)).set_visible(cx, supports_a2ui);

                // Set A2UI toggle state
                if supports_a2ui {
                    self.view.mp_switch(ids!(a2ui_toggle)).set_on(cx, provider.a2ui_enabled);
                }

                // Clear status message
                self.view.label(ids!(status_message)).set_text(cx, "");
            } else {
                ::log::warn!("Provider {} not found in preferences", provider_id);
            }
        } else {
            ::log::warn!("Store not available in scope");
        }
    }

    fn save_provider(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(provider_id) = &self.selected_provider_id else { return };

        // Get values from inputs
        let url = self.view.text_input(ids!(api_host_input)).text();
        let api_key_text = self.view.text_input(ids!(api_key_input)).text();

        ::log::info!("save_provider: provider={}, url={}, api_key_len={}",
            provider_id, url, api_key_text.len());

        // Save to Store
        if let Some(store) = scope.data.get_mut::<Store>() {
            store.preferences.set_provider_url(provider_id, url);

            // Only update API key if user entered something, or if explicitly clearing
            // This prevents accidentally clearing the key if text input returns empty
            if !api_key_text.is_empty() {
                ::log::info!("save_provider: saving API key (len={})", api_key_text.len());
                store.preferences.set_provider_api_key(provider_id, Some(api_key_text));
            } else {
                // Check if there was already a key - if so, don't clear it
                let existing_key = store.preferences.get_provider(provider_id)
                    .and_then(|p| p.api_key.clone());
                if existing_key.is_some() {
                    ::log::warn!("save_provider: text input empty but existing key found, NOT clearing");
                } else {
                    ::log::info!("save_provider: no API key to save");
                }
            }

            // Show success message
            self.view.label(ids!(status_message)).set_text(cx, "Settings saved!");

            ::log::info!("Saved provider settings for {}", provider_id);
        }

        self.view.redraw(cx);
    }

    fn update_selection(&mut self, _cx: &mut Cx2d) {
        // Selection highlighting is now handled in draw_providers_list
    }

    /// Draw the providers PortalList
    fn draw_providers_list(&mut self, cx: &mut Cx2d, scope: &mut Scope, widget: WidgetRef) {
        let binding = widget.as_portal_list();
        let Some(mut list) = binding.borrow_mut() else { return };

        list.set_item_range(cx, 0, self.provider_ids.len());

        while let Some(item_id) = list.next_visible_item(cx) {
            if item_id >= self.provider_ids.len() {
                continue;
            }

            let provider_id = &self.provider_ids[item_id];
            let item_widget = list.item(cx, item_id, live_id!(ProviderListItem));

            // Get provider info from store
            let (name, enabled) = if let Some(store) = scope.data.get::<Store>() {
                if let Some(provider) = store.preferences.get_provider(provider_id) {
                    (provider.name.clone(), provider.enabled)
                } else {
                    (provider_id.clone(), false)
                }
            } else {
                (provider_id.clone(), false)
            };

            // Set selection state
            let is_selected = self.selected_provider_id.as_deref() == Some(provider_id.as_str());
            let selected_val = if is_selected { 1.0 } else { 0.0 };

            // Get status for this provider
            let status_val = match self.provider_statuses.get(provider_id) {
                Some(ProviderConnectionStatus::NotConnected) | None => 0.0,
                Some(ProviderConnectionStatus::Connecting) => 1.0,
                Some(ProviderConnectionStatus::Connected) => 2.0,
                Some(ProviderConnectionStatus::Error(_)) => 3.0,
            };

            // Apply styling
            item_widget.apply_over(cx, live!{
                draw_bg: { selected: (selected_val) }
            });
            item_widget.label(ids!(provider_name)).set_text(cx, &name);

            // Set status dot
            item_widget.view(ids!(status_dot)).apply_over(cx, live!{
                draw_bg: { status: (status_val) }
            });

            // Set icon if available - use file path loading
            if let Some(icon_dep) = self.get_provider_icon(provider_id) {
                let icon_path = icon_dep.as_str();
                let image_ref = item_widget.image(ids!(provider_icon));
                ::log::debug!("Icon for {}: path={}", provider_id, icon_path);
                // Use file path loading since as_str() returns resolved filesystem path
                match image_ref.load_image_file_by_path(cx, Path::new(icon_path)) {
                    Ok(_) => ::log::debug!("Icon loaded OK for {}", provider_id),
                    Err(e) => ::log::warn!("Icon load failed for {}: {:?}", provider_id, e),
                }
            } else {
                ::log::debug!("No icon configured for provider: {}", provider_id);
            }

            // Set enabled switch state
            item_widget.mp_switch(ids!(provider_enabled)).set_on(cx, enabled);

            item_widget.draw_all(cx, scope);
        }
    }

    /// Handle clicks on provider list items
    fn handle_provider_list_clicks(&mut self, cx: &mut Cx, scope: &mut Scope, actions: &Actions) {
        let providers_list = self.view.portal_list(ids!(providers_list));

        for (item_id, item) in providers_list.items_with_actions(actions) {
            // Handle enabled switch toggle
            let switch = item.mp_switch(ids!(provider_enabled));
            if let Some(new_state) = switch.changed(&actions) {
                if item_id < self.provider_ids.len() {
                    let provider_id = self.provider_ids[item_id].clone();
                    // Save enabled state to preferences
                    if let Some(store) = scope.data.get_mut::<Store>() {
                        store.preferences.set_provider_enabled(&provider_id, new_state);
                        ::log::info!("Provider '{}' enabled: {}", provider_id, new_state);
                    }
                    self.view.redraw(cx);
                }
                continue; // Don't select provider when toggling checkbox
            }

            // Check for finger down on the item (for selection)
            if let Some(fd) = item.as_view().finger_down(actions) {
                if fd.tap_count == 1 && item_id < self.provider_ids.len() {
                    let provider_id = self.provider_ids[item_id].clone();
                    self.select_provider(cx, scope, &provider_id);
                }
            }
        }
    }

    /// Handle model checkbox toggle events
    fn handle_model_checkbox_clicks(&mut self, cx: &mut Cx, scope: &mut Scope, actions: &Actions) {
        let models_list = self.view.portal_list(ids!(models_list));

        for (item_id, item) in models_list.items_with_actions(actions) {
            let switch = item.mp_switch(ids!(model_enabled));
            if let Some(new_state) = switch.changed(&actions) {
                if item_id < self.fetched_models.len() {
                    let model_name = self.fetched_models[item_id].0.clone();

                    // Update local state
                    self.fetched_models[item_id].1 = new_state;

                    // Save to preferences
                    self.save_model_enabled_state(scope, &model_name, new_state);

                    ::log::info!("Model '{}' enabled: {}", model_name, new_state);
                    self.view.redraw(cx);
                }
            }
        }
    }

    /// Handle the A2UI toggle for the current provider
    fn handle_a2ui_toggle(&mut self, cx: &mut Cx, scope: &mut Scope, actions: &Actions) {
        let a2ui_toggle = self.view.mp_switch(ids!(a2ui_toggle));
        if let Some(new_state) = a2ui_toggle.changed(&actions) {
            let Some(provider_id) = &self.selected_provider_id else { return };

            if let Some(store) = scope.data.get_mut::<Store>() {
                if let Some(provider) = store.preferences.get_provider_mut(provider_id) {
                    provider.a2ui_enabled = new_state;
                    store.preferences.save();
                    ::log::info!("Provider '{}' A2UI enabled: {}", provider_id, new_state);
                }
            }
            self.view.redraw(cx);
        }
    }

    /// Handle the Select All toggle for models
    fn handle_select_all_toggle(&mut self, cx: &mut Cx, scope: &mut Scope, actions: &Actions) {
        let select_all_toggle = self.view.mp_switch(ids!(select_all_toggle));
        if let Some(new_state) = select_all_toggle.changed(&actions) {
            // Set all models to the new state
            for (_, enabled) in &mut self.fetched_models {
                *enabled = new_state;
            }

            // Save all model states to preferences
            if let Some(provider_id) = &self.selected_provider_id {
                if let Some(store) = scope.data.get_mut::<Store>() {
                    if let Some(provider) = store.preferences.get_provider_mut(provider_id) {
                        // Update all models in preferences
                        for (model_name, enabled) in &self.fetched_models {
                            if let Some(model_entry) = provider.models.iter_mut().find(|(name, _)| name == model_name) {
                                model_entry.1 = *enabled;
                            } else {
                                provider.models.push((model_name.clone(), *enabled));
                            }
                        }
                        store.preferences.save();
                    }
                }
            }

            ::log::info!("Select All toggled: all models set to {}", new_state);
            self.view.redraw(cx);
        }
    }

    /// Save model enabled state to preferences
    fn save_model_enabled_state(&mut self, scope: &mut Scope, model_name: &str, enabled: bool) {
        let Some(provider_id) = &self.selected_provider_id else { return };

        if let Some(store) = scope.data.get_mut::<Store>() {
            if let Some(provider) = store.preferences.get_provider_mut(provider_id) {
                // Find and update or add the model entry
                if let Some(model_entry) = provider.models.iter_mut().find(|(name, _)| name == model_name) {
                    model_entry.1 = enabled;
                } else {
                    provider.models.push((model_name.to_string(), enabled));
                }
                store.preferences.save();
            }
        }
    }

    /// Start a connection test for the currently selected provider
    fn test_connection(&mut self, cx: &mut Cx, _scope: &mut Scope) {
        let Some(provider_id) = self.selected_provider_id.clone() else { return };

        // Get provider URL and API key from the current input values
        let url = self.view.text_input(ids!(api_host_input)).text().trim().to_string();
        let api_key = self.view.text_input(ids!(api_key_input)).text().trim().to_string();

        eprintln!(
            "[Settings] test_connection: provider={}, url='{}', api_key len={}, first_8='{}'",
            provider_id,
            url,
            api_key.len(),
            api_key.chars().take(8).collect::<String>()
        );

        if api_key.is_empty() {
            self.connection_status = ProviderConnectionStatus::Error("No API key provided".to_string());
            self.view.label(ids!(status_message)).set_text(cx, "Error: No API key provided");
            self.view.redraw(cx);
            return;
        }

        // Update status to connecting
        self.connection_status = ProviderConnectionStatus::Connecting;
        self.provider_statuses.insert(provider_id.clone(), ProviderConnectionStatus::Connecting);
        self.connection_test_in_progress = true;
        self.view.label(ids!(status_message)).set_text(cx, "Testing connection...");
        self.view.redraw(cx);

        // Clone shared state for the thread
        let state = self.connection_test_state.clone();
        let provider_id_clone = provider_id.clone();
        let url_clone = url.clone();
        let api_key_clone = api_key.clone();

        // Spawn a thread to test the connection
        std::thread::spawn(move || {
            let result = test_provider_connection(&url_clone, &api_key_clone);

            let test_result = match result {
                Ok((model_count, models)) => ConnectionTestResult {
                    provider_id: provider_id_clone,
                    status: ProviderConnectionStatus::Connected,
                    model_count: Some(model_count),
                    models,
                },
                Err(e) => ConnectionTestResult {
                    provider_id: provider_id_clone,
                    status: ProviderConnectionStatus::Error(e),
                    model_count: None,
                    models: vec![],
                },
            };

            // Store result in shared state
            if let Ok(mut guard) = state.lock() {
                *guard = Some(test_result);
            }
        });
    }

    /// Check for connection test results and update UI
    fn check_connection_test_result(&mut self, cx: &mut Cx, scope: &mut Scope) {
        if !self.connection_test_in_progress {
            return;
        }

        // Try to get the result from shared state
        let result = {
            if let Ok(mut guard) = self.connection_test_state.lock() {
                guard.take()
            } else {
                None
            }
        };

        if let Some(test_result) = result {
            // Store the status for this provider (for the list indicator)
            self.provider_statuses.insert(
                test_result.provider_id.clone(),
                test_result.status.clone()
            );

            // Only apply detailed results if this is for the currently selected provider
            if self.selected_provider_id.as_ref() == Some(&test_result.provider_id) {
                self.connection_status = test_result.status.clone();
                self.model_count = test_result.model_count;
                self.connection_test_in_progress = false;

                // Get stored model preferences for this provider
                let stored_models: HashMap<String, bool> = if let Some(store) = scope.data.get::<Store>() {
                    if let Some(provider) = store.preferences.get_provider(&test_result.provider_id) {
                        provider.models.iter().cloned().collect()
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                };

                // Merge fetched models with stored enabled state
                self.fetched_models = test_result.models.into_iter().map(|name| {
                    // Use stored preference, default to enabled if not found
                    let enabled = stored_models.get(&name).copied().unwrap_or(true);
                    (name, enabled)
                }).collect();

                // Update status message
                let status_text = match &test_result.status {
                    ProviderConnectionStatus::Connected => {
                        if let Some(count) = test_result.model_count {
                            format!("Connected! Found {} models", count)
                        } else {
                            "Connected!".to_string()
                        }
                    }
                    ProviderConnectionStatus::Error(e) => format!("Error: {}", e),
                    _ => String::new(),
                };
                self.view.label(ids!(status_message)).set_text(cx, &status_text);
            }
            self.view.redraw(cx);
        }
    }

    /// Open the Add Provider modal
    fn open_add_provider_modal(&mut self, cx: &mut Cx) {
        self.modal_visible = true;
        // Clear the input fields
        self.view.text_input(ids!(new_provider_name)).set_text(cx, "");
        self.view.text_input(ids!(new_provider_url)).set_text(cx, "https://api.example.com/v1");
        self.view.text_input(ids!(new_provider_key)).set_text(cx, "");
        self.view.redraw(cx);
    }

    /// Close the Add Provider modal
    fn close_add_provider_modal(&mut self, cx: &mut Cx) {
        self.modal_visible = false;
        self.view.redraw(cx);
    }

    /// Save a new provider from the modal form
    fn save_new_provider(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let name = self.view.text_input(ids!(new_provider_name)).text();
        let url = self.view.text_input(ids!(new_provider_url)).text();
        let api_key = self.view.text_input(ids!(new_provider_key)).text();

        // Validate inputs
        if name.trim().is_empty() {
            ::log::warn!("Provider name is required");
            return;
        }
        if url.trim().is_empty() {
            ::log::warn!("Provider URL is required");
            return;
        }

        // Generate a unique ID from the name
        let id = name.trim().to_lowercase().replace(' ', "_");

        ::log::info!("Adding new provider: id={}, name={}, url={}", id, name, url);

        // Add to preferences
        if let Some(store) = scope.data.get_mut::<Store>() {
            // Check if provider already exists
            if store.preferences.get_provider(&id).is_some() {
                ::log::warn!("Provider with id '{}' already exists", id);
                return;
            }

            // Create new provider
            let mut new_provider = moly_data::ProviderPreferences::new(&id, name.trim(), url.trim());
            new_provider.was_customly_added = true;
            new_provider.enabled = true;
            if !api_key.is_empty() {
                new_provider.api_key = Some(api_key);
            }

            // Add to preferences and save
            store.preferences.providers_preferences.push(new_provider);
            store.preferences.save();

            ::log::info!("New provider '{}' added successfully", id);
        }

        // Close modal and refresh
        self.modal_visible = false;
        self.view.redraw(cx);
    }

    /// Delete a custom provider
    fn delete_provider(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(provider_id) = self.selected_provider_id.clone() else { return };

        if let Some(store) = scope.data.get_mut::<Store>() {
            // Find and verify the provider can be deleted
            if let Some(provider) = store.preferences.get_provider(&provider_id) {
                if !provider.was_customly_added {
                    ::log::warn!("Cannot delete built-in provider: {}", provider_id);
                    return;
                }
            } else {
                ::log::warn!("Provider not found: {}", provider_id);
                return;
            }

            // Remove the provider
            store.preferences.providers_preferences.retain(|p| p.id != provider_id);
            store.preferences.save();
            ::log::info!("Deleted provider: {}", provider_id);

            // Select the first provider
            self.selected_provider_id = Some("openai".to_string());
            self.load_provider_data(cx, scope);
        }

        self.view.redraw(cx);
    }
}

/// Test connection to a provider by fetching models
/// Returns (model_count, model_names) on success, or an error message on failure
fn test_provider_connection(base_url: &str, api_key: &str) -> Result<(usize, Vec<String>), String> {
    use reqwest::blocking::Client;
    use std::time::Duration;

    let base = base_url.trim_end_matches('/');

    // Try multiple endpoint patterns (different providers use different paths)
    let endpoints_to_try = [
        format!("{}/models", base),           // OpenAI standard: /v1/models
        format!("{}/v1/models", base),        // Some need explicit /v1
        format!("{}", base),                  // Base URL might already include /models
    ];

    // Create blocking client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut last_error = String::new();

    for models_url in &endpoints_to_try {
        eprintln!(
            "[Settings] Testing connection to: {} (key len={})",
            models_url,
            api_key.len()
        );

        // Make request to models endpoint
        let response = match client
            .get(models_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send()
        {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    "Connection timed out".to_string()
                } else if e.is_connect() {
                    "Failed to connect to server".to_string()
                } else {
                    format!("Request failed: {}", e)
                };
                eprintln!("[Settings] Connection error: {}", last_error);
                continue;
            }
        };

        let status = response.status();
        eprintln!("[Settings] Response status: {}", status);

        // If 404, try next endpoint
        if status.as_u16() == 404 {
            last_error = format!("Endpoint not found: {}", models_url);
            continue;
        }

        // Check response status
        if !status.is_success() {
            let error_text = response.text().unwrap_or_default();
            eprintln!("[Settings] Error response: {}", &error_text[..error_text.len().min(500)]);
            return Err(match status.as_u16() {
                401 => "Invalid API key".to_string(),
                403 => "Access denied".to_string(),
                429 => "Rate limited".to_string(),
                _ => format!("HTTP {}: {}", status.as_u16(), error_text),
            });
        }

        // Parse response
        let body = match response.text() {
            Ok(b) => b,
            Err(e) => {
                last_error = format!("Failed to read response: {}", e);
                continue;
            }
        };

        // Try to parse as OpenAI-compatible models response
        eprintln!("[Settings] Response body ({} bytes): {}",
            body.len(), &body[..body.len().min(300)]);
        match serde_json::from_str::<ModelsResponse>(&body) {
            Ok(models) => {
                let model_names: Vec<String> = models.data.into_iter().map(|m| m.id).collect();
                eprintln!("[Settings] Found {} models", model_names.len());
                return Ok((model_names.len(), model_names));
            }
            Err(e) => {
                // If we got a 200 but can't parse models, still consider it connected
                eprintln!("[Settings] Could not parse models response: {}", e);
                return Ok((0, vec![]));
            }
        }
    }

    // All endpoints failed
    Err(if last_error.is_empty() {
        "Could not find models endpoint".to_string()
    } else {
        last_error
    })
}
