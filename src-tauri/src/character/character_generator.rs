use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::character_creation_types::{CharacterGenerationState, GenerationMode};
use tauri::AppHandle;
use tauri::path::BaseDirectory;
use tauri::Manager;
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerationResponse {
    prompt_id: String,
    client_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageSaveResult {
    id: String,
    url: String,
    seed: i64,
    alt: String,
}

#[tauri::command]
pub async fn generate_character(app: AppHandle, state: CharacterGenerationState) -> Result<GenerationResponse, String> {
    let resource_path = app.path().resolve("../resources/workflows/Metamorphosis Workflow.json", BaseDirectory::Resource).expect("failed to resolve resource");

    let workflow_str = fs::read_to_string(&resource_path).map_err(|e| e.to_string())?;
    let mut workflow: Value = serde_json::from_str(&workflow_str).map_err(|e| e.to_string())?;

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

fn update_workflow_json(workflow: &mut Value, state: &CharacterGenerationState) -> Result<(), String> {
    // Determine the integer value for the switch based on the generation mode
    let switch_value = match state.generation_mode {
        GenerationMode::FaceFromPrompt => 1,
        GenerationMode::BodyFromPrompt => 2,
        GenerationMode::RegenerateFace => 3,
        GenerationMode::RegenerateBody => 4,
        GenerationMode::ClothingFromPrompt => 5,
    };

    // Update the 'select' input of the ImpactSwitch node (ID 95)
    update_node_input(workflow, "95", "select", switch_value)?;

    // Update prompts and image inputs based on the mode
    match state.generation_mode {
        GenerationMode::FaceFromPrompt => {
            update_node_input(workflow, "4", "text", state.positive_prompt.clone())?;
            update_node_input(workflow, "7", "text", state.negative_prompt.clone())?;
        }
        GenerationMode::BodyFromPrompt | GenerationMode::RegenerateBody => {
            if let Some(filename) = &state.base_face_image_filename {
                update_node_input(workflow, "303", "image", filename.clone())?;
            }
            update_node_input(workflow, "37", "text", state.positive_prompt.clone())?;
            update_node_input(workflow, "38", "text", state.negative_prompt.clone())?;
        }
        GenerationMode::RegenerateFace => {
             if let Some(filename) = &state.base_face_image_filename {
                update_node_input(workflow, "303", "image", filename.clone())?;
            }
        }
        GenerationMode::ClothingFromPrompt => {
            if let Some(filename) = &state.base_body_image_filename {
                update_node_input(workflow, "82", "image", filename.clone())?;
            }
            update_node_input(workflow, "65", "text", state.positive_prompt.clone())?;
            update_node_input(workflow, "70", "text", state.negative_prompt.clone())?;
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

    Ok(())
}

fn update_node_input<T: Into<Value>>(workflow: &mut Value, node_id: &str, input_name: &str, value: T) -> Result<(), String> {
    workflow
        .get_mut(node_id)
        .and_then(|node| node.get_mut("inputs"))
        .map(|inputs| inputs[input_name] = value.into())
        .ok_or_else(|| format!("Failed to update input '{}' for node '{}'", input_name, node_id))
}

#[tauri::command]
pub fn save_image_to_disk(
    app: AppHandle,
    base64_data: String,
    image_type: String,
    seed: i64,
    character_id: String,
) -> Result<ImageSaveResult, String> {
    let storage_path = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("Metamorphosis/character_renders/");

    fs::create_dir_all(&storage_path)
        .map_err(|e| format!("Failed to create character_renders directory: {}", e))?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let filename = format!("{}_{}_{}_{}.png", character_id, image_type, seed, timestamp);
    let full_path = storage_path.join(&filename);

    let image_bytes = general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("Failed to decode base64 image data: {}", e))?;

    fs::write(&full_path, &image_bytes)
        .map_err(|e| format!("Failed to write image to disk: {}", e))?;

    let url = tauri::Url::from_file_path(&full_path).unwrap().to_string();

    Ok(ImageSaveResult {
        id: Uuid::new_v4().to_string(),
        url,
        seed,
        alt: format!("Generated {} image with seed {}", image_type, seed),
    })
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