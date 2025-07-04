use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::character_creation_types::{CharacterGenerationState, GenerationMode};
use tauri::AppHandle;
use tauri::Manager;
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use chrono;


#[derive(Serialize, Deserialize, Debug)]
pub struct GenerationResponse {
    prompt_id: String,
    client_id: String,
}


#[tauri::command]
pub async fn generate_character(
    state: CharacterGenerationState,
) -> Result<GenerationResponse, String> {
    let mut workflow: Value = serde_json::from_str(&state.workflow_json).map_err(|e| e.to_string())?;

    update_workflow_json(&mut workflow, &state)?;

    let client_id = uuid::Uuid::new_v4().to_string();
    let payload = json!({
        "prompt": workflow,
        "client_id": client_id.clone(),
    });

    let client = reqwest::Client::new();
    let res = client.post("http://127.0.0.1:8188/prompt")
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        let response_body: Value = res.json().await.map_err(|e| e.to_string())?;
        let prompt_id = response_body["prompt_id"].as_str().ok_or("Missing prompt_id in response")?.to_string();
        
        Ok(GenerationResponse {
            prompt_id,
            client_id,
        })
    } else {
        let error_body = res.text().await.map_err(|e| e.to_string())?;
        Err(format!("Failed to queue prompt: {}", error_body))
    }
}

fn replace_template_placeholders(workflow: &mut Value, state: &CharacterGenerationState) -> Result<(), String> {
    // Log the character prompt being used for this generation
    println!("[PROMPT] Character attributes: {}", state.positive_prompt);
    println!("[PROMPT] Generation mode: {:?}", state.generation_mode);
    
    // Recursively traverse the workflow JSON to find and replace template placeholders
    replace_placeholders_recursive(workflow, state);
    Ok(())
}

fn replace_placeholders_recursive(value: &mut Value, state: &CharacterGenerationState) {
    match value {
        Value::String(s) => {
            // Replace __DYNAMIC_PROMPT__ with the actual character prompt
            if s.contains("__DYNAMIC_PROMPT__") {
                *s = s.replace("__DYNAMIC_PROMPT__", &state.positive_prompt);
            }
            
            // Replace __BACKGROUND_PROMPT__ based on context
            if s.contains("__BACKGROUND_PROMPT__") {
                let background = match state.context.as_deref() {
                    Some("character_creation") => "simple gradient background, vignetting",
                    // Future: Add other contexts like "gameplay" with rich backgrounds
                    _ => "simple gradient background" // Default fallback
                };
                *s = s.replace("__BACKGROUND_PROMPT__", background);
            }
            
            // Future: Add more placeholder replacements here for tag system
        }
        Value::Array(arr) => {
            for item in arr {
                replace_placeholders_recursive(item, state);
            }
        }
        Value::Object(obj) => {
            for (_, v) in obj {
                replace_placeholders_recursive(v, state);
            }
        }
        _ => {} // Numbers, booleans, null don't need replacement
    }
}

fn update_workflow_json(workflow: &mut Value, state: &CharacterGenerationState) -> Result<(), String> {
    // First, replace template placeholders in the workflow JSON
    replace_template_placeholders(workflow, state)?;
    
    // Determine the integer value for the switch based on the generation mode
    let switch_value = match state.generation_mode {
        GenerationMode::FaceFromPrompt => 1,
        GenerationMode::RegenerateFace => 2,
        GenerationMode::BodyFromPrompt => 3,
        GenerationMode::RegenerateBody => 4,
        GenerationMode::ClothingFromPrompt => 5,
    };

    // Update the 'select' input of the ImpactSwitch node (ID 95)
    update_node_input(workflow, "95", "select", switch_value)?;

    // Update image inputs based on the mode (prompts now handled by template system)
    match state.generation_mode {
        GenerationMode::FaceFromPrompt => {
            // Template system handles all prompts - no overrides needed
        }
        GenerationMode::BodyFromPrompt | GenerationMode::RegenerateBody => {
            // Only update image filename - template system handles prompts
            if let Some(filename) = &state.base_face_image_filename {
                update_node_input(workflow, "303", "image", filename.clone())?;
            }
        }
        GenerationMode::RegenerateFace => {
            // Only update image filename - template system handles prompts
            if let Some(filename) = &state.base_face_image_filename {
                update_node_input(workflow, "303", "image", filename.clone())?;
            }
        }
        GenerationMode::ClothingFromPrompt => {
            // Only update image filename - template system handles prompts
            if let Some(filename) = &state.base_body_image_filename {
                update_node_input(workflow, "82", "image", filename.clone())?;
            }
        }
    }

    // Update shared KSampler settings
    let ksampler_nodes = ["8", "34", "67"];
    for node_id in &ksampler_nodes {
        if let Some(node) = workflow.get_mut(node_id) {
            if let Some(inputs) = node.get_mut("inputs") {
                inputs["seed"] = state.seed.into();
                inputs["steps"] = state.steps.into();
                inputs["cfg"] = state.cfg.into();
                inputs["sampler_name"] = state.sampler_name.clone().into();
                inputs["scheduler"] = state.scheduler.clone().into();
                inputs["denoise"] = state.denoise.into();
            }
        }
    }

    // Update SaveImage node (335) with custom filename and subfolder
    if let Some(save_node) = workflow.get_mut("335") {
        if let Some(inputs) = save_node.get_mut("inputs") {
            // Generate timestamp for unique filenames
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
            
            // Determine image type based on generation mode
            let image_type = match state.generation_mode {
                GenerationMode::FaceFromPrompt | GenerationMode::RegenerateFace => "face",
                GenerationMode::BodyFromPrompt | GenerationMode::RegenerateBody => "body",
                GenerationMode::ClothingFromPrompt => "clothing",
            };
            
            // Get character ID or use "unknown"
            let character_id = state.character_id.as_deref().unwrap_or("unknown");
            
            // Ensure output directory structure exists
            ensure_character_output_directories(character_id, image_type)?;
            
            // Create subfolder path with filename: characters/{characterId}/{imageType}/{characterId}_{imageType}_{seed}_{timestamp}
            let filename_with_path = format!("characters/{}/{}/{}_{}_{}_{}", 
                character_id,
                image_type,
                character_id,
                image_type,
                state.seed,
                timestamp
            );
            
            inputs["filename_prefix"] = filename_with_path.into();
        }
    }

    Ok(())
}

fn update_node_input<T: Into<Value>>(workflow: &mut Value, node_id: &str, input_name: &str, value: T) -> Result<(), String> {
    workflow
        .get_mut(node_id)
        .and_then(|node| node.get_mut("inputs"))
        .map(|inputs| inputs[input_name] = value.into())
        .ok_or_else(|| format!("Failed to update input '{}' for node '{}'", input_name, node_id))
}

fn ensure_character_output_directories(character_id: &str, image_type: &str) -> Result<(), String> {
    // Get ComfyUI output directory path using the same logic as get_image_as_data_url
    let comfyui_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Failed to get parent of CARGO_MANIFEST_DIR")?
            .join("target")
            .join("debug")
            .join("vendor")
            .join("comfyui")
    } else {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?;
        let target_dir = exe_dir.parent()
            .ok_or_else(|| format!("Failed to get target directory from executable path: {}", exe_dir.display()))?;
        target_dir.join("vendor").join("comfyui")
    };

    // Create the character-specific directory structure
    let character_output_dir = comfyui_dir
        .join("output")
        .join("characters")
        .join(character_id)
        .join(image_type);

    // Create directories recursively
    fs::create_dir_all(&character_output_dir)
        .map_err(|e| format!("Failed to create character output directory {}: {}", character_output_dir.display(), e))?;

    Ok(())
}

#[tauri::command]
pub fn get_asset_url(app: AppHandle, filename: String, subfolder: String) -> Result<String, String> {
    // Use the same path resolution logic as process_handler.rs
    let comfyui_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Failed to get parent of CARGO_MANIFEST_DIR")?
            .join("target")
            .join("debug")
            .join("vendor")
            .join("comfyui")
    } else {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?;
        let target_dir = exe_dir.parent()
            .ok_or_else(|| format!("Failed to get target directory from executable path: {}", exe_dir.display()))?;
        target_dir.join("vendor").join("comfyui")
    };

    // Build the path to the generated image
    let image_path = comfyui_dir
        .join("output")
        .join(subfolder)
        .join(&filename);

    // Verify the image file exists
    if !image_path.exists() {
        return Err(format!("Image not found at path: {}", image_path.display()));
    }

    // For Tauri v2, we need to return a proper file:// URL
    // that convertFileSrc can handle correctly
    
    // Get the absolute path
    let absolute_path = if image_path.is_absolute() {
        image_path.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?
            .join(&image_path)
    };
    
    // Convert to string and normalize slashes
    let path_str = absolute_path.to_string_lossy().to_string();
    
    // Create a proper file:// URL
    #[cfg(target_os = "windows")]
    {
        // Windows: file:///C:/path/to/file (three slashes)
        let normalized = path_str.replace('\\', "/");
        Ok(format!("file:///{}", normalized))
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Unix: file:///path/to/file
        Ok(format!("file://{}", path_str))
    }
}


#[tauri::command]
pub fn prepare_image_for_edit(app: AppHandle, permanent_path: String) -> Result<String, String> {
    // 1. Resolve the temporary directory path
    let temp_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("Metamorphosis/temp/");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // 2. Generate a unique filename for the temporary copy
    let unique_id = Uuid::new_v4().to_string();
    let extension = PathBuf::from(&permanent_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png")
        .to_string();
    let temp_filename = format!("edit_{}.{}", unique_id, extension);
    let temp_path = temp_dir.join(&temp_filename);

    // 3. Copy the file from the permanent path to the temporary path
    fs::copy(&permanent_path, &temp_path)
        .map_err(|e| format!("Failed to copy image to temp directory: {}", e))?;

    // 4. Return the filename of the temporary copy
    Ok(temp_filename)
}

#[tauri::command]
pub fn get_image_as_data_url(app: AppHandle, filename: String, subfolder: String) -> Result<String, String> {
    // Use the same path resolution logic as get_asset_url
    let comfyui_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("Failed to get parent of CARGO_MANIFEST_DIR")?
            .join("target")
            .join("debug")
            .join("vendor")
            .join("comfyui")
    } else {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| format!("Failed to get parent directory of executable: {}", exe_path.display()))?;
        let target_dir = exe_dir.parent()
            .ok_or_else(|| format!("Failed to get target directory from executable path: {}", exe_dir.display()))?;
        target_dir.join("vendor").join("comfyui")
    };

    // Build the path to the generated image
    let image_path = comfyui_dir
        .join("output")
        .join(subfolder)
        .join(&filename);

    // Verify the image file exists
    if !image_path.exists() {
        return Err(format!("Image not found at path: {}", image_path.display()));
    }


    // Read the image file
    let image_data = fs::read(&image_path)
        .map_err(|e| format!("Failed to read image file: {}", e))?;

    // Determine MIME type based on file extension
    let mime_type = match image_path.extension().and_then(|s| s.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "image/png", // Default to PNG
    };

    // Convert to base64
    let base64_data = general_purpose::STANDARD.encode(&image_data);
    
    // Return as data URL
    Ok(format!("data:{};base64,{}", mime_type, base64_data))
}
    
#[tauri::command]
pub fn get_unified_workflow() -> Result<String, String> {
        Ok(include_str!("../../../resources/workflows/Metamorphosis Workflow.json").to_string())
    }
        
        #[tauri::command]
        pub fn generate_uuid() -> String {
            Uuid::new_v4().to_string()
        }