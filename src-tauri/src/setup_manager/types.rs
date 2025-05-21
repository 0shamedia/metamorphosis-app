// metamorphosis-app/src-tauri/src/setup_manager/types.rs
use serde::Serialize;

// Unified Setup Progress Payload
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetupProgressPayload {
    pub phase: String,
    pub current_step: String,
    pub progress: u8, // 0-100
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// Setup phases (kept for reference, but string literals will be used in emit_setup_progress)
#[derive(Debug, Clone, serde::Serialize)]
pub enum SetupPhase {
    Checking,
    InstallingComfyui,
    PythonSetup,
    InstallingCustomNodes, // Added for custom node installation
    DownloadingModels,
    Finalizing,
    Complete,
    Error,
}

// Model download status (may become obsolete if model download is fully integrated into setup-progress)
#[derive(Debug, Clone, serde::Serialize)]
pub enum ModelStatus {
    Queued,
    Downloading,
    Verifying,
    Completed,
    Error,
}

// Model information (may become obsolete)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub progress: f32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum SetupStatusEvent {
    BackendFullyVerifiedAndReady,
    FullSetupRequired { reason: String },
}

// Payloads for Custom Node Cloning Events
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CustomNodePayload {
    pub node_name: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CustomNodeCloneFailedPayload {
    pub node_name: String,
    pub error: String,
}