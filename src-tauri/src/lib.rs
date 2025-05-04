use std::fs;
use std::io::prelude::*; // Import prelude for write_all
use std::env;
use tauri::{AppHandle, Manager, Wry};
use tauri::async_runtime::{self};
use std::path::PathBuf;
use log::{info, error};
mod comfyui_sidecar; // Declare the new module
mod gpu_detection;
mod dependency_management;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_log::Builder::default().build()) // Initialize log plugin first
    .plugin(tauri_plugin_fs::init()) // Initialize the FS plugin
    .plugin(tauri_plugin_shell::init()) // Initialize the Shell plugin
    .plugin(tauri_plugin_opener::init()) // Initialize the Opener plugin
    .setup(|app| {
      // Logging should be configured via the plugin initialization above
      // You can still log here if needed after initialization
      info!("App setup started.");
      match app.handle().path().app_data_dir() {
          Ok(path) => info!("App data directory: {}", path.display()),
          Err(e) => error!("Failed to get app data directory: {}", e),
      }

      // Start the sidecar process
      comfyui_sidecar::start_comfyui_sidecar(app.handle().clone());

      Ok(())
    })
    .plugin(tauri_plugin_http::init()) // Register the HTTP plugin
    .on_window_event(|window, event| match event {
        tauri::WindowEvent::Destroyed => {
            // Ensure this only runs for the main window if multiple windows exist
            if window.label() == "main" { // Check label for main window
                info!("Main window destroyed, stopping ComfyUI sidecar...");
                comfyui_sidecar::stop_comfyui_sidecar();
            }
        }
        _ => {}
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|_app_handle, event| match event { // Handle app exit events too
        tauri::RunEvent::ExitRequested { .. } => {
            info!("Exit requested, stopping ComfyUI sidecar...");
            comfyui_sidecar::stop_comfyui_sidecar();
            // Optionally prevent default exit and wait for cleanup
        }
        tauri::RunEvent::Exit => {
             info!("Application exiting.");
             // Sidecar should ideally be stopped by ExitRequested or WindowEvent::Destroyed
        }
        _ => {}
    });
}
