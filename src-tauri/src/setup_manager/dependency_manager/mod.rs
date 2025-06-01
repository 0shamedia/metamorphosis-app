// metamorphosis-app/src-tauri/src/setup_manager/dependency_manager/mod.rs
pub mod command_runner;
pub mod disk_utils;
pub mod python_env;

// Re-export the public API that was previously in the old dependency_management.rs
pub use self::python_env::{
    install_python_dependencies_with_progress,
    install_custom_node_dependencies,
};

// The function `run_command_for_setup_progress` from command_runner.rs
// and any functions from disk_utils.rs are intended to be internal helpers
// for this `dependency_manager` module, primarily used by `python_env.rs`.
// If they were needed externally, they would be made `pub` in their respective
// files and re-exported here.