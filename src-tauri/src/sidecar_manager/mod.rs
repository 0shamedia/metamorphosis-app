// metamorphosis-app/src-tauri/src/sidecar_manager/mod.rs

pub mod event_utils;
pub mod process_handler;
pub mod health_checker;
pub mod orchestration;

// Re-export items that need to be public from the sidecar_manager module.
// These will then be re-exported by the parent `comfyui_sidecar.rs` if needed
// for use by other parts of the crate like `lib.rs` or `setup.rs`.
pub use orchestration::{
    ensure_backend_ready,
    ensure_comfyui_running_and_healthy,
    spawn_and_health_check_comfyui,
};
pub use process_handler::{
    is_comfyui_process_active,
    stop_comfyui_sidecar,
};

// Constants like COMFYUI_PORT might be pub from event_utils if needed externally,
// or just used internally by the modules within sidecar_manager.
pub use event_utils::COMFYUI_PORT;