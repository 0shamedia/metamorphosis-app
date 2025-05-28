import { fetch } from '@tauri-apps/plugin-http';
import { ComfyUIWebSocketMessage, ImageOption } from './types'; // Assuming ImageOption might be used or adapted

export const comfyuiApiUrl = 'http://127.0.0.1:8188';

export interface PromptQueueResponse {
  prompt_id: string;
  number: number;
  node_errors?: any;
}

/**
 * Submits a prompt to the ComfyUI API.
 * @param promptPayload The workflow JSON.
 * @param clientId The client ID for the WebSocket connection.
 * @returns The prompt ID and other queue information.
 */
export async function queuePrompt(
  promptPayload: any, // Consider defining a more specific type for promptPayload
  clientId: string
): Promise<PromptQueueResponse> {
  const httpResponse = await fetch(`${comfyuiApiUrl}/prompt`, {
    method: 'POST',
    body: JSON.stringify({ prompt: promptPayload, client_id: clientId }),
    headers: { 'Content-Type': 'application/json' },
  });

  if (!httpResponse.ok) {
    const errorBody = await httpResponse.text(); // Or httpResponse.json() if the error is JSON
    console.error(`ComfyUI API Error (queuePrompt): ${httpResponse.status} - ${httpResponse.statusText}`, errorBody);
    throw new Error(`ComfyUI API Error: ${httpResponse.status}. ${errorBody}`);
  }

  return await httpResponse.json() as PromptQueueResponse;
}


/**
 * Uploads an image to the ComfyUI /upload/image endpoint.
 * @param imageFile The image file to upload.
 * @param overwrite Optional. Whether to overwrite an existing image with the same name.
 * @param subfolder Optional. The subfolder to upload the image to.
 * @param type Optional. The type of image (e.g., 'input', 'temp').
 * @returns The response from the ComfyUI API.
 */
export interface ComfyApiImageUploadResponse {
  name: string;
  subfolder?: string;
  type?: string;
  // Potentially other fields ComfyUI might return
}

export interface UploadImageResult {
  filename: string;
  name: string; // Keep original name field as well
  subfolder?: string;
  type?: string;
}

export async function uploadImageToComfyUI(
  imageFile: File,
  overwrite: boolean = false,
  subfolder?: string,
  type: 'input' | 'temp' | 'mask' = 'input' // Added 'mask' as a common type
): Promise<UploadImageResult> {
  try {
    const formData = new FormData();
    formData.append('image', imageFile, imageFile.name);
    formData.append('overwrite', String(overwrite));
    if (subfolder) {
      formData.append('subfolder', subfolder);
    }
    formData.append('type', type);

    console.log(`[ApiClient] Uploading image "${imageFile.name}" to ComfyUI. Subfolder: ${subfolder}, Type: ${type}, Overwrite: ${overwrite}`);

    const response = await fetch(`${comfyuiApiUrl}/upload/image`, {
      method: 'POST',
      body: formData, // Pass FormData directly
      // Tauri's fetch with FormData usually sets the Content-Type header automatically.
    });

    if (!response.ok) {
      const errorText = await response.text(); // Or response.json() if error is JSON
      console.error(`[ApiClient] Error uploading image to ComfyUI: ${response.status} - ${response.statusText}`, errorText);
      throw new Error(`Failed to upload image: ${response.status} - ${errorText}`);
    }

    const responseData = await response.json() as ComfyApiImageUploadResponse;
    console.log('[ApiClient] Image uploaded successfully:', responseData);
    // Map the 'name' field to 'filename' for consistency with how it's used elsewhere
    return { ...responseData, filename: responseData.name };
  } catch (error) {
    console.error('[ApiClient] Exception during image upload:', error);
    throw error;
  }
}

/**
 * Fetches an image from the ComfyUI /view endpoint.
 * @param filename The name of the file.
 * @param subfolder The subfolder where the file is located.
 * @param type The type of the image (e.g., 'output', 'temp').
 * @returns A Blob representing the image.
 */
export async function getImageBlob(filename: string, subfolder: string, type: string): Promise<Blob> {
    const imageUrl = `${comfyuiApiUrl}/view?filename=${encodeURIComponent(filename)}&subfolder=${encodeURIComponent(subfolder || '')}&type=${type}`;
    // For binary data, fetch and then get ArrayBuffer
    const response = await fetch(imageUrl, { method: 'GET' });

    if (!response.ok) {
        throw new Error(`Failed to fetch image ${filename}: ${response.status} - ${response.statusText}`);
    }
    const arrayBuffer = await response.arrayBuffer();
    return new Blob([arrayBuffer]);
}

/**
 * Constructs a URL to view an image from ComfyUI.
 * @param filename The name of the file.
 * @param subfolder The subfolder where the file is located.
 * @param type The type of the image (e.g., 'output', 'temp', 'input').
 * @returns The full URL to view the image.
 */
export function getImageUrl(filename: string, subfolder: string, type: 'output' | 'temp' | 'input'): string {
    return `${comfyuiApiUrl}/view?filename=${encodeURIComponent(filename)}&subfolder=${encodeURIComponent(subfolder || '')}&type=${type}`;
}