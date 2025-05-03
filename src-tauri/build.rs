use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{self, Cursor},
    path::{Path, PathBuf},
};
use std::io::Write; // Import the Write trait for write_all


use std::panic;

use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;

const PYTHON_VERSION: &str = "3.12.10"; // Upgrade to Python 3.12.10
const PYTHON_RELEASE_TAG: &str = "20250409"; // Use latest valid release tag
const BASE_URL: &str = "https://github.com/astral-sh/python-build-standalone/releases/download"; // Updated base URL to astral-sh repo
const VENDOR_DIR: &str = "../vendor"; // Relative to src-tauri
const PYTHON_INSTALL_DIR_NAME: &str = "python"; // Directory inside vendor where python will be extracted

fn copy_recursively(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    // Create the destination directory if it doesn't exist
    fs::create_dir_all(destination)?;

    // Iterate over entries in the source directory
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let entry_filename = entry_path.file_name().ok_or("Invalid file name")?;
        let dest_path = destination.join(entry_filename);

        if entry_path.is_dir() {
            // If it's a directory, recursively call copy_recursively
            copy_recursively(&entry_path, &dest_path)?;
        } else {
            // If it's a file, copy it
            fs::copy(&entry_path, &dest_path)?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    // Assume ComfyUI source is already in ../vendor/comfyui
    println!("cargo:rerun-if-changed=../vendor/comfyui/requirements.txt");

    let target = env::var("TARGET")?;
    let out_dir = PathBuf::from(env::var("OUT_DIR")?); // Temp dir for downloads
    let vendor_path = PathBuf::from(VENDOR_DIR);
    let python_base_install_path = vendor_path.join(PYTHON_INSTALL_DIR_NAME);
    let python_root_install_path = python_base_install_path.join("python"); // The actual python install is nested

    // --- 1. Determine Python Download URL ---
    let (download_url, archive_ext, target_triple_short) = get_python_download_info(&target)?;
    let archive_filename = format!("python-{}+{}-{}.{}", PYTHON_VERSION, PYTHON_RELEASE_TAG, target_triple_short, archive_ext);
    let download_path = out_dir.join(&archive_filename);

    // --- 2. Create Vendor Directory ---
    fs::create_dir_all(&vendor_path)?;
    fs::create_dir_all(&python_base_install_path)?;

    // --- 3. Download Python Runtime (with caching) ---
    if !python_root_install_path.exists() { // Check if final install dir exists
        println!("Python not found in vendor directory. Downloading...");
        if !download_path.exists() {
            println!("Downloading Python standalone from: {}", download_url);
            download_file(&download_url, &download_path)?;
            println!("Downloaded Python to: {:?}", download_path);
        } else {
            println!("Using cached Python download: {:?}", download_path);
        }

        // --- 4. Extract Python Runtime ---
        println!("Extracting Python to: {:?}", python_base_install_path);
        let extract_result = std::panic::catch_unwind(|| {
            // This closure now returns the Result from extract_archive
            extract_archive(&download_path, &python_base_install_path, archive_ext) // Remove out_dir
        });

        match extract_result {
            Ok(Ok(())) => {
                println!("Extracted Python successfully.");
            }
            Ok(Err(e)) => {
                // Handle errors returned by extract_archive itself
                eprintln!("Error during Python extraction: {}", e);
                return Err(e); // Propagate the original error
            }
            Err(panic_payload) => {
                // Handle panics that occurred within extract_archive or the closure
                eprintln!("\n\n===== PANIC CAUGHT DURING PYTHON EXTRACTION =====");
                if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    eprintln!("Panic payload (str): {}", s);
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    eprintln!("Panic payload (String): {}", s);
                } else {
                    eprintln!("Panic payload (Unknown type)");
                }
                eprintln!("==============================================\n");
                // Re-panic to ensure the build fails, propagating the panic
                std::panic::resume_unwind(panic_payload);
            }
        }

    } else {
        println!("Found existing Python installation in: {:?}", python_root_install_path); // Note: This check might be less reliable now if ensurepip fails mid-way on a subsequent run. Consider always cleaning.
    }


    // --- 6. Copy Vendor Directory to Build Output ---
    println!("Copying vendor directory to build output...");
    let build_profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()); // Get build profile (debug/release)
    // Calculate target dir relative to OUT_DIR, which is inside target/<profile>/build/app-<hash>/out
    // We need to go up 4 levels to reach target/<profile>/
    let target_profile_dir = out_dir.join("../../../../");
    let dest_vendor_path = target_profile_dir.join("vendor");
    println!("DEBUG: Destination vendor path calculated by build.rs: {:?}", dest_vendor_path);
    
    // Get the absolute path of the source vendor directory using CARGO_MANIFEST_DIR
    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // CARGO_MANIFEST_DIR points to src-tauri, go up one level to metamorphosis-app
    let metamorphosis_app_dir = cargo_manifest_dir.parent().ok_or("Failed to get parent directory from CARGO_MANIFEST_DIR")?;
    let absolute_vendor_path = metamorphosis_app_dir.join("vendor"); // Path to vendor from metamorphosis-app directory
    
    // Ensure the destination vendor directory exists
    fs::create_dir_all(&dest_vendor_path)?;

    // Attempt to remove the destination directory before copying to avoid permission/lock issues
    println!("Attempting to clean destination vendor directory: {:?}", &dest_vendor_path);
    if dest_vendor_path.exists() {
        match fs::remove_dir_all(&dest_vendor_path) {
            Ok(_) => println!("Successfully cleaned destination vendor directory."),
            Err(e) => eprintln!("Warning: Failed to clean destination vendor directory: {}. Proceeding with copy, but this might cause issues.", e),
        }
    }


    println!("Source vendor path (absolute): {:?}", &absolute_vendor_path);
    println!("Destination vendor path (absolute): {:?}", &dest_vendor_path);

    // Use Rust-native recursive directory copying
    println!("Using Rust-native recursive directory copying...");
    copy_recursively(&absolute_vendor_path, &dest_vendor_path)?;
    


    println!("BUILD_RS_TEST: Reached verification section.");

    // --- Verify requirements.txt exists after copy ---
    let requirements_dest_path = dest_vendor_path.join("comfyui/requirements.txt");
    if requirements_dest_path.exists() {
        println!("BUILD_RS_VERIFY: requirements.txt found at {:?}", requirements_dest_path);
    } else {
        println!("BUILD_RS_VERIFY: requirements.txt NOT found at {:?}", requirements_dest_path);
    }
    // --- End verification ---

    // --- Tauri Build ---
    println!("Running Tauri build...");
    tauri_build::build();

    Ok(())
}

fn get_python_download_info(target: &str) -> Result<(String, &'static str, String), Box<dyn Error>> {
    let (target_triple_short, install_mode, archive_ext) = match target {
        "x86_64-pc-windows-msvc" => ("x86_64-pc-windows-msvc", "install_only", "tar.gz"), // Switch to install_only tar.gz
        "aarch64-pc-windows-msvc" => ("aarch64-pc-windows-msvc", "shared-pgo+lto", "zip"),
        "x86_64-apple-darwin" => ("x86_64-apple-darwin", "install_only", "tar.gz"),
        "aarch64-apple-darwin" => ("aarch64-apple-darwin", "install_only", "tar.gz"),
        "x86_64-unknown-linux-gnu" => ("x86_64-unknown-linux-gnu", "install_only", "tar.gz"),
        // Add other linux targets as needed (e.g., musl)
        "aarch64-unknown-linux-gnu" => ("aarch64-unknown-linux-gnu", "install_only", "tar.gz"),
        _ => return Err(format!("Unsupported target: {}", target).into()),
    };

    let filename_part = format!(
        "cpython-{}+{}-{}-{}",
        PYTHON_VERSION, PYTHON_RELEASE_TAG, target_triple_short, install_mode
    );

    let url = format!(
        "{}/{}/{}",
        BASE_URL, PYTHON_RELEASE_TAG, filename_part
    );

    // Append variant and extension based on OS
    // Construct the final URL - no '-full' suffix needed for install_only
    let full_url = format!("{}.{}", url, archive_ext);

    Ok((full_url, archive_ext, filename_part)) // Return filename_part without extension for archive_filename construction
}


fn download_file(url: &str, dest_path: &Path) -> Result<(), Box<dyn Error>> {
    let response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download file: {} (Status: {})",
            url,
            response.status()
        )
        .into());
    }
    let mut dest_file = File::create(dest_path)?;
    let content = response.bytes()?;
    io::copy(&mut Cursor::new(content), &mut dest_file)?;
    Ok(())
}

fn extract_archive(
    archive_path: &Path,
    extract_to: &Path,
    archive_ext: &str,
    // _out_dir parameter removed as it's no longer needed
) -> Result<(), Box<dyn Error>> {
    let file = File::open(archive_path)?;

    if archive_ext == "zip" {
        let mut archive = ZipArchive::new(file)?;
        // Ensure extraction happens directly into the target dir, handling nested 'python' dir if present
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => extract_to.join(path), // Use extract_to as base
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
             // Set permissions on Unix if needed (omitted for brevity)
        }

    } else if archive_ext == "tar.gz" {
        println!("Extracting tar.gz using Rust crates (tar, flate2)...");
        let tar_gz = File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        // Ensure the target directory exists
        fs::create_dir_all(extract_to)?;

        // Unpack directly into the target directory.
        // Manually unpack entries and strip the leading 'python/' component
        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?;

            // Strip the leading 'python/' component
            let stripped_path = match path.strip_prefix("python") {
                 Ok(p) => p,
                 Err(_) => {
                      // Should not happen if archive structure is as expected, but handle defensively
                      println!("Warning: Entry path {:?} does not start with 'python/', skipping.", path);
                      continue;
                 }
            };

            // Skip if the path is now empty (it was just the 'python/' directory itself)
            if stripped_path.as_os_str().is_empty() {
                continue;
            }

            let dest_path = extract_to.join(stripped_path);

            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Unpack the individual entry to the correct destination
            entry.unpack(&dest_path)?;
        }

        println!("Successfully extracted tar.gz archive using Rust crates (with manual stripping).");

    } else {
        // Keep zip handling as is
        return Err(format!("Unsupported archive extension: {}", archive_ext).into());
    }

    /*
    let nested_python_dir = extract_to.join("python");
    if nested_python_dir.is_dir() {
        println!("Moving extracted contents from {:?} to {:?}", nested_python_dir, extract_to);
        let mut options = CopyOptions::new();
        options.content_only = true;
        options.overwrite = true;
        copy(&nested_python_dir, extract_to, &options)?;
        fs::remove_dir_all(&nested_python_dir)?;
        println!("Successfully moved contents.");
    }
    */


    Ok(())
}


fn get_python_paths(python_root: &Path, target: &str) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let (pip_rel_path, site_packages_rel_path) = if target.contains("windows") {
        ("Scripts/pip.exe".to_string(), "Lib/site-packages".to_string())
    } else {
        // Assume Unix-like structure
        ("bin/pip".to_string(), format!("lib/python{}/site-packages", PYTHON_VERSION.split('.').take(2).collect::<Vec<&str>>().join("."))) // Use major.minor from PYTHON_VERSION
    };

    let pip_path = python_root.join(pip_rel_path);
    let site_packages_path = python_root.join(site_packages_rel_path);

    Ok((pip_path, site_packages_path))
}
