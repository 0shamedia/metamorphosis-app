use std::error::Error;
use std::fs; // Keep fs for potential top-level directory creations if needed

// Declare the new module
mod build_logic;

// Use items from the new modules
use build_logic::{paths, vendor_copier, comfyui_installer};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_logic/mod.rs");
    println!("cargo:rerun-if-changed=build_logic/paths.rs");
    println!("cargo:warning=BUILD_RS: Rerun if paths.rs changed.");
    println!("cargo:rerun-if-changed=build_logic/archive_utils.rs");
    println!("cargo:warning=BUILD_RS: Rerun if archive_utils.rs changed.");
    println!("cargo:rerun-if-changed=build_logic/vendor_copier.rs");
    println!("cargo:warning=BUILD_RS: Rerun if vendor_copier.rs changed.");

    // Assume ComfyUI source is already in ../vendor/comfyui (relative to src-tauri)
    // This path will be derived more robustly using the paths module.
    // The rerun-if-changed for requirements.txt should point to the actual source location.
    // println!("cargo:rerun-if-changed=../vendor/comfyui/requirements.txt"); // Will be handled by source_vendor_dir logic

    eprintln!("cargo:warning=BUILD_RS_MAIN: Starting build script execution.");

    // --- 1. Determine Core Paths ---
    eprintln!("cargo:warning=BUILD_RS_MAIN: Stage 1: Determining Core Paths.");
    let metamorphosis_app_dir = paths::get_metamorphosis_app_dir()?;
    let out_dir = paths::get_out_dir()?; // For Python download cache
    let source_vendor_dir = paths::get_source_vendor_dir(&metamorphosis_app_dir);
    
    // Ensure the source vendor directory exists before trying to use it.
    // python_installer and vendor_copier will handle their specific subdirectories.
    if !source_vendor_dir.exists() {
        eprintln!("cargo:warning=BUILD_RS_MAIN: Source vendor directory {:?} does not exist. Creating it.", source_vendor_dir);
        fs::create_dir_all(&source_vendor_dir)
            .map_err(|e| format!("Failed to create source vendor directory {:?}: {}", source_vendor_dir, e))?;
    }

    // --- 1.5. Ensure ComfyUI Base is Installed in Source Vendor ---
    eprintln!("cargo:warning=BUILD_RS_MAIN: Stage 1.5: Ensuring ComfyUI is installed in source vendor directory.");
    comfyui_installer::ensure_comfyui_base_installed(&metamorphosis_app_dir)
        .map_err(|e| format!("ComfyUI base installation failed: {}", e))?;
    eprintln!("cargo:warning=BUILD_RS_MAIN: ComfyUI base installation in source vendor directory ensured.");




    // --- 2. Copy Vendor Directories to Build Output ---
    eprintln!("cargo:warning=BUILD_RS_MAIN: Stage 2: Copying vendor directories to build output.");
    let target_profile_dir = paths::get_target_profile_dir(&metamorphosis_app_dir)?;
    let dest_vendor_dir = paths::get_dest_vendor_dir(&target_profile_dir);

    // Critical: Force use of std::fs for copying due to persistent fs_extra issues.
    let force_std_fs_copy = true; 
    eprintln!("cargo:warning=BUILD_RS_MAIN: Forcing std::fs for vendor copy: {}", force_std_fs_copy);

    vendor_copier::copy_vendor_directories(&metamorphosis_app_dir, &source_vendor_dir, &dest_vendor_dir, force_std_fs_copy)
        .map_err(|e| format!("Vendor directory copying failed: {}", e))?;
    eprintln!("cargo:warning=BUILD_RS_MAIN: Vendor directories copied to build output.");

    // --- 3. Tauri Build ---
    eprintln!("cargo:warning=BUILD_RS_MAIN: Stage 3: Running Tauri build.");
    tauri_build::build();
    eprintln!("cargo:warning=BUILD_RS_MAIN: Tauri build finished.");

    eprintln!("cargo:warning=BUILD_RS_MAIN: Build script execution completed successfully.");
    Ok(())
}
