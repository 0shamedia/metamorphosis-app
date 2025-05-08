use std::env;
use std::path::PathBuf; // Import PathBuf
use tauri::{Manager, Url}; // Import Url
use std::thread;
use std::time::Duration;

mod comfyui_sidecar; // Declare the modules
mod gpu_detection;
mod dependency_management;
mod setup; // Add our new setup module

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
      let setup_start = std::time::Instant::now();
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
                  
                  // Introduce a small delay to ensure loading.html is visible
                  thread::sleep(Duration::from_millis(300));
                  log::info!("[STARTUP] Delay complete, proceeding with navigation.");

                  let window_clone = window.clone(); // Clone window for use in async task
                  tauri::async_runtime::spawn(async move {
                      let navigation_start = std::time::Instant::now();
                      if cfg!(dev) {
                          // --- DEBUG MODE ---
                          if let Some(dev_url) = app_handle_clone.config().build.dev_url.clone() {
                              log::info!("[STARTUP_ASYNC] Navigating to Next.js dev server: {}", dev_url);
                              match window_clone.navigate(dev_url) { // Navigate using the tauri::Url directly
                                  Ok(_) => log::info!("[STARTUP_ASYNC] Navigation to dev server initiated in {:?}", navigation_start.elapsed()),
                                  Err(e) => log::error!("[STARTUP_ASYNC] Failed to navigate to dev server: {}", e),
                              }
                          } else {
                              log::error!("[STARTUP_ASYNC] dev_url is None in tauri.conf.json!");
                              // Optionally show an error to the user or fallback
                          }
                      } else {
                          // --- RELEASE MODE ---
                          // Parse the relative path string into a tauri::Url
                          let prod_path_str = "splash.html"; // Relative path for bundled asset
                          match Url::parse(prod_path_str) {
                              Ok(prod_url) => {
                                  log::info!("[STARTUP_ASYNC] Navigating to Next.js production path: {}", prod_url);
                                  match window_clone.navigate(prod_url) { // Navigate using the parsed tauri::Url
                                      Ok(_) => log::info!("[STARTUP_ASYNC] Navigation to production path initiated in {:?}", navigation_start.elapsed()),
                                      Err(e) => log::error!("[STARTUP_ASYNC] Failed to navigate to production path: {}", e),
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
                      log::info!("[STARTUP] Window size: {}x{}", size.width, size.height);
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
      setup::check_initialization_status,
      setup::start_application_setup, // This might become obsolete or change purpose
      setup::retry_application_setup, // This might become obsolete or change purpose
      // Register the new backend readiness command
      comfyui_sidecar::ensure_backend_ready
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