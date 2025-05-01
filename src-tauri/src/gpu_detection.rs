use log::{info, error};

// Enum to represent the detected GPU type
#[derive(Debug, PartialEq)]
pub enum GpuType {
    Nvidia,
    Amd,
    Intel,
    Other,
    Unknown, // Added Unknown variant
}

// Struct to hold detailed GPU information
#[derive(Debug)]
pub struct GpuInfo {
    pub gpu_type: GpuType,
    pub cuda_version: Option<String>,
}

// Function to get detailed GPU information
pub fn get_gpu_info() -> GpuInfo {
    info!("Attempting to get detailed GPU information...");

    #[cfg(target_os = "windows")]
    {
        info!("Attempting Windows-specific GPU detection using wmic...");
        match std::process::Command::new("wmic").args(&["path", "win32_videocontroller", "get", "name"]).output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    info!("wmic output:\n{}", stdout);

                    let gpu_name = stdout.to_lowercase();
                    if gpu_name.contains("nvidia") {
                        info!("Detected NVIDIA GPU via wmic.");
                        // Now try to get CUDA version using nvidia-smi
                        match std::process::Command::new("nvidia-smi").output() {
                            Ok(output) => {
                                if output.status.success() {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    info!("nvidia-smi output:\n{}", stdout);
                                    let cuda_version = stdout
                                        .lines()
                                        .find(|line| line.contains("CUDA Version:"))
                                        .and_then(|line| {
                                            line.split("CUDA Version:")
                                                .nth(1)
                                                .and_then(|version_part| version_part.split_whitespace().next())
                                                .map(|version| version.to_string())
                                        });
                                    if let Some(version) = &cuda_version {
                                        info!("Detected CUDA Version: {}", version);
                                    } else {
                                        info!("CUDA Version not found in nvidia-smi output.");
                                    }
                                    return GpuInfo {
                                        gpu_type: GpuType::Nvidia,
                                        cuda_version,
                                    };
                                } else {
                                    error!("nvidia-smi command failed with status: {:?}", output.status);
                                    error!("nvidia-smi stderr:\n{}", String::from_utf8_lossy(&output.stderr));
                                    info!("nvidia-smi command failed, returning NVIDIA GPU type without CUDA version.");
                                    return GpuInfo {
                                        gpu_type: GpuType::Nvidia,
                                        cuda_version: None,
                                    };
                                }
                            }
                            Err(e) => {
                                error!("Failed to execute nvidia-smi: {}", e);
                                info!("nvidia-smi not found or failed to execute, returning NVIDIA GPU type without CUDA version.");
                                return GpuInfo {
                                    gpu_type: GpuType::Nvidia,
                                    cuda_version: None,
                                };
                            }
                        }
                    } else if gpu_name.contains("amd") || gpu_name.contains("radeon") {
                        info!("Detected AMD GPU via wmic.");
                        // TODO: Implement ROCm version detection for Windows if possible/needed.
                        return GpuInfo {
                            gpu_type: GpuType::Amd,
                            cuda_version: None, // AMD uses ROCm, not CUDA
                        };
                    } else if gpu_name.contains("intel") {
                        info!("Detected Intel GPU via wmic.");
                        // TODO: Implement Intel GPU version detection for Windows if possible/needed.
                        return GpuInfo {
                            gpu_type: GpuType::Intel,
                            cuda_version: None, // Intel uses different technologies
                        };
                    } else {
                        info!("Detected other or unknown GPU via wmic: {}", stdout.trim());
                        // Fall through to other detection methods or default
                    }
                } else {
                    error!("wmic command failed with status: {:?}", output.status);
                    error!("wmic stderr:\n{}", String::from_utf8_lossy(&output.stderr));
                    info!("wmic command failed, proceeding with other detection methods.");
                    // Fall through to other detection methods or default
                }
            }
            Err(e) => {
                error!("Failed to execute wmic: {}", e);
                info!("wmic not found or failed to execute, proceeding with other detection methods.");
                // Fall through to other detection methods or default
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        info!("Attempting Linux-specific GPU detection using lspci...");
        match std::process::Command::new("lspci").args(&["-vnn"]).output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    info!("lspci output:\n{}", stdout);

                    let mut gpu_type = GpuType::Unknown;
                    let mut cuda_version: Option<String> = None;

                    for line in stdout.lines() {
                        let lower_line = line.to_lowercase();
                        if lower_line.contains("vga compatible controller") || lower_line.contains("3d controller") {
                            if lower_line.contains("vendor 0x10de") || lower_line.contains("nvidia") {
                                info!("Detected NVIDIA GPU via lspci.");
                                gpu_type = GpuType::Nvidia;
                                // Attempt to get CUDA version using nvidia-smi on Linux
                                match std::process::Command::new("nvidia-smi").output() {
                                    Ok(output) => {
                                        if output.status.success() {
                                            let stdout = String::from_utf8_lossy(&output.stdout);
                                            info!("nvidia-smi output (Linux):\n{}", stdout);
                                            cuda_version = stdout
                                                .lines()
                                                .find(|line| line.contains("CUDA Version:"))
                                                .and_then(|line| {
                                                    line.split("CUDA Version:")
                                                        .nth(1)
                                                        .and_then(|version_part| version_part.split_whitespace().next())
                                                        .map(|version| version.to_string())
                                                });
                                            if cuda_version.is_some() {
                                                info!("Detected CUDA Version (Linux): {}", cuda_version.as_ref().unwrap());
                                            } else {
                                                info!("CUDA Version not found in nvidia-smi output (Linux).");
                                            }
                                        } else {
                                            error!("nvidia-smi command failed with status (Linux): {:?}", output.status);
                                            error!("nvidia-smi stderr (Linux):\n{}", String::from_utf8_lossy(&output.stderr));
                                            info!("nvidia-smi command failed on Linux, CUDA version unknown.");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to execute nvidia-smi (Linux): {}", e);
                                        info!("nvidia-smi not found or failed to execute on Linux, CUDA version unknown.");
                                    }
                                }
                                break; // Found the primary GPU, exit loop
                            } else if lower_line.contains("vendor 0x1002") || lower_line.contains("amd") || lower_line.contains("radeon") {
                                info!("Detected AMD GPU via lspci.");
                                gpu_type = GpuType::Amd;
                                // TODO: Implement ROCm version detection for Linux (using rocm-smi)
                                break; // Found the primary GPU, exit loop
                            } else if lower_line.contains("vendor 0x8086") || lower_line.contains("intel") {
                                info!("Detected Intel GPU via lspci.");
                                gpu_type = GpuType::Intel;
                                // TODO: Implement Intel GPU version detection for Linux
                                break; // Found the primary GPU, exit loop
                            } else {
                                info!("Detected other or unknown GPU via lspci: {}", line.trim());
                                gpu_type = GpuType::Other;
                                // Continue searching in case there's a more specific entry
                            }
                        }
                    }

                    if gpu_type != GpuType::Unknown {
                         return GpuInfo {
                            gpu_type,
                            cuda_version, // Will be Some for NVIDIA, None otherwise
                        };
                    } else {
                        info!("lspci output did not clearly identify a known GPU type.");
                        // Fall through to other detection methods or default
                    }

                } else {
                    error!("lspci command failed with status: {:?}", output.status);
                    error!("lspci stderr:\n{}", String::from_utf8_lossy(&output.stderr));
                    info!("lspci command failed, proceeding with other detection methods.");
                    // Fall through to other detection methods or default
                }
            }
            Err(e) => {
                error!("Failed to execute lspci: {}", e);
                info!("lspci not found or failed to execute, proceeding with other detection methods.");
                // Fall through to other detection methods or default
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        info!("Attempting macOS-specific GPU detection using system_profiler...");
        match std::process::Command::new("system_profiler").args(&["SPDisplaysDataType"]).output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    info!("system_profiler output:\n{}", stdout);

                    let mut gpu_type = GpuType::Unknown;

                    // system_profiler output is structured, look for lines indicating GPU details
                    for line in stdout.lines() {
                        let trimmed_line = line.trim();
                        let lower_line = trimmed_line.to_lowercase();

                        if lower_line.contains("chipset model:") {
                            if lower_line.contains("amd") || lower_line.contains("radeon") {
                                info!("Detected AMD GPU via system_profiler.");
                                gpu_type = GpuType::Amd;
                                break; // Found the primary GPU, exit loop
                            } else if lower_line.contains("intel") {
                                info!("Detected Intel GPU via system_profiler.");
                                gpu_type = GpuType::Intel;
                                break; // Found the primary GPU, exit loop
                            } else if lower_line.contains("nvidia") || lower_line.contains("geforce") {
                                info!("Detected NVIDIA GPU via system_profiler.");
                                gpu_type = GpuType::Nvidia;
                                // CUDA is generally not available on modern macOS, so no version detection needed.
                                break; // Found the primary GPU, exit loop
                            } else {
                                info!("Detected other or unknown GPU via system_profiler: {}", trimmed_line);
                                gpu_type = GpuType::Other;
                                // Continue searching in case there's a more specific entry
                            }
                        }
                    }

                    if gpu_type != GpuType::Unknown {
                         return GpuInfo {
                            gpu_type,
                            cuda_version: None, // CUDA not typically available on macOS
                        };
                    } else {
                        info!("system_profiler output did not clearly identify a known GPU type.");
                        // Fall through to other detection methods or default
                    }

                } else {
                    error!("system_profiler command failed with status: {:?}", output.status);
                    error!("system_profiler stderr:\n{}", String::from_utf8_lossy(&output.stderr));
                    info!("system_profiler command failed, proceeding with other detection methods.");
                    // Fall through to other detection methods or default
                }
            }
            Err(e) => {
                error!("Failed to execute system_profiler: {}", e);
                info!("system_profiler not found or failed to execute, proceeding with other detection methods.");
                // Fall through to other detection methods or default
            }
        }
    }


    info!("Could not definitively detect NVIDIA, AMD, or Intel GPU using platform-specific methods.");
    GpuInfo {
        gpu_type: GpuType::Unknown, // Default to Unknown if detection is not conclusive
        cuda_version: None,
    }
}