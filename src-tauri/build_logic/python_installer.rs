use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use super::paths::{
    BASE_URL, PYTHON_RELEASE_TAG, PYTHON_VERSION,
    get_source_python_base_install_dir, get_source_python_root_install_dir,
};
use super::archive_utils; // Use super to access archive_utils

fn get_python_download_info(
    target: &str,
) -> Result<(String, String, String), Box<dyn Error>> { // Return: download_url, asset_filename, archive_ext
    let (target_triple_short, base_archive_ext_str, is_windows_x64_install_only) = match target {
        "x86_64-pc-windows-msvc" => ("x86_64-pc-windows-msvc", "tar.gz", true), // Special case for install_only
        "aarch64-pc-windows-msvc" => ("aarch64-pc-windows-msvc", "zip", false), // Assuming standard for now
        "x86_64-unknown-linux-gnu" => ("x86_64-unknown-linux-gnu", "tar.gz", false),
        "aarch64-unknown-linux-gnu" => ("aarch64-unknown-linux-gnu", "tar.gz", false),
        "x86_64-apple-darwin" => ("x86_64-apple-darwin", "tar.gz", false),
        "aarch64-apple-darwin" => ("aarch64-apple-darwin", "tar.gz", false),
        _ => return Err(format!("Unsupported target: {}", target).into()),
    };

    let asset_filename: String;
    let archive_ext_to_use: String = base_archive_ext_str.to_string();

    if is_windows_x64_install_only {
        // Construct filename for cpython-<VERSION>+<TAG>-<TRIPLE>-install_only.<EXT>
        asset_filename = format!(
            "cpython-{}+{}-{}-install_only.{}",
            PYTHON_VERSION, PYTHON_RELEASE_TAG, target_triple_short, archive_ext_to_use
        );
    } else {
        // Standard naming: cpython-<VERSION>-<TRIPLE>.<EXT>
        asset_filename = format!(
            "cpython-{}-{}.{}",
            PYTHON_VERSION, target_triple_short, archive_ext_to_use
        );
    }

    let download_url = format!(
        "{}/{}/{}",
        BASE_URL, PYTHON_RELEASE_TAG, asset_filename
    );

    Ok((download_url, asset_filename, archive_ext_to_use))
}

fn download_file(url: &str, dest_path: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:warning=PY_INSTALLER: Downloading from {} to {:?}", url, dest_path);
    let response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download Python: HTTP {}",
            response.status()
        )
        .into());
    }
    let mut dest_file = fs::File::create(dest_path)?;
    let content = response.bytes()?;
    std::io::copy(&mut content.as_ref(), &mut dest_file)?;
    println!("cargo:warning=PY_INSTALLER: Download complete: {:?}", dest_path);
    Ok(())
}

pub fn ensure_python_installed(
    source_vendor_dir: &Path,
    out_dir: &Path, // For downloads
) -> Result<(), Box<dyn Error>> {
    let target = env::var("TARGET")?;
    let python_base_install_path = get_source_python_base_install_dir(source_vendor_dir);
    let python_root_install_path = get_source_python_root_install_dir(&python_base_install_path);

    println!("cargo:warning=PY_INSTALLER: Python base install path: {:?}", python_base_install_path);
    println!("cargo:warning=PY_INSTALLER: Python root install path (check): {:?}", python_root_install_path);

    // --- 1. Determine Python Download URL ---
    let (download_url, asset_filename_str, archive_ext_str) = get_python_download_info(&target)?;
    let archive_filename = PathBuf::from(asset_filename_str); // Use the direct asset filename
    let download_path = out_dir.join(&archive_filename);
    println!("cargo:warning=PY_INSTALLER: Calculated Python download URL: {}", download_url);
    println!("cargo:warning=PY_INSTALLER: Archive download path: {:?}", download_path);


    // --- 2. Create Source Vendor Python Directory ---
    // source_vendor_dir itself should be created by the main build script if it calls this.
    // This function ensures the specific python base install directory exists.
    fs::create_dir_all(&python_base_install_path)?;
    println!("cargo:warning=PY_INSTALLER: Ensured Python base install directory exists: {:?}", python_base_install_path);


    // --- 3. Download Python Runtime (with caching) ---
    if !python_root_install_path.exists() { // This check might still be problematic if python_root_install_path is not where we expect python.exe
        println!("cargo:warning=PY_INSTALLER: Python not found in vendor directory (based on root_install_path check). Downloading...");
        if !download_path.exists() {
            println!("cargo:warning=PY_INSTALLER: Downloading Python standalone from: {}", download_url);
            match download_file(&download_url, &download_path) {
                Ok(_) => {
                    println!("cargo:warning=PY_INSTALLER: Python download successful: {:?}", download_path);
                }
                Err(e) => {
                    println!("cargo:warning=PY_INSTALLER: Python download FAILED: {}", e);
                    return Err(e);
                }
            }
        } else {
            println!("cargo:warning=PY_INSTALLER: Using cached Python download: {:?}", download_path);
        }

        // --- 4. Extract Python Runtime ---
        println!("cargo:warning=PY_INSTALLER: Target Python extraction directory: {:?}", python_base_install_path);
        
        let extract_result = std::panic::catch_unwind(|| {
            archive_utils::extract_archive(&download_path, &python_base_install_path, &archive_ext_str)
        });

        match extract_result {
            Ok(Ok(())) => {
                println!("cargo:warning=PY_INSTALLER: Python extraction successful.");
            }
            Ok(Err(e)) => {
                println!("cargo:warning=PY_INSTALLER: Python extraction FAILED: {}", e);
                return Err(e);
            }
            Err(panic_payload) => {
                println!("cargo:warning=PY_INSTALLER: Panic during Python extraction.");
                eprintln!("\n\n===== PANIC CAUGHT DURING PYTHON EXTRACTION (python_installer.rs) =====");
                if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    eprintln!("Panic payload (str): {}", s);
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    eprintln!("Panic payload (String): {}", s);
                } else {
                    eprintln!("Panic payload (Unknown type)");
                }
                eprintln!("======================================================================\n");
                std::panic::resume_unwind(panic_payload);
            }
        }
    } else {
        println!("cargo:warning=PY_INSTALLER: Found existing Python installation in: {:?}", python_root_install_path); // This log might be misleading if python_root_install_path is not the actual install dir
    }

    // --- 5. Verify Python Executable in source vendor ---
    // This check is primarily for Windows (python.exe).
    // For Linux/macOS, it would typically be 'bin/python'.
    // python_base_install_path is the directory where the archive is extracted.
    // The actual python executable should be directly in python_base_install_path (Windows)
    // or python_base_install_path/bin (Unix-like) after extraction.
    let python_exe_path_in_source_windows = python_base_install_path.join("python.exe");
    let python_exe_path_in_source_unix = python_base_install_path.join("bin").join("python");


    if cfg!(windows) && python_exe_path_in_source_windows.exists() {
        println!("cargo:warning=PY_INSTALLER: Verified python.exe found in source vendor at: {:?}", python_exe_path_in_source_windows);
    } else if !cfg!(windows) && python_exe_path_in_source_unix.exists() {
        println!("cargo:warning=PY_INSTALLER: Verified python found in source vendor at: {:?}", python_exe_path_in_source_unix);
    } else {
         let checked_path_str = if cfg!(windows) {
            format!("{:?}", python_exe_path_in_source_windows)
        } else {
            // For non-Windows, show both the Unix path and the Windows path for comprehensive debugging if Unix one fails.
            format!("{:?} (primary check) and also checked Windows path {:?}", python_exe_path_in_source_unix, python_exe_path_in_source_windows)
        };
        let err_msg = format!(
            "CRITICAL: Python executable NOT found in source vendor directory after install/extraction. Checked: {}",
            checked_path_str
        );
        println!("cargo:warning=PY_INSTALLER_ERROR: {}", err_msg);
        // List contents for debugging - now listing the corrected base path
        println!("cargo:warning=PY_INSTALLER_DEBUG: Listing contents of python_base_install_path ({:?}):", python_base_install_path);
        if python_base_install_path.exists() {
            for entry in fs::read_dir(&python_base_install_path)? {
                let entry = entry?;
                println!("cargo:warning=PY_INSTALLER_DEBUG:   - {:?}", entry.path());
            }
        } else {
            println!("cargo:warning=PY_INSTALLER_DEBUG: python_base_install_path does not exist.");
        }
        return Err(err_msg.into());
    }
    Ok(())
}