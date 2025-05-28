import { v4 as uuidv4 } from 'uuid';
import { CharacterAttributes, Tag } from '../types/character';
import useCharacterStore from '../store/characterStore';
import { generateComfyUIPromptPayload } from './comfyui/promptBuilder';
import { queuePrompt, uploadImageToComfyUI as apiClientUploadImage } from './comfyui/apiClient';
import { createWebSocketManager } from './comfyui/webSocketManager';
// Re-export types from the new central types file if they are directly used by consumers of comfyuiService
// However, it's often better if consumers rely on the abstractions provided by this service or the store.
export * from './comfyui/types';

/**
 * Initiates image generation with ComfyUI, handling HTTP prompt submission and WebSocket progress.
 * This function now orchestrates calls to the new modularized services.
 */
export async function initiateImageGeneration(
  attributes: CharacterAttributes,
  tags: Tag[],
  workflowType: "face" | "fullbody", // This is the type expected by the UI/store
  selectedFaceFilename?: string | null,
  selectedFaceSubfolder?: string | null
): Promise<{ promptId: string; clientId: string; closeSocket: () => void } | null> {
  const {
    setClientId,
    setGenerationProgress,
    setIsGeneratingFace,
    setIsGeneratingFullBody,
    setError
    // setFaceOptions and setFullBodyOptions are now handled by webSocketManager via store updates
  } = useCharacterStore.getState();

  const clientId = uuidv4();
  setClientId(clientId); // Set client ID in the store

  // Determine the actual workflow type to be used with ComfyUI
  // The UI might send "fullbody", but we want to use "fullbody_detailer"
  const actualComfyUIWorkflowType = workflowType === "fullbody" ? "fullbody_detailer" : workflowType;

  if (actualComfyUIWorkflowType === "face") {
    setIsGeneratingFace(true);
  } else { // This covers "fullbody_detailer"
    setIsGeneratingFullBody(true);
  }
  setError(null); // Clear any previous errors
  setGenerationProgress({ // Initial progress state
    promptId: null,
    currentNodeId: null,
    currentNodeTitle: null,
    step: 0,
    maxSteps: 0,
    message: "Preparing workflow...",
    queuePosition: null,
    clientId: clientId, // Include clientId in the progress state
  });

  try {
    // 1. Generate the prompt payload using the promptBuilder
    const { workflowJson: promptPayload, outputNodeId } = await generateComfyUIPromptPayload(
      attributes,
      tags,
      actualComfyUIWorkflowType, // Use the mapped type for ComfyUI
      selectedFaceFilename,
      selectedFaceSubfolder
    );

    // 2. Queue the prompt using the apiClient
    const { prompt_id: promptId } = await queuePrompt(promptPayload, clientId);

// Extract the inputSeed
    let inputSeed: string | number | undefined = undefined;
    // Determine seedNodeId and seedInputName based on workflowType (similar logic to promptBuilder)
    let seedNodeId: string;
    const seedInputName = "seed"; // Assuming "seed" is the input name in the workflow JSON
    if (actualComfyUIWorkflowType === "face") {
      seedNodeId = "3"; // Matching promptBuilder.ts
    } else { // fullbody or fullbody_detailer
      seedNodeId = "5"; // Matching promptBuilder.ts
    }
    if (promptPayload[seedNodeId]?.inputs && promptPayload[seedNodeId].inputs[seedInputName]) {
      inputSeed = promptPayload[seedNodeId].inputs[seedInputName];
    } else {
      console.warn(`[ComfyService] Could not extract inputSeed from promptPayload for node ${seedNodeId}`);
    }
    // Update progress: workflow submitted
    setGenerationProgress({
        promptId: promptId,
        currentNodeId: null,
        currentNodeTitle: null,
        step: 0,
        maxSteps: 0,
        message: "Workflow submitted, waiting for execution...",
        queuePosition: null,
        clientId: clientId,
    });

    // 3. Create and manage WebSocket connection using webSocketManager
    // The webSocketManager will internally handle store updates for progress, images, and errors.
    const wsManager = createWebSocketManager({
      clientId,
      promptId,
      outputNodeId,
      workflowType: actualComfyUIWorkflowType, // Pass the ComfyUI-specific workflow type
      inputSeed: inputSeed, // Pass the extracted inputSeed
      // Callbacks for onImageGenerated, onGenerationComplete, onGenerationError
      // are now handled by the webSocketManager updating the Zustand store directly.
    });

    return {
      promptId,
      clientId,
      closeSocket: () => wsManager.close(), // Provide a way to close the WebSocket
    };

  } catch (error) {
    console.error('[ComfyService] Error initiating image generation:', error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    setError(`Failed to start generation: ${errorMessage}`);
    // Reset generation state
    if (actualComfyUIWorkflowType === "face") {
      setIsGeneratingFace(false);
    } else {
      setIsGeneratingFullBody(false);
    }
    setGenerationProgress(null);
    setClientId(null); // Clear client ID on failure
    return null;
  }
}

/**
 * Uploads an image to the ComfyUI /upload/image endpoint.
 * This function now acts as a simple wrapper around the apiClient's version.
 * @param imageFile The image file to upload.
 * @param overwrite Optional. Whether to overwrite an existing image with the same name.
 * @param subfolder Optional. The subfolder to upload the image to.
 * @param type Optional. The type of image (e.g., 'input', 'temp', 'mask').
 * @returns The response from the ComfyUI API.
 */
export async function uploadImageToComfyUI(
  imageFile: File,
  overwrite: boolean = false,
  subfolder?: string,
  type: 'input' | 'temp' | 'mask' = 'input'
): Promise<any> { // Consider defining a more specific return type based on actual API response
  console.log(`[ComfyService] Delegating image upload for "${imageFile.name}" to apiClient.`);
  try {
    return await apiClientUploadImage(imageFile, overwrite, subfolder, type);
  } catch (error) {
    console.error('[ComfyService] Error during image upload delegation:', error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    // Optionally, update a global error state here if needed, e.g., via Zustand store
    // useCharacterStore.getState().setError(`Image upload failed: ${errorMessage}`);
    throw error; // Re-throw the error to be handled by the caller
  }
}