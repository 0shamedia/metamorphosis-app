// metamorphosis-app/src-tauri/src/setup_manager/mod.rs

pub mod event_utils;
pub mod verification;
pub mod orchestration;
pub mod types;
pub mod model_management;
pub mod model_config;
pub mod model_events;
pub mod model_utils;
pub mod model_downloader;
pub mod model_orchestrator;
pub mod custom_node_manager;
pub mod python_utils;
pub mod dependency_manager; // Added dependency_manager module

// Re-export key public functions and commands
pub use orchestration::{
    get_setup_status_and_initialize,
    start_application_setup,
    retry_application_setup,
};

pub use verification::{
    check_initialization_status,
    // Any other verification functions made public
};

pub use event_utils::{
    emit_setup_progress,
    // Types are now re-exported from types.rs below
};

pub use types::{
    SetupProgressPayload,
    SetupPhase,
    ModelStatus,
    ModelInfo,
    SetupStatusEvent,
    // Re-export new custom node payloads if they are intended for wider use,
    // otherwise they are used internally by custom_node_management and its callers.
    // CustomNodePayload,
    // CustomNodeCloneFailedPayload,
};

// Re-export from model_management

pub use model_orchestrator::{
    download_and_place_models,
};

pub use model_events::{
    ModelDownloadProgressPayload,
    ModelDownloadCompletePayload,
    ModelDownloadFailedPayload,
    OverallModelDownloadProgressInternal, // Changed from OverallModelDownloadProgress
    emit_model_download_progress,
    emit_model_download_complete,
    emit_model_download_failed,
    emit_overall_model_download_progress,
};

pub use model_utils::{
    get_final_model_path,
};

pub use model_config::{
    ModelConfig,
    get_core_models_list,
};

// Re-export from custom_node_manager
pub use custom_node_manager::{
    clone_comfyui_impact_pack,
    clone_comfyui_impact_subpack,
    clone_comfyui_smz_nodes,
    clone_repository_to_custom_nodes,
    install_custom_node_dependencies,
};

// Re-export from dependency_manager
pub use dependency_manager::{
    install_python_dependencies_with_progress,
};