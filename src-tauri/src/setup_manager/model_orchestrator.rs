// metamorphosis-app/src-tauri/src/setup_manager/model_orchestrator.rs

use tauri::{AppHandle, Wry};
use log::{info, error, debug};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

use super::model_config::ModelConfig; // Import ModelConfig
use super::model_events::{
    OverallModelDownloadProgress,
    emit_overall_model_download_progress,
}; // Import event payloads and emitters
use super::model_utils::get_final_model_path; // Import get_final_model_path
use super::model_downloader::download_single_model; // Import download_single_model

// --- Main Orchestration Function ---

pub async fn download_and_place_models(
    app_handle: AppHandle<Wry>,
    models_to_download: &[ModelConfig], // Changed to slice
    comfyui_models_base_path: &Path,  // Changed to reference
) -> Result<(), String> {
    info!("Starting download and placement of {} models.", models_to_download.len());
    let total_models = models_to_download.len();
    if total_models == 0 {
        info!("No models configured for download.");
        return Ok(());
    }

    let mut overall_progress_payload = OverallModelDownloadProgress {
        current_model_index: 0,
        total_models,
        current_model_id: "".to_string(),
        current_model_name: "".to_string(),
        current_model_progress_percentage: 0.0,
        overall_progress_percentage: 0.0,
    };

    for (index, model_config) in models_to_download.iter().enumerate() {
        overall_progress_payload.current_model_index = index;
        overall_progress_payload.current_model_id = model_config.id.clone();
        overall_progress_payload.current_model_name = model_config.name.clone();
        overall_progress_payload.current_model_progress_percentage = 0.0;

        // Calculate overall progress before starting the current model download
        // This ensures the UI shows which model is *about* to be downloaded
        overall_progress_payload.overall_progress_percentage = (index as f32 / total_models as f32) * 100.0;
        emit_overall_model_download_progress(&app_handle, overall_progress_payload.clone());

        let target_file_path = get_final_model_path(comfyui_models_base_path, model_config)?;
        debug!("Determined target path for {}: {}", model_config.name, target_file_path.display());

        let max_retries = 3;
        let mut attempt = 0;
        let mut last_error_message: Option<String> = None;

        while attempt < max_retries {
            attempt += 1;
            if attempt > 1 {
                let backoff_duration_secs = std::cmp::min(5 * (attempt -1) , 30) as u64; // Calculate backoff based on previous attempt
                info!(
                    "Retrying download for model {} (attempt {}/{}), waiting for {} seconds...",
                    model_config.name, attempt, max_retries, backoff_duration_secs
                );
                // Reset current model's progress for the UI before retrying
                overall_progress_payload.current_model_progress_percentage = 0.0;
                // Emit overall progress to signal the UI that a retry is starting for this model
                // Ensure overall_progress_percentage reflects the start of this model, not completion of previous
                overall_progress_payload.overall_progress_percentage = (index as f32 / total_models as f32) * 100.0;
                emit_overall_model_download_progress(&app_handle, overall_progress_payload.clone());

                debug!("Starting backoff for {}s for model {}", backoff_duration_secs, model_config.name);
                sleep(Duration::from_secs(backoff_duration_secs)).await; // Exponential backoff with a cap
                debug!("Backoff finished for model {}", model_config.name);
            } else {
                info!("Starting download for model {} (attempt {}/{})", model_config.name, attempt, max_retries);
            }

            match download_single_model(&app_handle, model_config, &target_file_path, &mut overall_progress_payload, attempt, max_retries).await {
                Ok(_) => {
                    info!("Successfully processed model: {}", model_config.name);
                    // overall_progress_payload.current_model_progress_percentage is set to 100.0 by download_single_model on success
                    overall_progress_payload.overall_progress_percentage = ((index + 1) as f32 / total_models as f32) * 100.0;
                    emit_overall_model_download_progress(&app_handle, overall_progress_payload.clone());
                    last_error_message = None; // Clear error on success
                    break; // Exit retry loop
                }
                Err(e) => {
                    error!("Attempt {}/{} failed for model {}: {}", attempt, max_retries, model_config.name, e);
                    last_error_message = Some(e.clone());
                    // ModelDownloadFailed event is emitted by download_single_model itself.
                }
            }
        }

        if let Some(err_msg) = last_error_message {
            error!("All {} attempts failed for model {}. Last error: {}", max_retries, model_config.name, err_msg);
            // The specific model download failure event was already emitted by the last call to download_single_model.
            return Err(format!("Failed to download model {} after {} attempts: {}", model_config.name, max_retries, err_msg));
        }
    }

    info!("All models processed successfully.");
    if !models_to_download.is_empty() {
        // Ensure the UI shows 100% completion for the last model and overall.
        overall_progress_payload.current_model_index = total_models.saturating_sub(1);
        if let Some(last_model) = models_to_download.last() {
            overall_progress_payload.current_model_id = last_model.id.clone();
            overall_progress_payload.current_model_name = last_model.name.clone();
        }
        overall_progress_payload.current_model_progress_percentage = 100.0;
        overall_progress_payload.overall_progress_percentage = 100.0;
        emit_overall_model_download_progress(&app_handle, overall_progress_payload);
    }

    Ok(())
}