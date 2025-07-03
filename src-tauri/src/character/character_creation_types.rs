use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum GenerationMode {
    FaceFromPrompt,
    BodyFromPrompt,
    RegenerateFace,
    RegenerateBody,
    ClothingFromPrompt,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CharacterGenerationState {
    pub workflow_json: String,
    pub generation_mode: GenerationMode,
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub seed: i64,
    pub steps: i64,
    pub cfg: f64,
    pub sampler_name: String,
    pub scheduler: String,
    pub denoise: f64,
    #[serde(default)]
    pub base_face_image_filename: Option<String>,
    #[serde(default)]
    pub base_body_image_filename: Option<String>,
}