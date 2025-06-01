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
            id: "clip_vision_vit_h_14_laion2b".to_string(),
            name: "CLIP Vision ViT-H-14 Laion2B".to_string(),
            url: "https://huggingface.co/h94/IP-Adapter/resolve/main/models/image_encoder/model.safetensors".to_string(),
            target_subdir: "clip_vision".to_string(),
            target_filename: "CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors".to_string(),
            downloaded_filename: Some("model.safetensors".to_string()),
            expected_size_bytes: Some(2_528_373_448),
            model_type: ModelType::CLIPVision,
            is_essential: true,
        },
        ModelConfig {
            id: "ip_adapter_faceid_plusv2_sdxl".to_string(),
            name: "IP-Adapter FaceID Plus V2 SDXL".to_string(),
            url: "https://huggingface.co/h94/IP-Adapter-FaceID/resolve/main/ip-adapter-faceid-plusv2_sdxl.bin".to_string(),
            target_subdir: "ipadapter".to_string(),
            target_filename: "ip-adapter-faceid-plusv2_sdxl.bin".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::IPAdapter,
            is_essential: true,
        },
        ModelConfig {
            id: "lora_ip_adapter_faceid_plusv2_sdxl".to_string(),
            name: "LoRA for IP-Adapter FaceID Plus V2 SDXL".to_string(),
            url: "https://huggingface.co/h94/IP-Adapter-FaceID/resolve/main/ip-adapter-faceid-plusv2_sdxl_lora.safetensors".to_string(),
            target_subdir: "loras".to_string(),
            target_filename: "ip-adapter-faceid-plusv2_sdxl_lora.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::LoRA,
            is_essential: true,
        },
        ModelConfig {
            id: "ip_adapter_plus_sdxl_vit_h".to_string(),
            name: "IP-Adapter Plus SDXL ViT-H".to_string(),
            url: "https://huggingface.co/h94/IP-Adapter/resolve/main/sdxl_models/ip-adapter-plus_sdxl_vit-h.safetensors".to_string(),
            target_subdir: "ipadapter".to_string(),
            target_filename: "ip-adapter-plus_sdxl_vit-h.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None,
            model_type: ModelType::IPAdapter,
            is_essential: true,
        },
        ModelConfig {
            id: "sdxl_vae".to_string(),
            name: "SDXL VAE".to_string(),
            url: "https://huggingface.co/stabilityai/sdxl-vae/resolve/main/sdxl_vae.safetensors".to_string(),
            target_subdir: "vae".to_string(),
            target_filename: "sdxl_vae.safetensors".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(334_641_164),
            model_type: ModelType::VAE,
            is_essential: true,
        },
        ModelConfig {
            id: "sam_vit_b_01ec64".to_string(), // ID kept for consistency, but model is ViT-H
            name: "SAM ViT-H".to_string(), // Name updated to reflect ViT-H
            url: "https://dl.fbaipublicfiles.com/segment_anything/sam_vit_h_4b8939.pth".to_string(),
            target_subdir: "sams".to_string(),
            target_filename: "sam_vit_h_4b8939.pth".to_string(),
            downloaded_filename: None,
            expected_size_bytes: None, // Size for sam_vit_h unknown
            model_type: ModelType::SAM,
            is_essential: true,
        },
        ModelConfig {
            id: "face_yolov8m_ultralytics".to_string(),
            name: "Face YOLOv8m (Ultralytics)".to_string(),
            url: "https://huggingface.co/datasets/nilor-corp/models/resolve/main/ultralytics/bbox/face_yolov8m.pt".to_string(),
            target_subdir: "ultralytics/bbox".to_string(),
            target_filename: "face_yolov8m.pt".to_string(),
            downloaded_filename: Some("face_yolov8m.pt".to_string()),
            expected_size_bytes: None,
            model_type: ModelType::Ultralytics,
            is_essential: true,
        },

        // --- InstantID Models ---
        // antelopev2.zip (replaces individual buffalo_l and glintr100.onnx)
        ModelConfig {
            id: "instantid_antelopev2_archive".to_string(),
            name: "InsightFace AntelopeV2 Package Archive".to_string(),
            url: "https://huggingface.co/MonsterMMORPG/tools/resolve/main/antelopev2.zip".to_string(),
            target_subdir: "insightface/models/".to_string(), // Zip downloaded here
            target_filename: "antelopev2.zip".to_string(),
            downloaded_filename: None, // Will be antelopev2.zip by default
            expected_size_bytes: None, // Add if known, helps with integrity checks
            model_type: ModelType::Archive,
            is_essential: true,
        },

        // Commented out buffalo_l models as they are part of antelopev2.zip
        // ModelConfig {
        //     id: "insightface_buffalo_l_det_10g".to_string(),
        //     name: "InsightFace Buffalo_L Detection".to_string(),
        //     url: "https://huggingface.co/public-data/insightface/resolve/main/models/buffalo_l/det_10g.onnx".to_string(),
        //     target_subdir: "models/insightface".to_string(), // Original target
        //     target_filename: "det_10g.onnx".to_string(),
        //     downloaded_filename: None,
        //     expected_size_bytes: None,
        //     model_type: ModelType::InsightFace,
        //     is_essential: true,
        // },
        // ModelConfig {
        //     id: "insightface_buffalo_l_w600k_r50".to_string(),
        //     name: "InsightFace Buffalo_L Recognition".to_string(),
        //     url: "https://huggingface.co/public-data/insightface/resolve/main/models/buffalo_l/w600k_r50.onnx".to_string(),
        //     target_subdir: "models/insightface".to_string(), // Original target
        //     target_filename: "w600k_r50.onnx".to_string(),
        //     downloaded_filename: None,
        //     expected_size_bytes: None,
        //     model_type: ModelType::InsightFace,
        //     is_essential: true,
        // },
        // ModelConfig {
        //     id: "insightface_buffalo_l_genderage".to_string(),
        //     name: "InsightFace Buffalo_L Gender/Age".to_string(),
        //     url: "https://huggingface.co/public-data/insightface/resolve/main/models/buffalo_l/genderage.onnx".to_string(),
        //     target_subdir: "models/insightface".to_string(), // Original target
        //     target_filename: "genderage.onnx".to_string(),
        //     downloaded_filename: None,
        //     expected_size_bytes: None,
        //     model_type: ModelType::InsightFace,
        //     is_essential: true,
        // },
        // ModelConfig {
        //     id: "insightface_buffalo_l_2d106det".to_string(),
        //     name: "InsightFace Buffalo_L 2D Landmarks".to_string(),
        //     url: "https://huggingface.co/public-data/insightface/resolve/main/models/buffalo_l/2d106det.onnx".to_string(),
        //     target_subdir: "models/insightface".to_string(), // Original target
        //     target_filename: "2d106det.onnx".to_string(),
        //     downloaded_filename: None,
        //     expected_size_bytes: None,
        //     model_type: ModelType::InsightFace,
        //     is_essential: true,
        // },
        // ModelConfig {
        //     id: "insightface_buffalo_l_1k3d68".to_string(),
        //     name: "InsightFace Buffalo_L 3D Keypoints".to_string(),
        //     url: "https://huggingface.co/public-data/insightface/resolve/main/models/buffalo_l/1k3d68.onnx".to_string(),
        //     target_subdir: "models/insightface".to_string(), // Original target
        //     target_filename: "1k3d68.onnx".to_string(),
        //     downloaded_filename: None,
        //     expected_size_bytes: None,
        //     model_type: ModelType::InsightFace,
        //     is_essential: true,
        // },
        // Commented out standalone glintr100.onnx as it's part of antelopev2.zip
        // ModelConfig {
        //     id: "instantid_glintr100_onnx".to_string(),
        //     name: "InstantID InsightFace GlinTR ONNX".to_string(),
        //     url: "https://huggingface.co/MonsterMMORPG/tools/resolve/main/glintr100.onnx?download=true".to_string(),
        //     target_subdir: "insightface".to_string(), // Original target
        //     target_filename: "glintr100.onnx".to_string(),
        //     downloaded_filename: None,
        //     expected_size_bytes: None,
        //     model_type: ModelType::InsightFace,
        //     is_essential: true,
        // },
        ModelConfig {
            id: "instantid_inswapper_128_onnx".to_string(),
            name: "InstantID Inswapper ONNX".to_string(),
            url: "https://huggingface.co/ezioruan/inswapper_128.onnx/resolve/main/inswapper_128.onnx?download=true".to_string(),
            target_subdir: "insightface/".to_string(), // This is a separate model, not part of antelopev2
            target_filename: "inswapper_128.onnx".to_string(),
            downloaded_filename: None,
            expected_size_bytes: Some(554_253_681),
            model_type: ModelType::InsightFace,
            is_essential: true,
        },
        ModelConfig {
            id: "instantid_controlnet_sdxl".to_string(),
            name: "InstantID ControlNet SDXL".to_string(),
            url: "https://huggingface.co/InstantX/InstantID/resolve/main/ControlNetModel/diffusion_pytorch_model.safetensors".to_string(),
            target_subdir: "controlnet".to_string(),
            target_filename: "control_instantid_sdxl.safetensors".to_string(),
            downloaded_filename: Some("diffusion_pytorch_model.safetensors".to_string()),
            expected_size_bytes: None,
            model_type: ModelType::ControlNet,
            is_essential: true,
        },
        ModelConfig {
            id: "instantid_ipadapter_sdxl_bin".to_string(),
            name: "InstantID IP-Adapter SDXL".to_string(),
            url: "https://huggingface.co/InstantX/InstantID/resolve/main/ip-adapter.bin".to_string(),
            target_subdir: "instantid".to_string(),
            target_filename: "ip-adapter_instant_id_sdxl.bin".to_string(),
            downloaded_filename: Some("ip-adapter.bin".to_string()),
            expected_size_bytes: None,
            model_type: ModelType::IPAdapter,
            is_essential: true,
        },

        ModelConfig {
            id: "controlnet_union_sdxl_promax".to_string(),
            name: "Union SDXL ControlNet ProMax".to_string(),
            url: "https://huggingface.co/xinsir/controlnet-union-sdxl-1.0/resolve/main/diffusion_pytorch_model_promax.safetensors?download=true".to_string(),
            target_subdir: "controlnet/".to_string(),
            target_filename: "control_union_sdxl_promax.safetensors".to_string(),
            downloaded_filename: Some("diffusion_pytorch_model_promax.safetensors".to_string()),
            model_type: ModelType::ControlNet,
            expected_size_bytes: None,
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
        // ICLight Models (kept commented as per original, assuming not currently needed)
        // ...
    ]
}
