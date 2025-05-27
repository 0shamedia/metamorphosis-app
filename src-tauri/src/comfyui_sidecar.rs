// metamorphosis-app/src-tauri/src/comfyui_sidecar.rs

// Declare the new top-level module for sidecar management.
// The actual implementation details are now within this module's sub-modules.
// pub mod sidecar_manager; // This was incorrect, sidecar_manager is a top-level module

// Re-export the public interface that was previously in this file.
// Other parts of the crate (e.g., lib.rs, setup.rs) will continue to use
// `comfyui_sidecar::function_name` or `comfyui_sidecar::CONSTANT_NAME`.

// Tauri commands are re-exported directly.
// ensure_backend_ready and ensure_comfyui_running_and_healthy were reported as unused re-exports here.
// They are likely called directly from crate::sidecar_manager::orchestration or defined as tauri commands.
// pub use crate::sidecar_manager::orchestration::{
//     ensure_backend_ready, // Marked as unused re-export
//     ensure_comfyui_running_and_healthy, // Marked as unused re-export
// };

// Public functions are re-exported.
pub use crate::sidecar_manager::process_handler::{
    stop_comfyui_sidecar,
};

// Public constants (if any were directly used externally from here).
// COMFYUI_PORT is re-exported from sidecar_manager::event_utils via sidecar_manager::mod.rs
// pub use crate::sidecar_manager::COMFYUI_PORT; // Marked as unused by compiler

// Note: Most `use` statements that were previously at the top of this file
// have been moved into the respective sub-modules within `sidecar_manager`
// (e.g., event_utils.rs, process_handler.rs, health_checker.rs, orchestration.rs)
// where they are directly needed. This keeps the top-level `comfyui_sidecar.rs` clean.