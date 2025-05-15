// metamorphosis-app/src-tauri/src/setup.rs

// This module now delegates its functionality to the setup_manager.
// pub mod setup_manager; // This was incorrect, setup_manager is a top-level module

// Re-export the necessary public commands and functions
// The actual items re-exported will depend on what's made public 
// in setup_manager/mod.rs and its submodules.
// Based on our setup_manager/mod.rs, these are the expected re-exports:
pub use crate::setup_manager::orchestration::{
    get_setup_status_and_initialize,
    start_application_setup,
    retry_application_setup,
};

pub use crate::setup_manager::verification::{
    check_initialization_status,
};

pub use crate::setup_manager::event_utils::{
    emit_setup_progress,
};

// Re-export types if they are directly used by other top-level modules
// (though typically they'd be used within setup_manager or by frontend via commands/events)
pub use crate::setup_manager::types::{
    SetupProgressPayload,
    SetupPhase,
    ModelStatus,
    ModelInfo,
    SetupStatusEvent
};

// Any other top-level items that were in the original setup.rs and are NOT moved 
// to setup_manager would remain here. For now, we assume most/all logic is moved.