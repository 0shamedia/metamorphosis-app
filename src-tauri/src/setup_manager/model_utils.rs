// metamorphosis-app/src-tauri/src/setup_manager/model_utils.rs

use std::path::{Path, PathBuf};
use std::fs;
use log::debug;

use super::model_config::ModelConfig; // Import ModelConfig from the new module

// Renamed from get_comfyui_model_destination_path for clarity
pub fn get_final_model_path(
    comfyui_models_base_path: &Path,
    model_config: &ModelConfig,
) -> Result<PathBuf, String> {
    let final_path = comfyui_models_base_path
        .join(&model_config.target_subdir)
        .join(&model_config.target_filename);

    // Ensure parent directory for the *final* path exists
    // This was previously in download_single_model for the temp path,
    // but it's better to ensure the final destination's parent exists here.
    if let Some(parent_dir) = final_path.parent() {
        if !parent_dir.exists() {
            debug!("Creating parent directory for final model path: {}", parent_dir.display());
            fs::create_dir_all(parent_dir)
                .map_err(|e| format!("Failed to create directory {}: {}", parent_dir.display(), e))?;
        }
    }
    Ok(final_path)
}