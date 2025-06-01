// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/disk_utils.rs

// Estimated required disk space for ComfyUI dependencies (20 GB)
pub(super) const REQUIRED_DISK_SPACE: u64 = 20 * 1024 * 1024 * 1024; // in bytes

// The actual function `available_space` comes from the `fs2` crate and is used directly
// in `python_env.rs`. If more complex disk utility functions were needed, they would go here.