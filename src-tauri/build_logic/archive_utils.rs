use std::{
    fs::{self, File},
    io::{self, Cursor},
    path::Path,
    error::Error,
};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;

/// Extracts an archive file to the specified destination directory.
/// Supports .tar.gz and .zip.
pub fn extract_archive(
    archive_path: &Path,
    extract_to_dir: &Path,
    archive_ext: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "cargo:warning=ARCHIVE_UTILS: Attempting to extract {:?} to {:?} (format: {})",
        archive_path, extract_to_dir, archive_ext
    );

    if !archive_path.exists() {
        return Err(format!("Archive file not found: {:?}", archive_path).into());
    }
    if !extract_to_dir.exists() {
        fs::create_dir_all(extract_to_dir)?;
        println!(
            "cargo:warning=ARCHIVE_UTILS: Created extraction directory: {:?}",
            extract_to_dir
        );
    }

    let file = File::open(archive_path)?;

    match archive_ext {
        "tar.gz" => {
            let tar_gz = GzDecoder::new(file);
            let mut archive = Archive::new(tar_gz);
            println!("cargo:warning=ARCHIVE_UTILS: Processing .tar.gz archive.");

            // Ensure the base extraction directory exists
            fs::create_dir_all(extract_to_dir)?;

            for entry_result in archive.entries()? {
                let mut entry = entry_result?;
                let path_in_archive = entry.path()?.into_owned();

                // Strip the top-level directory (e.g., "python/") from the path
                let stripped_path = match path_in_archive.strip_prefix("python") {
                    Ok(p) => p.to_path_buf(),
                    Err(_) => {
                        // If it doesn't start with "python/", use the original path
                        // This might happen for archives not structured with a single top-level "python" dir
                        // or if the path is already at the root.
                        println!("cargo:warning=ARCHIVE_UTILS: Path in archive does not start with 'python/': {:?}. Using original path.", path_in_archive);
                        path_in_archive.clone() // Use clone if path_in_archive is what we need
                    }
                };

                let final_dest_path = extract_to_dir.join(stripped_path);

                if final_dest_path == extract_to_dir { // Skip if it's the root extraction dir itself
                    println!("cargo:warning=ARCHIVE_UTILS: Skipping extraction of root directory entry: {:?}", path_in_archive);
                    continue;
                }

                if let Some(parent) = final_dest_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
 
                // Commented out to reduce verbosity
                // println!(
                //     "cargo:warning=ARCHIVE_UTILS: Unpacking {:?} to {:?}",
                //     path_in_archive, final_dest_path
                // );
                entry.unpack(&final_dest_path)?;
            }
            println!("cargo:warning=ARCHIVE_UTILS: Finished extracting .tar.gz.");
        }
        "zip" => {
            // ZipArchive requires Read + Seek
            let mut archive = ZipArchive::new(Cursor::new(fs::read(archive_path)?))?;
            println!("cargo:warning=ARCHIVE_UTILS: Processing .zip archive.");

            for i in 0..archive.len() {
                let mut file_in_zip = archive.by_index(i)?;
                let outpath = match file_in_zip.enclosed_name() {
                    Some(path) => extract_to_dir.join(path),
                    None => {
                        println!("cargo:warning=ARCHIVE_UTILS: Skipping entry with invalid path in zip: {}", file_in_zip.name());
                        continue;
                    }
                };

                if (*file_in_zip.name()).ends_with('/') {
                    // Commented out to reduce verbosity
                    // println!(
                    //     "cargo:warning=ARCHIVE_UTILS: Creating directory within zip: {:?}",
                    //     outpath
                    // );
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    // Commented out to reduce verbosity
                    // println!(
                    //     "cargo:warning=ARCHIVE_UTILS: Extracting file from zip: {:?} to {:?}",
                    //     file_in_zip.name(),
                    //     outpath
                    // );
                    let mut outfile = File::create(&outpath)?;
                    io::copy(&mut file_in_zip, &mut outfile)?;
                }

                // Set permissions for Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file_in_zip.unix_mode() {
                        fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                    }
                }
            }
            println!("cargo:warning=ARCHIVE_UTILS: Finished extracting .zip.");
        }
        _ => {
            return Err(format!("Unsupported archive extension: {}", archive_ext).into());
        }
    }
    Ok(())
}