// metamorphosis-app/src-tauri/src/setup_manager/model_downloader.rs

use tauri::{AppHandle, Wry};
use log::{info, error, debug};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use futures_util::StreamExt;
use std::time::Duration;
use reqwest;

use super::model_config::ModelConfig; // Import ModelConfig
use super::model_events::{
    ModelDownloadProgressPayload,
    ModelDownloadCompletePayload,
    ModelDownloadFailedPayload,
    OverallModelDownloadProgress,
    emit_model_download_progress,
    emit_model_download_complete,
    emit_model_download_failed,
}; // Import event payloads and emitters

pub async fn download_single_model(
    app_handle: &AppHandle<Wry>,
    model_config: &ModelConfig,
    target_file_path: &Path, // This is the final destination path
    overall_progress_payload: &mut OverallModelDownloadProgress,
    current_attempt: usize,
    max_attempts: usize,
) -> Result<PathBuf, String> {
    info!("Processing model: {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);
    debug!("Target file path for {}: {}", model_config.name, target_file_path.display());

    let downloaded_filename = model_config.downloaded_filename.as_deref().unwrap_or(&model_config.target_filename);
    let temp_download_path = target_file_path.with_file_name(format!("{}.tmp", downloaded_filename));

    debug!("Temporary download path for {}: {}", model_config.name, temp_download_path.display());

    // Idempotency Check: Check if the final target file already exists
    if target_file_path.exists() {
        let metadata = fs::metadata(target_file_path)
            .map_err(|e| format!("Failed to get metadata for existing file {}: {}", target_file_path.display(), e))?;
        if metadata.len() > 0 {
            if let Some(expected_size) = model_config.expected_size_bytes {
                if metadata.len() == expected_size {
                    info!("Model {} already exists at {} with correct size. Skipping download.", model_config.name, target_file_path.display());
                    emit_model_download_complete(
                        app_handle,
                        ModelDownloadCompletePayload {
                            model_id: model_config.id.clone(),
                            model_name: model_config.name.clone(),
                            file_path: target_file_path.to_path_buf(),
                            size_bytes: metadata.len(),
                        },
                    );
                    overall_progress_payload.current_model_progress_percentage = 100.0;
                    // Note: Overall progress will be updated in the calling function
                    return Ok(target_file_path.to_path_buf());
                } else {
                    info!(
                        "Model {} exists at {} but size ({} bytes) differs from expected ({} bytes). Re-downloading.",
                        model_config.name,
                        target_file_path.display(),
                        metadata.len(),
                        expected_size
                    );
                    // Optionally, delete the existing file before re-downloading
                    fs::remove_file(target_file_path).map_err(|e| format!("Failed to remove existing file {}: {}", target_file_path.display(), e))?;
                }
            } else {
                // No expected size, assume existing file is fine if it's not empty
                info!("Model {} already exists at {} and is not empty. Skipping download.", model_config.name, target_file_path.display());
                 emit_model_download_complete(
                    app_handle,
                    ModelDownloadCompletePayload {
                        model_id: model_config.id.clone(),
                        model_name: model_config.name.clone(),
                        file_path: target_file_path.to_path_buf(),
                        size_bytes: metadata.len(),
                    },
                );
                overall_progress_payload.current_model_progress_percentage = 100.0;
                return Ok(target_file_path.to_path_buf());
            }
        } else {
            info!("Model {} exists at {} but is empty. Re-downloading.", model_config.name, target_file_path.display());
            fs::remove_file(target_file_path).map_err(|e| format!("Failed to remove existing empty file {}: {}", target_file_path.display(), e))?;
        }
    }

    // Ensure parent directory for temp file exists
    if let Some(parent_dir) = temp_download_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)
                .map_err(|e| format!("Failed to create temporary directory {}: {}", parent_dir.display(), e))?;
        }
    }


    // This log was moved to the retry loop in download_and_place_models
    // info!("Starting download for model: {} from URL: {}", model_config.name, model_config.url);

    let client = reqwest::Client::builder()
        .user_agent("MetamorphosisApp/1.0")
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60 * 30)) // Overall request timeout (30 minutes)
        .build()
        .map_err(|e| format!("Failed to build reqwest client: {}", e))?;

    let response = client.get(&model_config.url).send().await.map_err(|e| {
        let err_msg = format!(
            "Failed to send request for model {} (Attempt {}/{}): {}",
            model_config.name, current_attempt, max_attempts, e
        );
        error!("{}", err_msg); // Log the specific send error
        err_msg
    })?;

    if !response.status().is_success() {
        let err_msg = format!(
            "Download failed for model {} (Attempt {}/{}): HTTP Status {}",
            model_config.name, current_attempt, max_attempts, response.status()
        );
        error!("{}", err_msg);
        emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
            model_id: model_config.id.clone(),
            model_name: model_config.name.clone(),
            error_message: err_msg.clone(),
        });
        return Err(err_msg);
    }

    let total_size = response.content_length();
    debug!("Total size for {}: {:?}", model_config.name, total_size);
    let mut downloaded_size: u64 = 0;
    let mut stream = response.bytes_stream();

    debug!("Creating temporary file for {} at {}", model_config.name, temp_download_path.display());
    let mut temp_file = File::create(&temp_download_path).map_err(|e| {
        let err_msg = format!(
            "Failed to create temporary file {} for model {} (Attempt {}/{}): {}",
            temp_download_path.display(), model_config.name, current_attempt, max_attempts, e
        );
        error!("{}", err_msg);
        err_msg
    })?;

    debug!("Starting stream processing for model {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);
    let mut last_progress_emit_time = std::time::Instant::now(); // For rate limiting progress events
    let progress_emit_interval = Duration::from_millis(250); // Emit progress at most every 250ms

    while let Some(item_result) = stream.next().await {
        match item_result {
            Ok(chunk) => {
                if let Err(e) = temp_file.write_all(&chunk) {
                    let err_msg = format!(
                        "Error writing chunk to temporary file for model {} (Attempt {}/{}): {}. Chunk size: {}",
                        model_config.name, current_attempt, max_attempts, e, chunk.len()
                    );
                    error!("{}", err_msg);
                    // Attempt to remove partial temporary file before returning error
                    fs::remove_file(&temp_download_path).ok();
                    emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
                        model_id: model_config.id.clone(),
                        model_name: model_config.name.clone(),
                        error_message: err_msg.clone(),
                    });
                    return Err(err_msg);
                }
                downloaded_size += chunk.len() as u64;

                // Calculate and emit progress inside the loop
                let progress_percentage = if let Some(total) = total_size {
                    if total > 0 { (downloaded_size as f32 / total as f32) * 100.0 } else { 0.0 }
                } else {
                    0.0 // Indeterminate progress if total size is unknown
                };
                overall_progress_payload.current_model_progress_percentage = progress_percentage;

                // Emit progress event, rate-limited
                let now = std::time::Instant::now();
                if now.duration_since(last_progress_emit_time) > progress_emit_interval {
                    let progress_percentage = if let Some(total) = total_size {
                        if total > 0 { (downloaded_size as f32 / total as f32) * 100.0 } else { 0.0 }
                    } else {
                        0.0 // Indeterminate progress if total size is unknown
                    };
                    overall_progress_payload.current_model_progress_percentage = progress_percentage;

                    emit_model_download_progress(
                        app_handle,
                        ModelDownloadProgressPayload {
                            model_id: model_config.id.clone(),
                            model_name: model_config.name.clone(),
                            downloaded_bytes: downloaded_size,
                            total_bytes: total_size,
                            progress_percentage,
                        },
                    );
                    last_progress_emit_time = now;
                }
            }
            Err(e) => {
                // This is a critical point: error while fetching a chunk from the stream
                let err_msg = format!(
                    "Error while downloading/decoding chunk for model {} (Attempt {}/{}): {}. Downloaded so far: {} bytes.",
                    model_config.name, current_attempt, max_attempts, e, downloaded_size
                );
                error!("{}", err_msg);
                // Attempt to remove partial temporary file before returning error
                fs::remove_file(&temp_download_path).ok();
                emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
                    model_id: model_config.id.clone(),
                    model_name: model_config.name.clone(),
                    error_message: err_msg.clone(),
                });
                return Err(err_msg); // This error will trigger a retry in the calling function
            }
        }
    }
    debug!("Stream processing finished for model {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts); // Log after the loop
    // Ensure final progress is emitted after the loop finishes
    let final_progress_percentage = if let Some(total) = total_size {
        if total > 0 { (downloaded_size as f32 / total as f32) * 100.0 } else { 0.0 }
    } else {
        0.0
    };
    overall_progress_payload.current_model_progress_percentage = final_progress_percentage;
    emit_model_download_progress(
        app_handle,
        ModelDownloadProgressPayload {
            model_id: model_config.id.clone(),
            model_name: model_config.name.clone(),
            downloaded_bytes: downloaded_size,
            total_bytes: total_size,
            progress_percentage: final_progress_percentage,
        },
    );

    // Ensure all data is written to disk
    debug!("Syncing temporary file for model {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);
    if let Err(e) = temp_file.sync_all() {
        let err_msg = format!(
            "Failed to sync temporary file {} for model {} (Attempt {}/{}): {}",
            temp_download_path.display(), model_config.name, current_attempt, max_attempts, e
        );
        error!("{}", err_msg);
        fs::remove_file(&temp_download_path).ok();
        emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
            model_id: model_config.id.clone(),
            model_name: model_config.name.clone(),
            error_message: err_msg.clone(),
        });
        return Err(err_msg);
    }
    drop(temp_file); // Close the file before renaming
    debug!("Temporary file synced and closed for model {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);

    // File Integrity Check
    debug!("Performing file integrity check for model {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);
    let metadata = fs::metadata(&temp_download_path).map_err(|e| {
        let err_msg = format!(
            "Failed to get metadata for temporary file {} for model {} (Attempt {}/{}): {}",
            temp_download_path.display(), model_config.name, current_attempt, max_attempts, e
        );
        error!("{}", err_msg);
        err_msg // Don't emit ModelDownloadFailed here, as the file might not even exist for metadata.
    })?;
    let file_size = metadata.len();

    if file_size == 0 {
        let err_msg = format!(
            "Downloaded file for model {} (Attempt {}/{}) is empty. Path: {}",
            model_config.name, current_attempt, max_attempts, temp_download_path.display()
        );
        error!("{}", err_msg);
        fs::remove_file(&temp_download_path).ok(); // Attempt to clean up
        emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
            model_id: model_config.id.clone(),
            model_name: model_config.name.clone(),
            error_message: err_msg.clone(),
        });
        return Err(err_msg);
    }

    if let Some(expected) = model_config.expected_size_bytes {
        if file_size != expected {
            let err_msg = format!(
                "Downloaded file size for model {} (Attempt {}/{}) is {} bytes, but expected {} bytes. Path: {}",
                model_config.name, current_attempt, max_attempts, file_size, expected, temp_download_path.display()
            );
            error!("{}", err_msg);
            fs::remove_file(&temp_download_path).ok(); // Attempt to clean up
            emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
                model_id: model_config.id.clone(),
                model_name: model_config.name.clone(),
                error_message: err_msg.clone(),
            });
            return Err(err_msg);
        }
        debug!("File size matches expected size for model {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);
    } else {
        debug!("No expected size for model {}, downloaded size: {} bytes (Attempt {}/{})", model_config.name, file_size, current_attempt, max_attempts);
    }

    debug!("Renaming temporary file {} to {} for model {} (Attempt {}/{})", temp_download_path.display(), target_file_path.display(), model_config.name, current_attempt, max_attempts);
    fs::rename(&temp_download_path, target_file_path).map_err(|e| {
        let err_msg = format!(
            "Failed to rename temporary file {} to {} for model {} (Attempt {}/{}): {}",
            temp_download_path.display(), target_file_path.display(), model_config.name, current_attempt, max_attempts, e
        );
        error!("{}", err_msg);
        // Don't emit ModelDownloadFailed here as the core download succeeded, this is a post-processing step.
        // The calling function will handle this as a failure of download_single_model.
        err_msg
    })?;

    info!("Successfully downloaded and placed model: {} at {} (Attempt {}/{})", model_config.name, target_file_path.display(), current_attempt, max_attempts);
    emit_model_download_complete(
        app_handle,
        ModelDownloadCompletePayload {
            model_id: model_config.id.clone(),
            model_name: model_config.name.clone(),
            file_path: target_file_path.to_path_buf(),
            size_bytes: file_size,
        },
    );
    overall_progress_payload.current_model_progress_percentage = 100.0;

    Ok(target_file_path.to_path_buf())
}