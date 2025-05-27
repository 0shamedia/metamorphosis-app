// metamorphosis-app/src-tauri/src/setup_manager/model_config.rs

use serde::{Deserialize, Serialize};

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
}

// --- Model Definitions ---

pub fn get_core_models_list() -> Vec<ModelConfig> {
    vec![
        ModelConfig {
            id: "clip_vision_vit_h_14_laion2b".to_string(),
            name: "CLIP Vision ViT-H-14 Laion2B".to_string(),
            url: "https://huggingface.co/laion/CLIP-ViT-H-14-laion2B-s32B-b79K/resolve/main/model.safetensors".to_string(),
            target_subdir: "clip_vision".to_string(),
            target_filename: "CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(3_944_552_236),
        },
        ModelConfig {
            id: "ip_adapter_faceid_sdxl".to_string(),
            name: "IP-Adapter FaceID SDXL".to_string(),
            url: "https://huggingface.co/h94/IP-Adapter-FaceID/resolve/main/ip-adapter-faceid_sdxl.bin".to_string(),
            target_subdir: "ipadapter".to_string(),
            target_filename: "ip-adapter-faceid_sdxl.bin".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(1_071_149_741),
        },
        ModelConfig {
            id: "sdxl_vae".to_string(),
            name: "SDXL VAE".to_string(),
            url: "https://huggingface.co/stabilityai/sdxl-vae/resolve/main/sdxl_vae.safetensors".to_string(),
            target_subdir: "vae".to_string(),
            target_filename: "sdxl_vae.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(334_641_164),
        },
        ModelConfig {
            id: "nova_3dcg_xl_illustrious_v30_sdxl".to_string(), // Updated ID
            name: "Nova 3DCG XL Illustrious V3.0 SDXL".to_string(), // Updated name
            url: "https://store-na-phx-1.gofile.io/download/direct/0cf3cfd8-86ea-4b64-a1af-2136d8b57865/nova3DCGXL_illustriousV30.safetensors".to_string(), // New GoFile URL
            target_subdir: "checkpoints".to_string(),
            target_filename: "nova3DCGXL_illustriousV30.safetensors".to_string(), // New filename
            downloaded_filename: Some("nova3DCGXL_illustriousV30.safetensors".to_string()), // Explicitly set downloaded filename
            expected_size_bytes: Some(6_938_372_138), // Updated to actual downloaded size from GoFile
        },
        ModelConfig {
            id: "sam_vit_b_01ec64".to_string(),
            name: "SAM ViT-B".to_string(),
            url: "https://huggingface.co/datasets/Gourieff/ReActor/resolve/main/models/sams/sam_vit_b_01ec64.pth".to_string(),
            target_subdir: "sams".to_string(),
            target_filename: "sam_vit_b_01ec64.pth".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(375_042_383),
        },
        ModelConfig {
            id: "face_yolov8m".to_string(),
            name: "Face YOLOv8m".to_string(),
            url: "https://huggingface.co/datasets/Gourieff/ReActor/resolve/main/models/detection/bbox/face_yolov8m.pt".to_string(),
            target_subdir: "ultralytics/bbox".to_string(),
            target_filename: "face_yolov8m.pt".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(51_996_128),
        },
        ModelConfig {
            id: "hand_yolov8s".to_string(),
            name: "Hand YOLOv8s".to_string(),
            url: "https://huggingface.co/Bingsu/adetailer/resolve/main/hand_yolov8s.pt".to_string(),
            target_subdir: "ultralytics/bbox".to_string(),
            target_filename: "hand_yolov8s.pt".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(22_507_643),
        },
    ]
}
