pub mod a2ui_builder;
pub mod model_runtime_client;
pub mod a2ui_tools;
pub mod chats;
pub mod local_models;
pub mod model_registry;
pub mod moly_client;
pub mod ominix_image_client;
pub mod preferences;
pub mod providers;
pub mod providers_manager;
pub mod store;
pub mod update_checker;

pub use chats::{ChatData, ChatId, Chats};
pub use local_models::{
    // V1 (legacy)
    LocalModel, LocalModelsConfig, ModelCategory, ModelStatus,
    // V2 (new JSON-based system)
    LocalModelV2, LocalModelsConfigV2, ModelState, ModelSource, ModelStorage,
    ModelFileInfo, ModelRuntime, ModelStatusInfo, DownloadProgress, SourceType,
};
pub use moly_client::{MolyClient, ServerConnectionStatus};
pub use ominix_image_client::{OminiXImageClient, ImageGenerationConfig};
pub use preferences::Preferences;
pub use providers::{ProviderPreferences, ProviderId, ProviderType, ProviderConnectionStatus, get_supported_providers};
pub use providers_manager::ProvidersManager;
pub use model_registry::{
    ModelRegistry, RegistryModel, RegistryCategory, RegistrySource, RegistryStorage,
    RegistryRuntime, RegistryUiHints, ApiType, PanelType, SourceKind, ExtraModelSource,
};
pub use model_runtime_client::{ModelRuntimeClient, ServerModelStatus, ServerModelInfo, ensure_server_running, kill_server_process};
pub use store::{Store, StoreAction};
pub use update_checker::{UpdateInfo, check_for_update};

// A2UI (AI-to-UI) exports
pub use a2ui_builder::A2uiBuilder;
pub use a2ui_tools::{get_a2ui_tools_json, is_a2ui_tool, a2ui_tool_names, A2UI_SYSTEM_PROMPT};

// Re-export moly_protocol types used by the models UI
pub use moly_protocol::data::{Model, File as ModelFile, FileId, DownloadedFile, PendingDownload, PendingDownloadsStatus, Author};
