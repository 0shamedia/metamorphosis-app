export interface ModelDownloadProgressPayload {
  modelName: string;
  downloadedBytes: number;
  totalBytes: number | null;
  progress: number; // Percentage 0-100
}

export interface ModelDownloadCompletePayload {
  modelName: string;
  path: string;
  durationSeconds: number;
}

export interface ModelDownloadFailedPayload {
  modelName: string;
  error: string;
}

export interface OverallModelDownloadProgressPayload {
  completedModels: number;
  totalModels: number;
  progress: number; // Percentage 0-100
}
export interface CustomNodeCloneStartPayload {
  nodeName: string;
}

export interface CustomNodeCloneSuccessPayload {
  nodeName: string;
}

export interface CustomNodeCloneFailedPayload {
  nodeName: string;
  error: string;
}

export interface CustomNodeAlreadyExistsPayload {
  nodeName: string;
}

export interface PythonVersionDetectedPayload {
  version: string;
}

export interface InsightfaceWheelDownloadStartPayload {
  url: string;
}

export interface InsightfaceWheelDownloadProgressPayload {
  downloaded: number; // u64 can be represented as number in TS if not too large, otherwise string
  total?: number; // Option&lt;u64&gt;
}

// InsightfaceWheelDownloadComplete has no payload, so no interface needed unless we want an empty one for consistency.
// For event handling, often the event name itself is enough. If a payload structure is strictly expected, an empty interface can be used.
// export interface InsightfaceWheelDownloadCompletePayload {}

export interface PackageInstallStartPayload {
  packageName: string;
  method: string;
}

export interface PackageInstallSuccessPayload {
  packageName: string;
}

export interface PackageInstallFailedPayload {
  packageName: string;
  error: string;
  osHint?: string; // Option&lt;string&gt;
}

export interface PackageAlreadyInstalledPayload {
  packageName: string;
}

export interface PipUpdateStartPayload {
  // Potentially add a message if the backend sends one
}

export interface PipUpdateSuccessPayload {
  // Potentially add a message or details
}

export interface PipUpdateFailedPayload {
  error: string;
}

// It's also good practice to define a type for the event names themselves,
// and a generic event payload type, or a discriminated union if all events are handled similarly.
// For now, focusing on individual payload interfaces as requested.

// General Setup/Installation Events (from previous context, might be relevant)
export interface InstallationStatusPayload {
  step: string; // e.g., "PythonDownload", "DependencyInstall"
  status: 'InProgress' | 'Success' | 'Failed';
  progress?: number; // 0-100
  message?: string;
  error?: string;
}

export interface BackendStatusPayload {
  status: 'initializing' | 'dependencies_missing' | 'dependencies_installing' | 'sidecar_starting' | 'sidecar_spawned_checking_health' | 'backend_ready' | 'backend_error' | 'comfyui_healthy';
  message?: string;
  error?: string;
  gpuInfo?: {
    detected: boolean;
    driverVersion?: string;
    cudaVersion?: string;
    cudnnVersion?: string;
    deviceName?: string;
  };
  pythonVersion?: string;
  diskSpace?: {
    totalBytes: number;
    freeBytes: number;
  };
  comfyuiDir?: string;
  venvDir?: string;
  isFirstRun?: boolean;
}

export type SetupStatusType =
  | 'InitialCheck'
  | 'FullSetupRequired'
  | 'QuickVerificationRequired'
  | 'BackendFullyVerifiedAndReady'
  | 'Error';

export interface SetupStatusPayload {
  type: SetupStatusType;
  data?: {
    message?: string;
    error?: string;
    // Add other relevant data fields as needed
    pythonVersion?: string;
    comfyuiDir?: string;
    venvDir?: string;
    gpuInfo?: BackendStatusPayload['gpuInfo']; // Re-use from BackendStatusPayload
    diskSpace?: BackendStatusPayload['diskSpace']; // Re-use from BackendStatusPayload
    isFirstRun?: boolean;
  };
}