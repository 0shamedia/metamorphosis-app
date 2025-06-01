use std::path::{Path, PathBuf};
use std::env;

pub const PYTHON_VERSION: &str = "3.12.10";
pub const PYTHON_RELEASE_TAG: &str = "20250409";
pub const BASE_URL: &str = "https://github.com/astral-sh/python-build-standalone/releases/download";
pub const VENDOR_DIR_NAME: &str = "vendor"; // Relative to metamorphosis_app_dir
pub const PYTHON_INSTALL_DIR_NAME: &str = "python"; // Directory inside VENDOR_DIR_NAME

/// Returns the root directory of the Tauri application (e.g., `metamorphosis-app/`).
/// This is typically the parent of `src-tauri/` (where `CARGO_MANIFEST_DIR` points).
pub fn get_metamorphosis_app_dir() -> Result<PathBuf, String> {
    let cargo_manifest_dir_str = env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| format!("Failed to get CARGO_MANIFEST_DIR: {}", e))?;
    let cargo_manifest_dir = PathBuf::from(cargo_manifest_dir_str);
    eprintln!("cargo:warning=PATHS: CARGO_MANIFEST_DIR is: {:?}", cargo_manifest_dir);

    let app_dir = cargo_manifest_dir.parent()
        .ok_or_else(|| format!("Failed to get parent of CARGO_MANIFEST_DIR: {:?}", cargo_manifest_dir))?
        .to_path_buf();
    eprintln!("cargo:warning=PATHS: Calculated metamorphosis_app_dir: {:?}", app_dir);
    Ok(app_dir)
}

/// Returns the absolute path to the source vendor directory (e.g., `metamorphosis-app/vendor/`).
pub fn get_source_vendor_dir(app_dir: &Path) -> PathBuf {
    let path = app_dir.join(VENDOR_DIR_NAME);
    eprintln!("cargo:warning=PATHS: Calculated source_vendor_dir: {:?}", path);
    path
}

/// Returns the absolute path to the base Python installation directory within the source vendor directory
/// (e.g., `metamorphosis-app/vendor/python/`).
pub fn get_source_python_base_install_dir(source_vendor_dir: &Path) -> PathBuf {
    let path = source_vendor_dir.join(PYTHON_INSTALL_DIR_NAME);
    eprintln!("cargo:warning=PATHS: Calculated source_python_base_install_dir: {:?}", path);
    path
}

/// Returns the absolute path to the root of the Python installation (where `python.exe` or `bin/python` is)
/// within the source vendor directory (e.g., `metamorphosis-app/vendor/python/python/`).
pub fn get_source_python_root_install_dir(source_python_base_install_dir: &Path) -> PathBuf {
    let path = source_python_base_install_dir.join("python"); // The actual python install is nested
    eprintln!("cargo:warning=PATHS: Calculated source_python_root_install_dir: {:?}", path);
    path
}

/// Returns the path to the target directory for the current build profile
/// (e.g., `metamorphosis-app/target/debug/` or `metamorphosis-app/target/release/`).
pub fn get_target_profile_dir(app_dir: &Path) -> Result<PathBuf, String> {
    let build_profile = env::var("PROFILE").map_err(|e| format!("Failed to get PROFILE env var: {}", e))?;
    eprintln!("cargo:warning=PATHS: Build profile is: {}", build_profile);
    let path = app_dir.join("target").join(build_profile);
    eprintln!("cargo:warning=PATHS: Calculated target_profile_dir: {:?}", path);
    Ok(path)
}

/// Returns the path to the destination vendor directory within the build output
/// (e.g., `metamorphosis-app/target/debug/vendor/`).
pub fn get_dest_vendor_dir(target_profile_dir: &Path) -> PathBuf {
    let path = target_profile_dir.join(VENDOR_DIR_NAME);
    eprintln!("cargo:warning=PATHS: Calculated dest_vendor_dir: {:?}", path);
    path
}

/// Returns the path to the `OUT_DIR` environment variable.
pub fn get_out_dir() -> Result<PathBuf, String> {
    let out_dir_str = env::var("OUT_DIR").map_err(|e| format!("Failed to get OUT_DIR: {}", e))?;
    Ok(PathBuf::from(out_dir_str))
}