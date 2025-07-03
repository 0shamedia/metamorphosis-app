// --- WebSocket Message Types ---
export interface ComfyUIWebSocketMessageBase {
  type: string;
  data: any;
}

export interface ComfyUIStatusData {
  status: {
    exec_info: {
      queue_remaining: number;
    };
  };
  sid?: string; // Optional session ID
}
export interface ComfyUIStatusMessage extends ComfyUIWebSocketMessageBase {
  type: 'status';
  data: ComfyUIStatusData;
}

export interface ComfyUIExecutionStartData {
  prompt_id: string;
}
export interface ComfyUIExecutionStartMessage extends ComfyUIWebSocketMessageBase {
  type: 'execution_start';
  data: ComfyUIExecutionStartData;
}

export interface ComfyUIExecutionCachedData {
  nodes: string[];
  prompt_id: string;
}
export interface ComfyUIExecutionCachedMessage extends ComfyUIWebSocketMessageBase {
  type: 'execution_cached';
  data: ComfyUIExecutionCachedData;
}

export interface ComfyUIExecutingData {
  node: string | null; // Node ID, or null if end of current prompt execution
  prompt_id: string;
}
export interface ComfyUIExecutingMessage extends ComfyUIWebSocketMessageBase {
  type: 'executing';
  data: ComfyUIExecutingData;
}

export interface ComfyUIProgressData {
  value: number;
  max: number;
  node?: string; // Optional, KSampler often sends this
  prompt_id: string; // Added to align with other messages
}
export interface ComfyUIProgressMessage extends ComfyUIWebSocketMessageBase {
  type: 'progress';
  data: ComfyUIProgressData;
}

export interface ComfyUIImageOutput {
  filename: string;
  subfolder: string;
  type: 'output' | 'temp' | 'input';
}
export interface ComfyUIExecutedOutputNode {
  images: ComfyUIImageOutput[];
  [key: string]: any; // For other output types like text
}

// Specific output type for the SaveImage node (and potentially others)
export interface ComfyUISaveImageOutput {
  images: ComfyUIImageOutput[];
}

// Type guard to check for the SaveImage output structure
export function isSaveImageOutput(
  output: any,
): output is ComfyUISaveImageOutput {
  return output && Array.isArray(output.images);
}

export interface ComfyUIExecutedData {
  prompt_id: string;
  output: { [nodeId: string]: ComfyUIExecutedOutputNode } | ComfyUISaveImageOutput;
  node?: string; // Sometimes present, indicates the node that finished
}
export interface ComfyUIExecutedMessage extends ComfyUIWebSocketMessageBase {
  type: 'executed';
  data: ComfyUIExecutedData;
}

export interface ComfyUIExecutionErrorData {
  prompt_id: string;
  exception_message: string;
  exception_type: string;
  traceback: string[];
  node_id: string;
  node_type: string;
  // ... other error details
}
export interface ComfyUIExecutionErrorMessage extends ComfyUIWebSocketMessageBase {
  type: 'execution_error';
  data: ComfyUIExecutionErrorData;
}

export interface ComfyUIExecutionInterruptedData {
    prompt_id: string;
    // ... other interruption details
}
export interface ComfyUIExecutionInterruptedMessage extends ComfyUIWebSocketMessageBase {
    type: 'execution_interrupted';
    data: ComfyUIExecutionInterruptedData;
}


export type ComfyUIWebSocketMessage =
  | ComfyUIStatusMessage
  | ComfyUIExecutionStartMessage
  | ComfyUIExecutionCachedMessage
  | ComfyUIExecutingMessage
  | ComfyUIProgressMessage
  | ComfyUIExecutedMessage
  | ComfyUIExecutionErrorMessage
  | ComfyUIExecutionInterruptedMessage;

// Add any other shared types related to ComfyUI interactions here
// For example, types related to prompt payloads if they become complex enough
// or API response/request types if not co-located with the API client.

export interface ImageOption { // Copied from comfyuiService.ts, might need to be in a more global types file if used elsewhere
    id: string;
    url: string;
    alt: string;
    // Add other relevant properties if needed, e.g., filename, subfolder for re-use
    filename?: string;
    subfolder?: string;
    type?: 'output' | 'temp' | 'input';
seed?: string | number; // Added to match global ImageOption type
}

export interface ImageSaveResult {
  assetUrl: string;
  filePath: string;
}