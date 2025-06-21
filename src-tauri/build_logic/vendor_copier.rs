use std::{
    fs,
    io,
    path::{Path, PathBuf},
    error::Error,
};
use fs_extra::dir::{copy as fs_extra_copy, CopyOptions as FsExtraCopyOptions};

// Helper function for manual recursive copy using std::fs
fn copy_recursively_std(source: &Path, destination: &Path) -> io::Result<()> {
    eprintln!("cargo:warning=VENDOR_COPIER_STD: Recursively copying (std::fs) from {:?} to {:?}", source, destination);
    if !destination.exists() {
        fs::create_dir_all(destination).map_err(|e| {
            eprintln!("cargo:warning=VENDOR_COPIER_STD_ERROR: Failed to create destination directory {:?}: {}", destination, e);
            e
        })?;
        eprintln!("cargo:warning=VENDOR_COPIER_STD: Created destination directory: {:?}", destination);
    }
    for entry_result in fs::read_dir(source)? {
        let entry = entry_result?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if file_type.is_dir() {
            eprintln!("cargo:warning=VENDOR_COPIER_STD: Recursing into directory: {:?}", source_path);
            copy_recursively_std(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            eprintln!("cargo:warning=VENDOR_COPIER_STD: Attempting to copy file: {:?} to {:?}", source_path, destination_path);
            fs::copy(&source_path, &destination_path).map_err(|e| {
                eprintln!("cargo:warning=VENDOR_COPIER_STD_ERROR: Failed to copy file from {:?} to {:?}: {}", source_path, destination_path, e);
                e
            })?;
            eprintln!("cargo:warning=VENDOR_COPIER_STD: Successfully copied file: {:?}", destination_path);
        } else {
            eprintln!("cargo:warning=VENDOR_COPIER_STD: Skipping non-file/non-dir: {:?}", source_path);
        }
    }
    Ok(())
}


/// Copies the *contents* of a source directory into a target directory.
///
/// Args:
///   source_dir_contents: The source directory whose contents will be copied.
///   target_dir_base: The destination directory where the contents will be placed.
///                    This directory will be created if it doesn't exist.
///   use_std_fs: If true, forces the use of `std::fs` for copying. Otherwise, tries `fs_extra` first.
fn copy_directory_contents(
    source_dir_contents: &Path,
    target_dir_base: &Path, // This is where the contents of source_dir_contents will go
    subdir_name_for_logs: &str, // e.g., "ComfyUI" or "Python"
    use_std_fs: bool,
) -> Result<(), Box<dyn Error>> {
    eprintln!(
        "cargo:warning=VENDOR_COPIER: [{}] Starting to copy contents of {:?} into {:?}",
        subdir_name_for_logs, source_dir_contents, target_dir_base
    );

    if !source_dir_contents.is_dir() {
        return Err(format!(
            "Source for content copy is not a directory: {:?}",
            source_dir_contents
        ).into());
    }

    // Ensure the target base directory exists (this is where contents will be copied INTO)
    if !target_dir_base.exists() {
        eprintln!("cargo:warning=VENDOR_COPIER: [{}] Creating target base directory for contents: {:?}", subdir_name_for_logs, target_dir_base);
        fs::create_dir_all(target_dir_base)?;
    }

    if use_std_fs {
        eprintln!("cargo:warning=VENDOR_COPIER: [{}] Forcing use of std::fs for copying contents.", subdir_name_for_logs);
        copy_recursively_std(source_dir_contents, target_dir_base)
            .map_err(|e| format!("[{}] std::fs copy failed for {:?}: {}", subdir_name_for_logs, source_dir_contents, e))?;
        eprintln!("cargo:warning=VENDOR_COPIER: [{}] std::fs copy successful.", subdir_name_for_logs);
    } else {
        eprintln!("cargo:warning=VENDOR_COPIER: [{}] Attempting fs_extra::dir::copy for contents.", subdir_name_for_logs);
        let mut copy_options = FsExtraCopyOptions::new();
        copy_options.overwrite = true;
        copy_options.copy_inside = true; // Copy contents of source into destination
        eprintln!("cargo:warning=VENDOR_COPIER: [{}] fs_extra options: overwrite={}, copy_inside={}", subdir_name_for_logs, copy_options.overwrite, copy_options.copy_inside);

        if let Err(e) = fs_extra_copy(source_dir_contents, target_dir_base, &copy_options) {
            eprintln!(
                "cargo:warning=VENDOR_COPIER_ERROR: [{}] fs_extra::dir::copy FAILED. Error: {}. Falling back to std::fs.",
                subdir_name_for_logs, e
            );
            // Fallback to std::fs copy
            copy_recursively_std(source_dir_contents, target_dir_base).map_err(|e_std| {
                format!(
                    "[{}] Fallback std::fs copy also failed: {}",
                    subdir_name_for_logs, e_std
                )
            })?;
            eprintln!("cargo:warning=VENDOR_COPIER: [{}] Fallback std::fs copy successful.", subdir_name_for_logs);
        } else {
            eprintln!("cargo:warning=VENDOR_COPIER: [{}] fs_extra::dir::copy reported successful. Performing quick verification.", subdir_name_for_logs);
            if !target_dir_base.exists() || !target_dir_base.is_dir() {
                eprintln!("cargo:warning=VENDOR_COPIER_ERROR: [{}] fs_extra reported success, but target_dir_base {:?} does not exist or is not a directory. Forcing fallback to std::fs.", subdir_name_for_logs, target_dir_base);
                copy_recursively_std(source_dir_contents, target_dir_base).map_err(|e_std| {
                    format!(
                        "[{}] Forced fallback std::fs copy also failed: {}",
                        subdir_name_for_logs, e_std
                    )
                })?;
                eprintln!("cargo:warning=VENDOR_COPIER: [{}] Forced fallback std::fs copy successful.", subdir_name_for_logs);
            } else {
                eprintln!("cargo:warning=VENDOR_COPIER_VERIFY: [{}] fs_extra copy quick verification passed (target exists and is a dir).", subdir_name_for_logs);
            }
        }
    }
    eprintln!(
        "cargo:warning=VENDOR_COPIER: [{}] Finished copying contents of {:?} into {:?}",
        subdir_name_for_logs, source_dir_contents, target_dir_base
    );
    Ok(())
}


pub fn copy_vendor_directories(
    metamorphosis_app_dir: &Path, // Added to get access to src-tauri/scripts
    source_vendor_dir: &Path, // e.g., metamorphosis-app/vendor/
    dest_vendor_dir: &Path,   // e.g., metamorphosis-app/target/debug/vendor/
    force_std_fs_copy: bool,
) -> Result<(), Box<dyn Error>> {
    println!("cargo:warning=VENDOR_COPIER: sync_vendor_to_target called (simulated by copy_vendor_directories entry)");
    eprintln!("cargo:warning=VENDOR_COPIER: Starting copy_vendor_directories.");
    eprintln!("cargo:warning=VENDOR_COPIER: Source vendor dir: {:?}", source_vendor_dir);
    eprintln!("cargo:warning=VENDOR_COPIER: Destination vendor dir (base for subdirs): {:?}", dest_vendor_dir);

    if !source_vendor_dir.is_dir() {
        return Err(format!("Source vendor directory not found or not a directory: {:?}", source_vendor_dir).into());
    }

    // Ensure the top-level destination vendor directory exists (e.g., .../target/debug/vendor/)
    if !dest_vendor_dir.exists() {
        eprintln!("cargo:warning=VENDOR_COPIER: Creating destination vendor base directory: {:?}", dest_vendor_dir);
        fs::create_dir_all(dest_vendor_dir)?;
    }

    // --- Copy ComfyUI ---
    let source_comfyui_path = source_vendor_dir.join("comfyui");
    let target_comfyui_dest_base = dest_vendor_dir.join("comfyui"); // e.g. .../target/debug/vendor/comfyui/
    
    println!("cargo:warning=VENDOR_COPIER: Before attempting to copy comfyui directory.");
    eprintln!("cargo:warning=VENDOR_COPIER: [ComfyUI] Preparing to copy contents.");
    eprintln!("cargo:warning=VENDOR_COPIER: [ComfyUI] Source (contents from): {:?}", &source_comfyui_path);
    eprintln!("cargo:warning=VENDOR_COPIER: [ComfyUI] Target (destination for contents): {:?}", &target_comfyui_dest_base);

    if source_comfyui_path.is_dir() {
        // Cleanup existing target ComfyUI directory before copying contents
        if target_comfyui_dest_base.exists() {
            eprintln!("cargo:warning=VENDOR_COPIER: [ComfyUI] Removing existing target directory: {:?}", &target_comfyui_dest_base);
            fs::remove_dir_all(&target_comfyui_dest_base).map_err(|e| format!("[ComfyUI] Failed to remove existing target dir {:?}: {}", target_comfyui_dest_base, e))?;
        }
        // copy_directory_contents will create target_comfyui_dest_base if it doesn't exist
        copy_directory_contents(&source_comfyui_path, &target_comfyui_dest_base, "ComfyUI", force_std_fs_copy)?;
        eprintln!("cargo:warning=VENDOR_COPIER: [ComfyUI] Content copy process completed.");

        // --- Copy check_torch.py from scripts to ComfyUI vendor directory ---
        let script_check_torch_source = metamorphosis_app_dir.join("src-tauri/scripts/script_check_torch.py");
        let check_torch_dest = target_comfyui_dest_base.join("check_torch.py");

        eprintln!("cargo:warning=VENDOR_COPIER: Attempting to copy script_check_torch.py to ComfyUI vendor directory.");
        eprintln!("cargo:warning=VENDOR_COPIER: Source: {:?}", script_check_torch_source);
        eprintln!("cargo:warning=VENDOR_COPIER: Destination: {:?}", check_torch_dest);

        if script_check_torch_source.exists() {
            fs::copy(&script_check_torch_source, &check_torch_dest)
                .map_err(|e| format!("Failed to copy check_torch.py from {:?} to {:?}: {}", script_check_torch_source, check_torch_dest, e))?;
            eprintln!("cargo:warning=VENDOR_COPIER: Successfully copied script_check_torch.py to check_torch.py.");
        } else {
            eprintln!("cargo:warning=VENDOR_COPIER_WARN: script_check_torch.py not found at {:?}. Skipping copy.", script_check_torch_source);
        }

        // Verification
        let check_main_py = target_comfyui_dest_base.join("main.py");
        if !check_main_py.exists() {
            let err_msg = format!(
                "CRITICAL VENDOR_COPIER ERROR: comfyui/main.py NOT found in target vendor directory after copy. Checked: {:?}",
                check_main_py
            );
            eprintln!("cargo:warning={}", err_msg);
            eprintln!("cargo:warning=VENDOR_COPIER_DIAG: [ComfyUI] Contents of {:?}:", target_comfyui_dest_base);
            if target_comfyui_dest_base.exists() {
                for entry in fs::read_dir(&target_comfyui_dest_base)? {
                    eprintln!("cargo:warning=VENDOR_COPIER_DIAG: [ComfyUI]   - {:?}", entry?.path());
                }
            } else {
                 eprintln!("cargo:warning=VENDOR_COPIER_DIAG: [ComfyUI] Target directory {:?} does not exist.", target_comfyui_dest_base);
            }
            return Err(err_msg.into());
        }
        eprintln!("cargo:warning=VENDOR_COPIER_VERIFY: [ComfyUI] main.py FOUND in target: {:?}", check_main_py);
    } else {
        let err_msg = format!("[ComfyUI] Source directory not found or not a directory: {:?}. This is critical and indicates a problem with ComfyUI installation.", source_comfyui_path);
        eprintln!("cargo:warning=VENDOR_COPIER_ERROR: {}", err_msg);
        return Err(err_msg.into());
    }
    eprintln!("cargo:warning=VENDOR_COPIER: [ComfyUI] Section finished.");
    println!("cargo:warning=VENDOR_COPIER: After attempting to copy comfyui directory.");

    println!("cargo:warning=VENDOR_COPIER: After attempting to copy python directory.");

    eprintln!("cargo:warning=VENDOR_COPIER: Vendor directory copying finished successfully.");
    Ok(())
}