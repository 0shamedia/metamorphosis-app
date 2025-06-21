use std::{
    error::Error,
    fs,
    path::Path,
    process::Stdio, // Ensure Stdio is imported for std::process::Command
};
use super::paths::{
    get_source_vendor_dir,
};
// Removed tokio::process::Command as build scripts must be synchronous
// Removed unused imports: env, PathBuf

// Constants for ComfyUI
const COMFYUI_REPO_URL: &str = "https://github.com/comfyanonymous/ComfyUI";
const COMFYUI_BRANCH: &str = "master"; // Or a specific tag/commit for version control

/// Ensures the base ComfyUI repository is present in the source vendor directory.
/// If not found, it clones the repository.
pub fn ensure_comfyui_base_installed(
    metamorphosis_app_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let source_vendor_dir = get_source_vendor_dir(metamorphosis_app_dir);
    let comfyui_path = source_vendor_dir.join("comfyui");

    println!("cargo:warning=COMFYUI_INSTALLER: Checking for ComfyUI at {:?}", comfyui_path);

    if comfyui_path.exists() {
        println!("cargo:warning=COMFYUI_INSTALLER: ComfyUI directory already exists at {:?}. Skipping clone.", comfyui_path);
        return Ok(());
    }

    println!("cargo:warning=COMFYUI_INSTALLER: ComfyUI not found. Cloning from {} (branch: {}) into {:?}", COMFYUI_REPO_URL, COMFYUI_BRANCH, comfyui_path);

    // Ensure the parent directory exists for cloning
    if let Some(parent) = comfyui_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
            println!("cargo:warning=COMFYUI_INSTALLER: Created parent directory for ComfyUI: {:?}", parent);
        }
    }

    // Use std::process::Command for blocking operation in build script
    let mut command = std::process::Command::new("git");
    command.arg("clone")
           .arg("--depth") // Only clone the latest commit
           .arg("1")
           .arg("--branch")
           .arg(COMFYUI_BRANCH)
           .arg(COMFYUI_REPO_URL)
           .arg(&comfyui_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

    println!("cargo:warning=COMFYUI_INSTALLER: Executing git clone command for ComfyUI...");
    let output = command.output()?;

    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        println!("cargo:warning=COMFYUI_INSTALLER: Successfully cloned ComfyUI. Output: {}", stdout_str.trim());
        Ok(())
    } else {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let err_msg = format!(
            "Failed to clone ComfyUI. Git command exited with error. Status: {:?}. Stderr: {}",
            output.status.code(),
            stderr_str.trim()
        );
        println!("cargo:warning=COMFYUI_INSTALLER_ERROR: {}", err_msg);
        Err(err_msg.into())
    }
}