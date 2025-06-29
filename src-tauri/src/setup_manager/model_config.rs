// metamorphosis-app/src-tauri/src/setup_manager/model_config.rs

use serde::{Deserialize, Serialize};
use super::types::ModelType; // Import ModelType

// --- Configuration Structures ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub id: String, // Unique identifier for the model
    pub name: String, // User-friendly name
    pub url: String,
    pub target_subdir: String, // e.g., "clip_vision", "ipadapter"
    pub target_filename: String, // Final filename in the target directory
    #[serde(default)] // If downloaded_filename is not present in config, it's same as target_filename
    pub downloaded_filename: Option<String>, // Filename as it's downloaded (if different from target)
    #[serde(default)]
    pub expected_size_bytes: Option<u64>, // Optional: for a more robust check
    #[serde(default)]
    pub model_type: ModelType, // Type of the model, used for special handling like extraction
    #[serde(default = "default_is_essential")]
    pub is_essential: bool, // Whether the model is essential for core functionality
}

fn default_is_essential() -> bool {
    true // Default to true, can be overridden in specific model configs
}


// --- Model Definitions ---

pub fn get_core_models_list() -> Vec<ModelConfig> {
    vec![
        ModelConfig {
            id: "metamorphosis_v3".to_string(),
            name: "Metamorphosis V3".to_string(),
            url: "https://huggingface.co/0sha/Metamorphosis_v1/resolve/main/Metamorphosis_v3.safetensors".to_string(),
            target_subdir: "checkpoints".to_string(),
            target_filename: "Metamorphosis_v3.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(6_938_374_086),
            model_type: ModelType::Checkpoint,
            is_essential: true,
        },
        ModelConfig {
            id: "clipseg_weights_rd64_refined".to_string(),
            name: "CLIPSeg Weights (RD64-Refined)".to_string(),
            url: "https://huggingface.co/CIDAS/clipseg-rd64-refined/resolve/main/pytorch_model.bin".to_string(),
            target_subdir: "clipseg".to_string(), // Target directory for CLIPSeg models
            target_filename: "clipseg_weights.pth".to_string(), // Renamed for convention
            downloaded_filename: Some("pytorch_model.bin".to_string()),
            expected_size_bytes: None, // Add if known
            model_type: ModelType::Generic, // Or a more specific type if applicable
            is_essential: true, // Assuming it's essential for the user's workflow
        },
        ModelConfig {
            id: "upernet_global_small".to_string(),
            name: "Upernet Global Small".to_string(),
            url: "https://huggingface.co/lllyasviel/ControlNet/resolve/main/annotator/ckpts/upernet_global_small.pth".to_string(),
            target_subdir: "custom_nodes/comfyui_controlnet_aux/ckpts/lllyasviel/Annotators".to_string(),
            target_filename: "upernet_global_small.pth".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::Generic,
            is_essential: true,
        },
        ModelConfig {
            id: "control_lora_depth_rank128".to_string(),
            name: "Control-LoRA Depth Rank128".to_string(),
            url: "https://huggingface.co/stabilityai/control-lora/resolve/main/control-LoRAs-rank128/control-lora-depth-rank128.safetensors".to_string(),
            target_subdir: "controlnet".to_string(),
            target_filename: "control-lora-depth-rank128.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::LoRA,
            is_essential: true,
        },
        ModelConfig {
            id: "control_lora_openpose_rank256".to_string(),
            name: "Control-LoRA OpenPose Rank256".to_string(),
            url: "https://huggingface.co/thibaud/controlnet-openpose-sdxl-1.0/resolve/main/control-lora-openposeXL2-rank256.safetensors".to_string(),
            target_subdir: "controlnet".to_string(),
            target_filename: "control-lora-openposeXL2-rank256.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::LoRA,
            is_essential: true,
        },
        ModelConfig {
            id: "control_lora_canny_rank128".to_string(),
            name: "Control-LoRA Canny Rank128".to_string(),
            url: "https://huggingface.co/stabilityai/control-lora/resolve/main/control-LoRAs-rank128/control-lora-canny-rank128.safetensors".to_string(),
            target_subdir: "controlnet".to_string(),
            target_filename: "control-lora-canny-rank128.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::LoRA,
            is_essential: true,
        },
        // ICLight Models (kept commented as per original, assuming not currently needed)
        // ...
    ]
}
