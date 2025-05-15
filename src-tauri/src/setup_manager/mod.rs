// metamorphosis-app/src-tauri/src/setup_manager/mod.rs

pub mod event_utils;
pub mod verification;
pub mod orchestration;
pub mod types;

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
    SetupStatusEvent
};