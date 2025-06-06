// metamorphosis-app/src-tauri/src/setup_manager/custom_node_manager/mod.rs

pub mod node_definitions;
pub mod cloning;
pub mod installation;

// Re-export public functions from sub-modules
pub use cloning::{
    clone_comfyui_impact_pack,
    clone_comfyui_impact_subpack,
    clone_comfyui_smz_nodes,
    clone_rgthree_comfy_nodes,
    clone_repository_to_custom_nodes, // Re-export this as it's used by orchestration
};
pub use installation::{
    install_custom_node_dependencies, // Re-export this as it's used by clipseg_handler
};