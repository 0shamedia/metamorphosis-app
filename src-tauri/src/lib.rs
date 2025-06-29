use std::env;
use tauri::{App, Manager, Url}; // Import App, Manager, Url
use std::time::Duration;
use log4rs::config;
use log4rs::Handle;
use std::fs; // Import the fs module for file system operations
use std::sync::{Arc, Mutex};

mod gpu_detection;
mod setup;                 // This file re-exports from setup_manager
pub mod sidecar_manager;   // Declare the new top-level module
pub mod setup_manager;     // Declare the new top-level module
pub mod character;
pub mod process_manager;   // Declare the new process manager module

// Define a state to track the shutdown process
pub struct ShutdownState(pub Arc<Mutex<bool>>);
pub struct LoggingHandle(pub Handle);

fn init_logging(app: &mut App) -> Result<Handle, Box<dyn std::error::Error>> {
    let log_config_path = app.path().app_config_dir()?.join("log4rs.yaml");

    if !log_config_path.exists() {
        // If the config doesn't exist, create a default one.
        // This is useful for initial setup or if the config gets deleted.
        let logs_path = app.path().app_data_dir()?.join("logs");
        fs::create_dir_all(&logs_path)?;
        let log_file_path = logs_path.join("app.log");

        let default_config = format!(
            r#"
refresh_rate: 30 seconds
appenders:
  stdout:
    kind: console
    encoder:
      pattern: "{{d(%Y-%m-%d %H:%M:%S)}} [{{l}}] {{m}}{{n}}"
  file:
    kind: file
    path: "{}"
    encoder:
      pattern: "{{d(%Y-%m-%d %H:%M:%S)}} [{{l}}] [{{T}}] {{M}} - {{m}}{{n}}"
    append: true
root:
  level: info
  appenders:
    - stdout
    - file
"#,
            log_file_path.to_str().unwrap().replace('\\', "/")
        );
        fs::write(&log_config_path, default_config)?;
    }

    let config = config::load_config_file(&log_config_path, Default::default())?;
    let handle = log4rs::init_config(config)?;
    Ok(handle)
}

// Command to get the unified workflow template
#[tauri::command]
fn get_unified_workflow() -> Result<String, String> {
    Ok(include_str!("../../resources/workflows/Metamorphosis Workflow.json").to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let app_start_time = std::time::Instant::now();
  
  tauri::Builder::default()
    .plugin(tauri_plugin_fs::init()) // Initialize the FS plugin
    .plugin(tauri_plugin_shell::init()) // Initialize the Shell plugin
    .plugin(tauri_plugin_opener::init()) // Initialize the Opener plugin
    .manage(process_manager::ProcessManager::new()) // Add the process manager to the state
    .manage(ShutdownState(Arc::new(Mutex::new(false)))) // Add shutdown state
    .setup(move |app| {
        match init_logging(app) {
            Ok(handle) => {
                app.manage(LoggingHandle(handle));
            }
            Err(e) => {
                eprintln!("Failed to initialize logging: {}", e);
                // Exit or handle error appropriately
            }
        }
      log::info!("======= METAMORPHOSIS APPLICATION STARTUP =======");
      log::info!("Runtime Info: OS: {}, Arch: {}", std::env::consts::OS, std::env::consts::ARCH);
      log::info!("Start Time: {:?}", app_start_time);
      log::info!("===============================================");
      log::info!("[STARTUP] App setup started - elapsed: {:?}", app_start_time.elapsed());
      
      let app_handle_clone = app.handle().clone(); // Clone app_handle for async task

      // Log app paths for debugging
      log::info!("[STARTUP] OS: {}, Architecture: {}", std::env::consts::OS, std::env::consts::ARCH);
      log::info!("[STARTUP] Current executable: {:?}", std::env::current_exe());
      log::info!("[STARTUP] Current directory: {:?}", std::env::current_dir());
      
      // Log app data directory and ensure logs subdirectory exists
      match app.handle().path().app_data_dir() {
          Ok(app_data_path) => {
              log::info!("[STARTUP] App data directory: {}", app_data_path.display());
              let logs_path = app_data_path.join("logs");
              
              // Explicitly create the logs directory if it doesn't exist
              if !logs_path.exists() {
                  log::info!("[STARTUP] Logs directory does NOT exist at: {}. Attempting to create...", logs_path.display());
                  match fs::create_dir_all(&logs_path) {
                      Ok(_) => log::info!("[STARTUP] Successfully created logs directory at: {}", logs_path.display()),
                      Err(e) => log::error!("[STARTUP] Failed to create logs directory at {}: {}", logs_path.display(), e),
                  }
              } else {
                  log::info!("[STARTUP] Logs directory already exists at: {}", logs_path.display());
              }
          },
          Err(e) => log::error!("[STARTUP] Failed to get app data directory: {}", e),
      }

      // Make sure the main window is visible early
      let window_start = std::time::Instant::now();
      log::info!("[STARTUP] Attempting to show main window");
      
      if let Some(window) = app.get_webview_window("main") {
          match window.show() { // Show the window (displaying loading.html)
              Ok(_) => {
                  log::info!("[STARTUP] Main window shown with loading.html in {:?}", window_start.elapsed());
                  
                  let main_window_clone_for_nav = window.clone();
                  let app_handle_clone_for_nav = app_handle_clone.clone();

                  tauri::async_runtime::spawn(async move {
                      let delay_duration = Duration::from_millis(500);
                      log::info!("[STARTUP_ASYNC] Starting {:?} delay before navigation to allow loading.html to render.", delay_duration);
                      tokio::time::sleep(delay_duration).await; // Re-add or ensure this delay
                      log::info!("[STARTUP_ASYNC] Delay complete. Proceeding with navigation.");

                      let navigation_start = std::time::Instant::now();
                      log::info!("[STARTUP_ASYNC] Initiating navigation logic at {:?}", navigation_start);
                      if cfg!(dev) {
                          // --- DEBUG MODE ---
                          if let Some(dev_url) = app_handle_clone_for_nav.config().build.dev_url.clone() {
                              // dev_url is already a tauri::Url, no need to parse
                              log::info!("[STARTUP_ASYNC] Navigating to Next.js dev server: {}", dev_url);
                              let url_to_log = dev_url.clone(); // Clone for logging if needed after move
                              match main_window_clone_for_nav.navigate(dev_url) { // dev_url is moved here
                                  Ok(_) => log::info!("[STARTUP_ASYNC] Navigation to dev server {} initiated in {:?}", url_to_log, navigation_start.elapsed()),
                                  Err(e) => log::error!("[STARTUP_ASYNC] Failed to navigate to dev server {}: {}", url_to_log, e),
                              }
                          } else {
                              log::error!("[STARTUP_ASYNC] dev_url is None in tauri.conf.json!");
                          }
                      } else {
                          // --- RELEASE MODE ---
                          // For Next.js, navigating to "/splash" should work if routing is set up.
                          // If splash.html is a static export at the root, "splash.html" is correct.
                          // Using "/splash" as it's more standard for Next.js SPA routing.
                          let prod_path_str = "/splash";
                          match Url::parse(prod_path_str) {
                              Ok(prod_url) => {
                                  log::info!("[STARTUP_ASYNC] Navigating to Next.js production path: {}", prod_url);
                                  let url_to_log = prod_url.clone(); // Clone for logging
                                  match main_window_clone_for_nav.navigate(prod_url) {
                                      Ok(_) => log::info!("[STARTUP_ASYNC] Navigation to production path {} initiated in {:?}", url_to_log, navigation_start.elapsed()),
                                      Err(e) => log::error!("[STARTUP_ASYNC] Failed to navigate to production path {}: {}", url_to_log, e),
                                  }
                              }
                              Err(e) => {
                                  log::error!("[STARTUP_ASYNC] Failed to parse production path string '{}' into Url: {}", prod_path_str, e);
                              }
                          }
                      }
                  });

                  // Log window properties
                  if let Ok(size) = window.inner_size() {
                      log::info!("[STARTUP] Window size after setup: {}x{}", size.width, size.height);
                  }
              },
              Err(e) => {
                  log::error!("[STARTUP] Failed to show window: {}", e);
              }
          }
      } else {
          log::error!("[STARTUP] Main window not found");
      }

      // Sidecar process will be started explicitly by the frontend later
      // log::info!("[STARTUP] Starting ComfyUI sidecar process");
      // comfyui_sidecar::start_comfyui_sidecar(app.handle().clone());
      log::info!("[STARTUP] Setup complete - elapsed: {:?}", app_start_time.elapsed());

      Ok(())
    })
    .plugin(tauri_plugin_http::init()) // Register the HTTP plugin
    .invoke_handler(tauri::generate_handler![
      // Register our setup commands
      setup_manager::verification::check_initialization_status,
      setup_manager::orchestration::start_application_setup,
      setup_manager::orchestration::retry_application_setup,
      setup_manager::orchestration::get_setup_status_and_initialize,
      // Register the new backend readiness command
      sidecar_manager::orchestration::ensure_backend_ready,
      sidecar_manager::orchestration::ensure_comfyui_running_and_healthy,
      // Register the new unified workflow command
      get_unified_workflow,
      character::character_generator::generate_character,
      character::character_generator::save_image_to_disk
    ])
    .on_window_event(move |window, event| {
        match event {
            tauri::WindowEvent::Destroyed => {
                if window.label() == "main" {
                    log::info!("[LIFECYCLE] Main window destroyed. Logger will shut down with app state.");
                    // The logger handle in the app's state will be dropped when the app shuts down,
                    // which will gracefully flush and close the logger. No explicit call needed.
                }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    let shutdown_state = window.state::<ShutdownState>();
                    let mut is_shutting_down = shutdown_state.0.lock().unwrap();

                    if *is_shutting_down {
                        // Shutdown is already in progress, allow the window to close.
                        log::info!("[LIFECYCLE] Shutdown already initiated, allowing window to close now.");
                        return; // This allows the default close to proceed, breaking the loop.
                    }

                    // First time close is requested, start the shutdown process.
                    *is_shutting_down = true;
                    log::info!("[LIFECYCLE] Main window close requested. Starting graceful shutdown.");
                    api.prevent_close();

                    let app_handle = window.app_handle().clone();
                    let window_clone = window.clone();

                    tauri::async_runtime::spawn(async move {
                        {
                            let process_manager = app_handle.state::<process_manager::ProcessManager>();
                            process_manager.shutdown_all_processes(&app_handle).await;
                        }
                        log::info!("[LIFECYCLE] All managed processes stopped. Requesting final window close.");
                        
                        // This will re-trigger CloseRequested, but the flag will be set, so it will fall through.
                        if let Err(e) = window_clone.close() {
                            log::error!("[LIFECYCLE] Failed to close window after cleanup: {}", e);
                        }
                    });
                }
            }
            _ => {}
        }
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(move |_app_handle, event| match event {
        tauri::RunEvent::Exit => {
            log::info!("[LIFECYCLE] Application exiting - total runtime: {:?}", app_start_time.elapsed());
            // Logger is handled by the managed state's Drop implementation.
        }
        tauri::RunEvent::Ready => {
            log::info!("[LIFECYCLE] Application ready - startup time: {:?}", app_start_time.elapsed());
        }
        _ => {}
    });
  
}