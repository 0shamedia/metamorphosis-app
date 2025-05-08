use tauri::{Window, AppHandle, Manager}; // Import Manager
use tauri::Emitter; // Add this to import the Emitter trait
use serde_json::json;
use log::{error, info};
use std::time::Duration;
use tokio::time::sleep;
// use crate::dependency_management; // No longer needed for these path functions
use std::path::PathBuf; // Added for path operations
use std::fs; // Added for directory creation

// Setup phases
#[derive(Debug, Clone, serde::Serialize)]
pub enum SetupPhase {
    Checking,
    InstallingComfyui,
    PythonSetup,
    DownloadingModels,
    Finalizing,
    Complete,
    Error,
}

impl SetupPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            SetupPhase::Checking => "checking",
            SetupPhase::InstallingComfyui => "installing_comfyui",
            SetupPhase::PythonSetup => "python_setup",
            SetupPhase::DownloadingModels => "downloading_models",
            SetupPhase::Finalizing => "finalizing",
            SetupPhase::Complete => "complete",
            SetupPhase::Error => "error",
        }
    }
}

// Model download status
#[derive(Debug, Clone, serde::Serialize)]
pub enum ModelStatus {
    Queued,
    Downloading,
    Verifying,
    Completed,
    Error,
}

impl ModelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelStatus::Queued => "queued",
            ModelStatus::Downloading => "downloading",
            ModelStatus::Verifying => "verifying",
            ModelStatus::Completed => "completed",
            ModelStatus::Error => "error",
        }
    }
}

// Model information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub progress: f32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[tauri::command]
pub async fn check_initialization_status(window: Window) -> Result<(), String> {
    // Record start time for performance tracking
    let start_time = std::time::Instant::now();
    info!("[SETUP] check_initialization_status started");
    
    // Log system information for diagnostics
    info!("[SETUP] System information:");
    info!("[SETUP] OS: {}", std::env::consts::OS);
    info!("[SETUP] Arch: {}", std::env::consts::ARCH);
    info!("[SETUP] Current dir: {:?}", std::env::current_dir().unwrap_or_default());
    
    // Make sure the window is visible first
    info!("[SETUP] Attempting to show window...");
    let show_start = std::time::Instant::now();
    
    match window.show() {
        Ok(_) => {
            let elapsed = show_start.elapsed();
            info!("[SETUP] Window successfully shown in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP] Error showing window: {} (after {:?})", e, show_start.elapsed());
            return Err(format!("Failed to show window: {}", e));
        }
    }
    
    // Check for window dimensions
    match window.inner_size() {
        Ok(size) => {
            info!("[SETUP] Window dimensions: {}x{}", size.width, size.height);
        },
        Err(e) => {
            error!("[SETUP] Error getting window dimensions: {}", e);
        }
    }
    
    // Send initial status - we're initializing
    info!("[SETUP] Emitting initializing status...");
    let emit_start = std::time::Instant::now();
    
    match window.emit("initialization-status", json!({
        "status": "initializing",
        "message": "Initializing Metamorphosis..."
    })) {
        Ok(_) => {
            let elapsed = emit_start.elapsed();
            info!("[SETUP] Successfully emitted initializing status in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP] Error emitting initialization status: {} (after {:?})", e, emit_start.elapsed());
            return Err(format!("Failed to emit status: {}", e));
        }
    }
    
    // Here we could check if there are any required files that need to be present
    // For now, we'll just simulate a brief check
    info!("[SETUP] Performing initialization checks...");
    let check_start = std::time::Instant::now();
    
    // info!("[SETUP] Simulating initialization check (sleeping for 1500ms)...");
    // sleep(Duration::from_millis(1500)).await;

    let app_handle = window.app_handle();

    // Check 1: Verify Application Data Directory
    info!("[SETUP] Check 1: Verifying Application Data Directory...");
    match app_handle.path().app_data_dir() {
        Ok(app_data_path) => {
            if !app_data_path.exists() {
                if let Err(e) = fs::create_dir_all(&app_data_path) {
                    let error_msg = format!("Failed to create app data directory at {:?}: {}", app_data_path, e);
                    error!("[SETUP] {}", error_msg);
                    window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                    return Err(error_msg);
                }
                info!("[SETUP] Created app data directory at {:?}", app_data_path);
            } else {
                info!("[SETUP] App data directory verified at {:?}", app_data_path);
            }
            window.emit("initialization-status", json!({ "status": "progress", "stage": "VerifyingAppDataDir", "progress": 25, "message": "Verifying application data..." })).map_err(|e| e.to_string())?;
        }
        Err(e) => {
            let error_msg = format!("Failed to resolve application data directory path: {}", e);
            error!("[SETUP] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP] Check 1 completed in {:?}", check_start.elapsed());
    let check_2_start = std::time::Instant::now();

    let check_2_start = std::time::Instant::now();

    // Check 2: Check Python Executable Path
    info!("[SETUP] Check 2: Checking Python Executable Path...");
    let python_executable_path_result: Result<PathBuf, String> = {
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
        let exe_dir = exe_path.parent().ok_or_else(|| "Failed to get executable directory".to_string())?;
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR for python executable".to_string())?
                .join("target")
                .join("debug")
                .join("vendor")
                .join("python")
                .join("python.exe")
        } else {
            exe_dir.join("vendor").join("python").join("python.exe")
        }
        .canonicalize() // Resolve to absolute path to be sure
        .map_err(|e| format!("Failed to canonicalize python path: {}", e))
    };

    match python_executable_path_result {
        Ok(python_path) => {
            if python_path.exists() && python_path.is_file() {
                info!("[SETUP] Python executable path verified at {:?}", python_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingPythonPath", "progress": 50, "message": "Verifying Python environment..." })).map_err(|e| e.to_string())?;
            } else {
                let error_msg = format!("Python executable not found or is not a file at resolved path: {:?}", python_path);
                error!("[SETUP] {}", error_msg);
                window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to determine Python executable path: {}", e);
            error!("[SETUP] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP] Check 2 completed in {:?}", check_2_start.elapsed());
    let check_3_start = std::time::Instant::now();

    let check_3_start = std::time::Instant::now();

    // Check 3: Check ComfyUI Directory Path
    info!("[SETUP] Check 3: Checking ComfyUI Directory Path...");
    let comfyui_directory_path_result: Result<PathBuf, String> = {
        let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
        let exe_dir = exe_path.parent().ok_or_else(|| "Failed to get executable directory".to_string())?;
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or_else(|| "Failed to get parent of CARGO_MANIFEST_DIR for comfyui_dir".to_string())?
                .join("target")
                .join("debug")
                .join("vendor")
                .join("comfyui")
        } else {
            exe_dir.join("vendor").join("comfyui")
        }
        .canonicalize() // Resolve to absolute path
        .map_err(|e| format!("Failed to canonicalize comfyui path: {}", e))
    };

    match comfyui_directory_path_result {
        Ok(comfyui_path) => {
            if comfyui_path.exists() && comfyui_path.is_dir() {
                info!("[SETUP] ComfyUI directory path verified at {:?}", comfyui_path);
                window.emit("initialization-status", json!({ "status": "progress", "stage": "CheckingComfyUIPath", "progress": 75, "message": "Verifying ComfyUI components..." })).map_err(|e| e.to_string())?;
            } else {
                let error_msg = format!("ComfyUI directory not found or is not a directory at resolved path: {:?}", comfyui_path);
                error!("[SETUP] {}", error_msg);
                window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
                return Err(error_msg);
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to determine ComfyUI directory path: {}", e);
            error!("[SETUP] {}", error_msg);
            window.emit("initialization-status", json!({ "status": "error", "message": format!("Initialization failed: {}", error_msg) })).ok();
            return Err(error_msg);
        }
    }
    info!("[SETUP] Check 3 completed in {:?}", check_3_start.elapsed());
    info!("[SETUP] All initialization checks completed in {:?}", check_start.elapsed());

    // Send ready status
    info!("[SETUP] Emitting ready status...");
    let ready_emit_start = std::time::Instant::now();

    match window.emit("initialization-status", json!({
        "status": "ready",
        "message": "Initialization complete. Ready to proceed."
    })) {
        Ok(_) => {
            let elapsed = ready_emit_start.elapsed();
            info!("[SETUP] Successfully emitted ready status in {:?}", elapsed);
        },
        Err(e) => {
            error!("[SETUP] Error emitting ready status: {} (after {:?})", e, ready_emit_start.elapsed());
            return Err(format!("Failed to emit ready status: {}", e));
        }
    }

    let total_elapsed = start_time.elapsed();
    info!("[SETUP] Initialization status check complete in {:?}", total_elapsed);
    Ok(())
}

/// Start the application setup process
#[tauri::command]
pub async fn start_application_setup(window: Window) -> Result<(), String> {
    // Spawn the setup process in the background
    let win = window.clone();
    tauri::async_runtime::spawn(async move {
        // Run the setup process
        if let Err(e) = run_setup_process(win).await {
            error!("Setup process failed: {}", e);
            // Notify the frontend of the error
            if let Err(emit_err) = window.emit("setup-progress", json!({
                "phase": "error",
                "currentStep": "Setup error",
                "progress": 0,
                "detailMessage": "Failed to complete setup",
                "error": e
            })) {
                error!("Failed to emit setup error: {}", emit_err);
            }
        }
    });
    
    Ok(())
}

/// Retry the application setup process
#[tauri::command]
pub async fn retry_application_setup(window: Window) -> Result<(), String> {
    start_application_setup(window).await
}

/// The actual setup process implementation
async fn run_setup_process(window: Window) -> Result<(), String> {
    // 1. System Check Phase
    window.emit("setup-progress", json!({
        "phase": "checking",
        "currentStep": "Checking system requirements",
        "progress": 0,
        "detailMessage": "Verifying system compatibility..."
    })).map_err(|e| e.to_string())?;
    
    // Simulate some check work
    for i in 1..=5 {
        sleep(Duration::from_millis(300)).await;
        window.emit("setup-progress", json!({
            "phase": "checking",
            "currentStep": "Checking system requirements",
            "progress": i * 20,
            "detailMessage": format!("Verifying system compatibility... ({}%)", i * 20)
        })).map_err(|e| e.to_string())?;
    }
    
    // 2. ComfyUI Installation Phase
    window.emit("setup-progress", json!({
        "phase": "installing_comfyui",
        "currentStep": "Installing ComfyUI",
        "progress": 0,
        "detailMessage": "Preparing ComfyUI installation..."
    })).map_err(|e| e.to_string())?;
    
    // THIS IS WHERE YOUR ACTUAL COMFYUI INSTALLATION CODE WOULD GO
    // For now, we'll just simulate the progress
    for i in 1..=10 {
        sleep(Duration::from_millis(300)).await;
        window.emit("setup-progress", json!({
            "phase": "installing_comfyui",
            "currentStep": "Installing ComfyUI",
            "progress": i * 10,
            "detailMessage": format!("Installing ComfyUI... ({}%)", i * 10)
        })).map_err(|e| e.to_string())?;
    }
    
    // 3. Python Setup Phase
    window.emit("setup-progress", json!({
        "phase": "python_setup",
        "currentStep": "Setting up Python environment",
        "progress": 0,
        "detailMessage": "Preparing Python environment..."
    })).map_err(|e| e.to_string())?;
    
    // THIS IS WHERE YOUR ACTUAL PYTHON SETUP CODE WOULD GO
    // For now, we'll just simulate the progress
    for i in 1..=10 {
        sleep(Duration::from_millis(300)).await;
        window.emit("setup-progress", json!({
            "phase": "python_setup",
            "currentStep": "Setting up Python environment",
            "progress": i * 10,
            "detailMessage": format!("Installing Python dependencies... ({}%)", i * 10)
        })).map_err(|e| e.to_string())?;
    }
    
    // 4. Model Download Phase
    window.emit("setup-progress", json!({
        "phase": "downloading_models",
        "currentStep": "Downloading required models",
        "progress": 0,
        "detailMessage": "Preparing to download AI models..."
    })).map_err(|e| e.to_string())?;
    
    // Define the models we need to download
    let models = vec![
        ModelInfo {
            id: "sd-v1-5".to_string(),
            name: "Stable Diffusion v1.5".to_string(),
            progress: 0.0,
            status: ModelStatus::Queued.as_str().to_string(),
            error_message: None,
        },
        ModelInfo {
            id: "vae-model".to_string(),
            name: "VAE Model".to_string(),
            progress: 0.0,
            status: ModelStatus::Queued.as_str().to_string(),
            error_message: None,
        },
        ModelInfo {
            id: "lora-base".to_string(),
            name: "Character Base LoRA".to_string(),
            progress: 0.0,
            status: ModelStatus::Queued.as_str().to_string(),
            error_message: None,
        },
    ];
    
    // Send initial model list
    window.emit("model-download-status", json!({
        "models": models
    })).map_err(|e| e.to_string())?;
    
    // Simulate downloading each model
    let mut current_models = models;
    
    // Download first model
    current_models[0].status = ModelStatus::Downloading.as_str().to_string();
    window.emit("model-download-status", json!({
        "models": current_models
    })).map_err(|e| e.to_string())?;
    
    for i in 1..=10 {
        sleep(Duration::from_millis(500)).await;
        current_models[0].progress = i as f32 * 10.0;
        window.emit("model-download-status", json!({
            "models": current_models
        })).map_err(|e| e.to_string())?;
        
        window.emit("setup-progress", json!({
            "phase": "downloading_models",
            "currentStep": "Downloading Stable Diffusion v1.5",
            "progress": i * 10,
            "detailMessage": format!("Downloading Stable Diffusion v1.5... ({}%)", i * 10)
        })).map_err(|e| e.to_string())?;
    }
    
    // Mark first model as complete and start second model
    current_models[0].status = ModelStatus::Completed.as_str().to_string();
    current_models[0].progress = 100.0;
    current_models[1].status = ModelStatus::Downloading.as_str().to_string();
    window.emit("model-download-status", json!({
        "models": current_models
    })).map_err(|e| e.to_string())?;
    
    for i in 1..=10 {
        sleep(Duration::from_millis(300)).await;
        current_models[1].progress = i as f32 * 10.0;
        window.emit("model-download-status", json!({
            "models": current_models
        })).map_err(|e| e.to_string())?;
        
        window.emit("setup-progress", json!({
            "phase": "downloading_models",
            "currentStep": "Downloading VAE Model",
            "progress": i * 10,
            "detailMessage": format!("Downloading VAE Model... ({}%)", i * 10)
        })).map_err(|e| e.to_string())?;
    }
    
    // Mark second model as complete and start third model
    current_models[1].status = ModelStatus::Completed.as_str().to_string();
    current_models[1].progress = 100.0;
    current_models[2].status = ModelStatus::Downloading.as_str().to_string();
    window.emit("model-download-status", json!({
        "models": current_models
    })).map_err(|e| e.to_string())?;
    
    for i in 1..=10 {
        sleep(Duration::from_millis(200)).await;
        current_models[2].progress = i as f32 * 10.0;
        window.emit("model-download-status", json!({
            "models": current_models
        })).map_err(|e| e.to_string())?;
        
        window.emit("setup-progress", json!({
            "phase": "downloading_models",
            "currentStep": "Downloading Character Base LoRA",
            "progress": i * 10,
            "detailMessage": format!("Downloading Character Base LoRA... ({}%)", i * 10)
        })).map_err(|e| e.to_string())?;
    }
    
    // Mark all models as complete
    current_models[2].status = ModelStatus::Completed.as_str().to_string();
    current_models[2].progress = 100.0;
    window.emit("model-download-status", json!({
        "models": current_models
    })).map_err(|e| e.to_string())?;
    
    // 5. Finalizing Phase
    window.emit("setup-progress", json!({
        "phase": "finalizing",
        "currentStep": "Finalizing installation",
        "progress": 0,
        "detailMessage": "Completing the setup process..."
    })).map_err(|e| e.to_string())?;
    
    // Simulate finalization work
    for i in 1..=10 {
        sleep(Duration::from_millis(200)).await;
        window.emit("setup-progress", json!({
            "phase": "finalizing",
            "currentStep": "Finalizing installation",
            "progress": i * 10,
            "detailMessage": format!("Configuring application settings... ({}%)", i * 10)
        })).map_err(|e| e.to_string())?;
    }
    
    // 6. Complete Phase
    window.emit("setup-progress", json!({
        "phase": "complete",
        "currentStep": "Setup complete",
        "progress": 100,
        "detailMessage": "Metamorphosis is ready to use!"
    })).map_err(|e| e.to_string())?;
    
    Ok(())
}