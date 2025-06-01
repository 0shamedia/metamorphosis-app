use log::warn;
// metamorphosis-app/src-tauri/src/setup_manager/model_downloader.rs

use tauri::{AppHandle, Wry};
use log::{info, error, debug};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use futures_util::StreamExt;
use std::time::Duration;
use reqwest;
use zip::ZipArchive; // Added for archive extraction

use super::model_config::ModelConfig; // Import ModelConfig
use super::types::ModelType; // Import ModelType
use super::model_events::{
    ModelDownloadProgressPayload,
    ModelDownloadCompletePayload,
    ModelDownloadFailedPayload,
    OverallModelDownloadProgressInternal, // Changed from OverallModelDownloadProgress
    emit_model_download_progress,
    emit_model_download_complete,
    emit_model_download_failed,
}; // Import event payloads and emitters

pub async fn download_single_model(
    app_handle: &AppHandle<Wry>,
    model_config: &ModelConfig,
    target_file_path: &Path, // This is the final destination path
    overall_progress_payload: &mut OverallModelDownloadProgressInternal, // Changed type
    current_attempt: usize,
    max_attempts: usize,
) -> Result<PathBuf, String> {
    info!("Processing model: {} (Attempt {}/{})", model_config.name, current_attempt, max_attempts);
    debug!("Target file path for {}: {}", model_config.name, target_file_path.display());

    let downloaded_filename = model_config.downloaded_filename.as_deref().unwrap_or(&model_config.target_filename);
    let temp_download_path = target_file_path.with_file_name(format!("{}.tmp", downloaded_filename));

    debug!("Temporary download path for {}: {}", model_config.name, temp_download_path.display());

    // --- BEGIN: Check for existing extracted archive contents (specifically for antelopev2) ---
    if model_config.model_type == ModelType::Archive && model_config.id == "instantid_antelopev2_archive" {
        // Expected extraction directory is a subdirectory named "antelopev2"
        // within the directory where the zip file itself would be placed.
        // target_file_path is .../ComfyUI/models/insightface/models/antelopev2.zip
        // So, parent is .../ComfyUI/models/insightface/models/
        // And extraction_dir is .../ComfyUI/models/insightface/models/antelopev2/
        if let Some(archive_parent_dir) = target_file_path.parent() {
            let archive_name_stem = target_file_path.file_stem().unwrap_or_default().to_string_lossy();
            let expected_extraction_dir = archive_parent_dir.join(archive_name_stem.as_ref()); // e.g., .../antelopev2

            debug!("Checking for existing extracted contents of {} in {}", model_config.name, expected_extraction_dir.display());

            if expected_extraction_dir.exists() && expected_extraction_dir.is_dir() {
                // Perform sanity check for key files
                let key_file_1 = expected_extraction_dir.join("glintr100.onnx");
                let key_file_2 = expected_extraction_dir.join("scrfd_10g_bnkps.onnx");

                if key_file_1.exists() && key_file_1.is_file() && key_file_2.exists() && key_file_2.is_file() {
                    info!(
                        "Archive {} contents already exist and seem valid in {}. Skipping download and extraction.",
                        model_config.name,
                        expected_extraction_dir.display()
                    );
                    // Emit complete event as if the archive (zip) was "downloaded"
                    // The size reported here would be for the zip if we had it, or 0 if we don't.
                    // For simplicity, let's use 0 as we are skipping the zip download.
                    // Alternatively, we could try to get the size of the zip if it *also* exists,
                    // but the primary goal is to skip if extracted content is present.
                    emit_model_download_complete(
                        app_handle,
                        ModelDownloadCompletePayload {
                            model_id: model_config.id.clone(),
                            model_name: model_config.name.clone(),
                            file_path: target_file_path.to_path_buf(), // Path of the .zip
                            size_bytes: 0, // Placeholder size, as we skipped download
                        },
                    );
                    overall_progress_payload.current_model_progress_percentage = 100.0;
                    return Ok(target_file_path.to_path_buf()); // Return path to the .zip, as per function signature
                } else {
                    info!(
                        "Extraction directory {} for archive {} exists, but key files are missing. Proceeding with download.",
                        expected_extraction_dir.display(),
                        model_config.name
                    );
                }
            } else {
                info!(
                    "Expected extraction directory {} for archive {} does not exist. Proceeding with download.",
                    expected_extraction_dir.display(),
                    model_config.name
                );
            }
        } else {
            warn!("Could not determine parent directory for archive {}. Cannot check for extracted contents.", model_config.name);
        }
    }
    // --- END: Check for existing extracted archive contents ---

    // Idempotency Check: Check if the final target file (e.g. .zip) already exists
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
                            progress: progress_percentage, // Changed field name
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
            progress: final_progress_percentage, // Changed field name
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

    info!("Successfully downloaded model: {} to {} (Attempt {}/{})", model_config.name, target_file_path.display(), current_attempt, max_attempts);

    // --- Archive Extraction Logic ---
    if model_config.model_type == ModelType::Archive {
        // The target_file_path for an archive is where the .zip is initially downloaded.
        // The actual contents need to go into a subdirectory, often named after the archive itself (without .zip).
        let archive_path = target_file_path; // Path to the downloaded .zip file

        // Determine extraction directory: ComfyUI/models/insightface/models/antelopev2/
        // model_config.target_subdir for antelopev2.zip is "models/insightface/models/"
        // The final part "antelopev2" comes from the archive name or a specific requirement.
        // For antelopev2.zip, the contents should go into a folder named "antelopev2"
        // inside the `target_subdir` of the *archive file itself*.

        // Let's construct the extraction path more carefully.
        // The `target_file_path` for the ModelConfig of the .zip is where the .zip lands.
        // e.g., ComfyUI/models/insightface/models/antelopev2.zip
        // The contents should go into ComfyUI/models/insightface/models/antelopev2/
        let extraction_base_dir = archive_path.parent().ok_or_else(|| {
            format!("Could not get parent directory for archive: {}", archive_path.display())
        })?;
        let archive_name_stem = archive_path.file_stem().ok_or_else(|| {
            format!("Could not get file stem for archive: {}", archive_path.display())
        })?.to_string_lossy().to_string();

        // This is the crucial part: the extraction target for antelopev2 contents.
        let final_extraction_path = extraction_base_dir.join(&archive_name_stem); // e.g., .../models/insightface/models/antelopev2

        info!("Extracting archive {} to {}", archive_path.display(), final_extraction_path.display());

        match extract_archive(app_handle, archive_path, &final_extraction_path, model_config) {
            Ok(_) => {
                info!("Successfully extracted archive {} to {}", model_config.name, final_extraction_path.display());
                // Optionally, delete the archive file after successful extraction
                if let Err(e) = fs::remove_file(archive_path) {
                    error!("Failed to delete archive file {} after extraction: {}", archive_path.display(), e);
                    // Not a fatal error for the download process itself, but log it.
                } else {
                    info!("Successfully deleted archive file {} after extraction.", archive_path.display());
                }
            }
            Err(e) => {
                let err_msg = format!("Failed to extract archive {} (Attempt {}/{}): {}", model_config.name, current_attempt, max_attempts, e);
                error!("{}", err_msg);
                emit_model_download_failed(app_handle, ModelDownloadFailedPayload {
                    model_id: model_config.id.clone(),
                    model_name: model_config.name.clone(),
                    error_message: err_msg.clone(),
                });
                // Attempt to clean up the downloaded archive if extraction fails
                fs::remove_file(archive_path).ok();
                return Err(err_msg);
            }
        }
    }
    // --- End Archive Extraction Logic ---

    info!("Successfully processed model: {} at {} (Attempt {}/{})", model_config.name, target_file_path.display(), current_attempt, max_attempts);
    emit_model_download_complete(
        app_handle,
        ModelDownloadCompletePayload {
            model_id: model_config.id.clone(),
            model_name: model_config.name.clone(),
            file_path: target_file_path.to_path_buf(), // For archives, this is the path of the .zip before deletion
            size_bytes: file_size,
        },
    );
    overall_progress_payload.current_model_progress_percentage = 100.0;

    Ok(target_file_path.to_path_buf())
}

// --- Archive Extraction Function ---
fn extract_archive(
    _app_handle: &AppHandle<Wry>, // For emitting progress if needed, though currently not implemented for extraction sub-steps
    archive_path: &Path,
    extraction_target_dir: &Path, // The directory where contents should go (e.g., .../antelopev2/)
    model_config: &ModelConfig, // For logging and potentially emitting events
) -> Result<(), String> {
    info!("[EXTRACT] Starting extraction for model: {}, archive: {}, target: {}", model_config.name, archive_path.display(), extraction_target_dir.display());

    let file = File::open(archive_path)
        .map_err(|e| format!("[EXTRACT] Failed to open archive file {}: {}", archive_path.display(), e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("[EXTRACT] Failed to read archive {}: {}", archive_path.display(), e))?;

    if !extraction_target_dir.exists() {
        fs::create_dir_all(extraction_target_dir)
            .map_err(|e| format!("[EXTRACT] Failed to create extraction directory {}: {}", extraction_target_dir.display(), e))?;
        info!("[EXTRACT] Created extraction directory: {}", extraction_target_dir.display());
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("[EXTRACT] Error accessing file at index {} in archive {}: {}", i, archive_path.display(), e))?;

        let outpath_within_archive = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => {
                info!("[EXTRACT] Entry {} has a suspicious path (cannot be safely extracted), skipping.", file.name());
                continue;
            }
        };
        debug!("[EXTRACT] Archive entry raw name: '{}', Enclosed name for processing: '{}'", file.name(), outpath_within_archive.display());

        // --- Logic to strip a leading directory component if it matches "antelopev2" for the specific archive ---
        // This ensures that if antelopev2.zip contains "antelopev2/model.onnx", it's extracted as "model.onnx"
        // into `extraction_target_dir`. If it contains just "antelopev2/", the resulting path component is empty.
        let mut final_outpath_component = outpath_within_archive.clone(); // Start with the original archive path

        if model_config.id == "instantid_antelopev2_archive" {
            if let Some(first_comp) = outpath_within_archive.components().next() {
                if first_comp.as_os_str() == "antelopev2" {
                    // Strip the first component ("antelopev2")
                    final_outpath_component = outpath_within_archive.components().skip(1).collect::<PathBuf>();
                    debug!("[EXTRACT] Stripped 'antelopev2' prefix. Original archive path: '{}', Resulting component for target: '{}'", outpath_within_archive.display(), final_outpath_component.display());
                } else {
                    debug!("[EXTRACT] First component of archive path '{}' is not 'antelopev2'. No stripping.", outpath_within_archive.display());
                }
            } else {
                debug!("[EXTRACT] Archive path '{}' has no components. No stripping.", outpath_within_archive.display());
            }
        } else {
            // For other archives, or if conditions not met, use the original path from archive.
            debug!("[EXTRACT] Not the 'instantid_antelopev2_archive' or conditions not met for stripping. Using archive path component as is: '{}'", final_outpath_component.display());
        }
        // --- End of stripping logic ---

        let full_outpath = extraction_target_dir.join(&final_outpath_component);
        debug!("[EXTRACT] Calculated final disk output path: '{}' (extraction_target_dir: '{}', final_outpath_component: '{}')", full_outpath.display(), extraction_target_dir.display(), final_outpath_component.display());


        if file.name().ends_with('/') { // Check original name for directory marker
            // It's a directory entry from the archive.
            // If final_outpath_component is empty, it means the archive directory (after stripping)
            // maps directly to the extraction_target_dir. In this case, we don't need to create it again,
            // as extraction_target_dir is ensured to exist by line 408.
            if final_outpath_component.as_os_str().is_empty() {
                 debug!("[EXTRACT] Archive directory entry '{}' resolves to the extraction target directory itself after stripping. Skipping explicit creation of '{}'.", file.name(), full_outpath.display());
            } else if !full_outpath.exists() {
                // This is for creating subdirectories *within* the extraction_target_dir
                debug!("[EXTRACT] Attempting to create directory (from archive entry {}): '{}'", file.name(), full_outpath.display());
                fs::create_dir_all(&full_outpath)
                    .map_err(|e| format!("[EXTRACT] Failed to create directory '{}': {}", full_outpath.display(), e))?;
                info!("[EXTRACT] Successfully created directory '{}'", full_outpath.display());
            } else {
                debug!("[EXTRACT] Directory '{}' (from archive entry {}) already exists.", full_outpath.display(), file.name());
            }
        } else {
            // It's a file
            if let Some(p) = full_outpath.parent() {
                if !p.exists() {
                    debug!("[EXTRACT] Parent directory '{}' for file '{}' does not exist. Attempting creation.", p.display(), full_outpath.display());
                    fs::create_dir_all(p)
                        .map_err(|e| format!("[EXTRACT] Failed to create parent directory '{}': {}", p.display(), e))?;
                    info!("[EXTRACT] Successfully created parent directory '{}'", p.display());
                } else {
                    debug!("[EXTRACT] Parent directory '{}' for file '{}' already exists.", p.display(), full_outpath.display());
                }
            }
            debug!("[EXTRACT] Attempting to create/truncate output file for archive entry '{}' at '{}'", file.name(), full_outpath.display());
            let mut outfile = File::create(&full_outpath)
                .map_err(|e| format!("[EXTRACT] Failed to create output file '{}': {}", full_outpath.display(), e))?;
            
            debug!("[EXTRACT] Attempting to copy data from archive entry '{}' to '{}'", file.name(), full_outpath.display());
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("[EXTRACT] Failed to copy content from archive entry '{}' to '{}': {}", file.name(), full_outpath.display(), e))?;
            info!("[EXTRACT] Successfully copied data from archive entry '{}' to '{}'", file.name(), full_outpath.display());
        }

        // Set permissions if on Unix-like system
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&full_outpath, fs::Permissions::from_mode(mode))
                    .map_err(|e| format!("[EXTRACT] Failed to set permissions for {}: {}", full_outpath.display(), e))?;
            }
        }
    }
    info!("[EXTRACT] Successfully extracted all files from archive: {} to {}", archive_path.display(), extraction_target_dir.display());
    Ok(())
}