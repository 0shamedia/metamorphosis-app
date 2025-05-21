use std::env;
use tauri::{Manager, Url}; // Import Url
use std::time::Duration;

mod comfyui_sidecar;       // This file re-exports from sidecar_manager
mod gpu_detection;
mod dependency_management;
mod setup;                 // This file re-exports from setup_manager
pub mod sidecar_manager;   // Declare the new top-level module
pub mod setup_manager;     // Declare the new top-level module

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let app_start_time = std::time::Instant::now();
  
  println!("======= METAMORPHOSIS APPLICATION STARTUP =======");
  println!("Runtime Info: OS: {}, Arch: {}", std::env::consts::OS, std::env::consts::ARCH);
  println!("Start Time: {:?}", app_start_time);
  println!("===============================================");
  
  tauri::Builder::default()
    .plugin(tauri_plugin_log::Builder::default().build()) // Initialize log plugin first
    .plugin(tauri_plugin_fs::init()) // Initialize the FS plugin
    .plugin(tauri_plugin_shell::init()) // Initialize the Shell plugin
    .plugin(tauri_plugin_opener::init()) // Initialize the Opener plugin
    .setup(move |app| {
      // Logging should be configured via the plugin initialization above
      log::info!("[STARTUP] App setup started - elapsed: {:?}", app_start_time.elapsed());
      
      let app_handle_clone = app.handle().clone(); // Clone app_handle for async task

      // Log app paths for debugging
      log::info!("[STARTUP] OS: {}, Architecture: {}", std::env::consts::OS, std::env::consts::ARCH);
      log::info!("[STARTUP] Current executable: {:?}", std::env::current_exe());
      log::info!("[STARTUP] Current directory: {:?}", std::env::current_dir());
      
      // Log app data directory
      match app.handle().path().app_data_dir() {
          Ok(path) => log::info!("[STARTUP] App data directory: {}", path.display()),
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
      sidecar_manager::orchestration::ensure_comfyui_running_and_healthy
    ])
    .on_window_event(|window, event| match event {
        tauri::WindowEvent::Destroyed => {
            // Ensure this only runs for the main window if multiple windows exist
            if window.label() == "main" { // Check label for main window
                log::info!("[LIFECYCLE] Main window destroyed, stopping ComfyUI sidecar");
                comfyui_sidecar::stop_comfyui_sidecar();
            }
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            if window.label() == "main" {
                log::info!("[LIFECYCLE] Main window close requested");
            }
        }
        _ => {}
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(move |_app_handle, event| match event { // Handle app exit events too
        tauri::RunEvent::ExitRequested { .. } => {
            log::info!("[LIFECYCLE] Exit requested, stopping ComfyUI sidecar");
            comfyui_sidecar::stop_comfyui_sidecar();
        }
        tauri::RunEvent::Exit => {
             log::info!("[LIFECYCLE] Application exiting - total runtime: {:?}", app_start_time.elapsed());
        }
        tauri::RunEvent::Ready => {
            log::info!("[LIFECYCLE] Application ready - startup time: {:?}", app_start_time.elapsed());
        }
        _ => {}
    });
  
  println!("Total initialization time: {:?}", app_start_time.elapsed());
}