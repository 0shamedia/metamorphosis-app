import { fetch } from '@tauri-apps/plugin-http';
// import { resolveResource } from '@tauri-apps/api/path'; // No longer needed
// import { readTextFile } from '@tauri-apps/plugin-fs'; // No longer needed
import { invoke } from '@tauri-apps/api/core'; // For calling Tauri commands
import { v4 as uuidv4 } from 'uuid';
import { CharacterAttributes, Tag, ImageOption } from '../types/character'; // Assuming ImageOption is in types/character
import useCharacterStore from '../store/characterStore'; // Import the store

const comfyuiApiUrl = 'http://127.0.0.1:8188';
const comfyuiWsUrl = 'ws://127.0.0.1:8188/ws';

// --- WebSocket Message Types ---
interface ComfyUIWebSocketMessageBase {
  type: string;
  data: any;
}

interface ComfyUIStatusData {
  status: {
    exec_info: {
      queue_remaining: number;
    };
  };
  sid?: string; // Optional session ID
}
interface ComfyUIStatusMessage extends ComfyUIWebSocketMessageBase {
  type: 'status';
  data: ComfyUIStatusData;
}

interface ComfyUIExecutionStartData {
  prompt_id: string;
}
interface ComfyUIExecutionStartMessage extends ComfyUIWebSocketMessageBase {
  type: 'execution_start';
  data: ComfyUIExecutionStartData;
}

interface ComfyUIExecutionCachedData {
  nodes: string[];
  prompt_id: string;
}
interface ComfyUIExecutionCachedMessage extends ComfyUIWebSocketMessageBase {
  type: 'execution_cached';
  data: ComfyUIExecutionCachedData;
}

interface ComfyUIExecutingData {
  node: string | null; // Node ID, or null if end of current prompt execution
  prompt_id: string;
}
interface ComfyUIExecutingMessage extends ComfyUIWebSocketMessageBase {
  type: 'executing';
  data: ComfyUIExecutingData;
}

interface ComfyUIProgressData {
  value: number;
  max: number;
  node?: string; // Optional, KSampler often sends this
  prompt_id: string; // Added to align with other messages
}
interface ComfyUIProgressMessage extends ComfyUIWebSocketMessageBase {
  type: 'progress';
  data: ComfyUIProgressData;
}

interface ComfyUIImageOutput {
  filename: string;
  subfolder: string;
  type: 'output' | 'temp' | 'input';
}
interface ComfyUIExecutedOutputNode {
  images: ComfyUIImageOutput[];
  [key: string]: any; // For other output types like text
}
interface ComfyUIExecutedData {
  prompt_id: string;
  output: {
    [nodeId: string]: ComfyUIExecutedOutputNode;
  };
  node?: string; // Sometimes present, indicates the node that finished
}
interface ComfyUIExecutedMessage extends ComfyUIWebSocketMessageBase {
  type: 'executed';
  data: ComfyUIExecutedData;
}

interface ComfyUIExecutionErrorData {
  prompt_id: string;
  exception_message: string;
  exception_type: string;
  traceback: string[];
  node_id: string;
  node_type: string;
  // ... other error details
}
interface ComfyUIExecutionErrorMessage extends ComfyUIWebSocketMessageBase {
  type: 'execution_error';
  data: ComfyUIExecutionErrorData;
}

interface ComfyUIExecutionInterruptedData {
    prompt_id: string;
    // ... other interruption details
}
interface ComfyUIExecutionInterruptedMessage extends ComfyUIWebSocketMessageBase {
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


/**
 * Generates the ComfyUI prompt payload based on character data and workflow type.
 */
export async function generateComfyUIPromptPayload(
  characterData: CharacterAttributes,
  tags: Tag[],
  workflowType: "face" | "fullbody" | "fullbody_detailer", // Added fullbody_detailer
  selectedFaceFilename?: string | null,
  selectedFaceSubfolder?: string | null
): Promise<{ workflowJson: object; outputNodeId: string }> {
  try {
    // Determine the correct workflow file name based on workflowType
    // const templateFileName = workflowType === "face"
    //   ? "face_workflow_template.json"
    //   : workflowType === "fullbody_detailer"
    //   ? "fullbody_workflow_facedetailer.json"
    //   : "fullbody_workflow_template.json";
    // The actual filename isn't used here, Rust side handles it based on workflowType string

    const templateContent = await invoke<string>('get_workflow_template', { workflowType });
    const workflowJson = JSON.parse(templateContent);

    let positivePromptNodeId: string;
    let negativePromptNodeId: string;
    let seedNodeId: string;
    let determinedOutputNodeId: string;
    let loadImageNodeId: string | null = null;
    let insightFaceLoaderNodeId: string | null = null;
    let faceDetailerPositivePromptNodeId: string | null = null; // For fullbody_detailer

    if (workflowType === "face") {
      positivePromptNodeId = "6";
      negativePromptNodeId = "7";
      seedNodeId = "3";
      determinedOutputNodeId = "9";
    } else if (workflowType === "fullbody_detailer") {
      positivePromptNodeId = "2";
      negativePromptNodeId = "3";
      seedNodeId = "5";
      loadImageNodeId = "12";
      insightFaceLoaderNodeId = "10"; // Assuming this is still relevant for IPAdapter
      faceDetailerPositivePromptNodeId = "22"; // New node for FaceDetailer prompt
      determinedOutputNodeId = "7"; // SaveImage node for final output
    } else { // "fullbody" (original)
      positivePromptNodeId = "2";
      negativePromptNodeId = "3";
      seedNodeId = "5";
      loadImageNodeId = "12";
      insightFaceLoaderNodeId = "10";
      determinedOutputNodeId = "7";
    }

    const textInputName = "text";
    const seedInputName = "seed";
    const imageNameInput = "image";

    // --- Construct dynamic prompt parts ---
    const promptParts: string[] = [];

    // 1. Anatomy
    if (characterData.anatomy?.toLowerCase() === "male") {
      promptParts.push("1boy");
    } else if (characterData.anatomy?.toLowerCase() === "female") {
      promptParts.push("1girl");
    }

    // 2. Gender Expression
    if (characterData.genderExpression !== undefined) {
      const expression = characterData.genderExpression; // This value is -10 to 10
      if (expression <= -3) promptParts.push("masculine");
      else if (expression <= 2) promptParts.push("androgynous"); // -2 to 2
      else promptParts.push("feminine"); // 3 to 10
    }
    
    // 3. Other Attributes (Ethnicity, Hair, Eyes, Body Type for fullbody)
    if (characterData.ethnicity) promptParts.push(characterData.ethnicity); // Removed " ethnicity" suffix
    if (characterData.hairColor) promptParts.push(characterData.hairColor + " hair");
    if (characterData.eyeColor) promptParts.push(characterData.eyeColor + " eyes");

    if (workflowType === "fullbody" || workflowType === "fullbody_detailer") {
      if (characterData.bodyType) promptParts.push(characterData.bodyType + " body type");
    }
    
    // 4. Tags
    tags.forEach(tag => {
      promptParts.push(tag.name);
    });

    // 5. Simple Background (is now part of the base template for face, ensure it's in fullbody too)
    // promptParts.push("simple background"); // Removed as it's in the template

    const dynamicPromptContent = promptParts.filter(part => part.trim() !== "").join(", ");

    // --- Inject into Main Positive Prompt (Node 2 or 6) ---
    if (workflowJson[positivePromptNodeId]?.inputs && workflowJson[positivePromptNodeId].inputs[textInputName]) {
      let basePromptText = workflowJson[positivePromptNodeId].inputs[textInputName] as string;
      const placeholder = "__DYNAMIC_PROMPT__";
      if (basePromptText.includes(placeholder)) {
        basePromptText = basePromptText.replace(placeholder, dynamicPromptContent);
      } else {
        console.warn(`Placeholder '${placeholder}' not found in positive prompt node ${positivePromptNodeId}. Appending dynamic content.`);
        if (basePromptText.trim().length > 0 && !basePromptText.trim().endsWith(',')) {
            basePromptText += ", ";
        }
        basePromptText += dynamicPromptContent;
      }
      workflowJson[positivePromptNodeId].inputs[textInputName] = basePromptText;
      // Updated logging to show for all workflow types to help debug
      console.log(`[ComfyService] ${workflowType} Workflow - Main Positive Prompt (Node ${positivePromptNodeId}): ${basePromptText}`);
    }

    // --- Inject into FaceDetailer Positive Prompt (Node 22 for fullbody_detailer) ---
    if (workflowType === "fullbody_detailer" && faceDetailerPositivePromptNodeId && workflowJson[faceDetailerPositivePromptNodeId]?.inputs) {
      let faceDetailerPrompt = workflowJson[faceDetailerPositivePromptNodeId].inputs[textInputName] as string;
      const faceDetailerAdditions: string[] = [];
      if (characterData.eyeColor) faceDetailerAdditions.push(characterData.eyeColor + " eyes"); // More specific for face
      if (characterData.hairColor) faceDetailerAdditions.push(characterData.hairColor + " hair"); // More specific for face
      
      if (faceDetailerAdditions.length > 0) {
        if (faceDetailerPrompt.trim().length > 0 && !faceDetailerPrompt.trim().endsWith(',')) {
            faceDetailerPrompt += ", ";
        }
        faceDetailerPrompt += faceDetailerAdditions.join(", ");
      }
      workflowJson[faceDetailerPositivePromptNodeId].inputs[textInputName] = faceDetailerPrompt;
    }

    // --- Negative Prompt (remains largely the same, but ensure it's set) ---
    const negativePromptText = "modern, recent, old, oldest, cartoon, anime, graphic, text, painting, crayon, graphite, abstract, glitch, deformed, mutated, ugly, disfigured, long body, lowres, bad anatomy, bad hands, missing fingers, extra digits, fewer digits, cropped, very displeasing, (worst quality, bad quality:1.2), bad anatomy, sketch, jpeg artifacts, signature, watermark, username, conjoined, bad ai-generated, shine, shiny, porcelain skin, child, loli";
    if (workflowJson[negativePromptNodeId]?.inputs) {
      workflowJson[negativePromptNodeId].inputs[textInputName] = negativePromptText;
    }

    // --- Seed ---
    if (workflowJson[seedNodeId]?.inputs) {
      workflowJson[seedNodeId].inputs[seedInputName] = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
    }

    // --- LoadImage for IPAdapter (fullbody workflows) ---
    if ((workflowType === "fullbody" || workflowType === "fullbody_detailer") && selectedFaceFilename && loadImageNodeId && workflowJson[loadImageNodeId]?.inputs) {
      const imagePath = selectedFaceSubfolder ? `${selectedFaceSubfolder}/${selectedFaceFilename}` : selectedFaceFilename;
      workflowJson[loadImageNodeId].inputs[imageNameInput] = imagePath;
      console.log(`[ComfyService] Set LoadImage node (${loadImageNodeId}) input to: ${imagePath}`);
    }

    // --- IPAdapterInsightFaceLoader model_name (fullbody workflows) ---
    if ((workflowType === "fullbody" || workflowType === "fullbody_detailer") && insightFaceLoaderNodeId && workflowJson[insightFaceLoaderNodeId]?.inputs) {
      workflowJson[insightFaceLoaderNodeId].inputs["model_name"] = "buffalo_l";
      console.log(`[ComfyService] Set IPAdapterInsightFaceLoader node (${insightFaceLoaderNodeId}) model_name to: buffalo_l`);
    }
    
    return { workflowJson, outputNodeId: determinedOutputNodeId };

  } catch (error) {
    console.error(`Error generating ComfyUI prompt payload for ${workflowType} workflow:`, error);
    throw error;
  }
}


/**
 * Initiates image generation with ComfyUI, handling HTTP prompt submission and WebSocket progress.
 */
export async function initiateImageGeneration(
  attributes: CharacterAttributes,
  tags: Tag[],
  workflowType: "face" | "fullbody",
  selectedFaceFilename?: string | null,
  selectedFaceSubfolder?: string | null // Added subfolder
): Promise<{ promptId: string; clientId: string; closeSocket: () => void } | null> {
  const {
    setClientId,
    setGenerationProgress,
    setIsGeneratingFace,
    setIsGeneratingFullBody,
    setFaceOptions,
    setFullBodyOptions,
    setError
  } = useCharacterStore.getState();

  const clientId = uuidv4();
  setClientId(clientId);

  if (workflowType === "face") {
    setIsGeneratingFace(true);
  } else {
    setIsGeneratingFullBody(true);
  }
  setError(null);
  setGenerationProgress({
    promptId: null,
    currentNodeId: null,
    currentNodeTitle: null, // Will need a mapping from ID to title if desired
    step: 0,
    maxSteps: 0,
    message: "Preparing workflow...",
    queuePosition: null,
  });

  try {
    // If "fullbody" is requested, use "fullbody_detailer" as the new default.
    const actualWorkflowType = workflowType === "fullbody" ? "fullbody_detailer" : workflowType;

    const { workflowJson: promptPayload, outputNodeId } = await generateComfyUIPromptPayload(
      attributes,
      tags,
      actualWorkflowType, // Use the potentially remapped type
      selectedFaceFilename,
      selectedFaceSubfolder
    );
    // const outputNodeId = (promptPayload as any)._outputNodeId || (workflowType === "face" ? "9" : "7"); // fallback // No longer needed

    const httpResponse = await fetch(`${comfyuiApiUrl}/prompt`, {
      method: 'POST',
      body: JSON.stringify({ prompt: promptPayload, client_id: clientId }),
      headers: { 'Content-Type': 'application/json' },
    });

    if (!httpResponse.ok) {
      const errorBody = await httpResponse.text();
      console.error(`ComfyUI API Error: ${httpResponse.status} - ${httpResponse.statusText}`, errorBody);
      setError(`ComfyUI API Error: ${httpResponse.status}. ${errorBody}`);
      if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
      setGenerationProgress(null);
      return null;
    }

    const responseData = await httpResponse.json() as { prompt_id: string; number: number; node_errors?: any };
    const promptId = responseData.prompt_id;

    setGenerationProgress({
        promptId: promptId,
        currentNodeId: null,
        currentNodeTitle: null,
        step: 0,
        maxSteps: 0,
        message: "Workflow submitted, waiting for execution...",
        queuePosition: null,
    });

    const socket = new WebSocket(`${comfyuiWsUrl}?clientId=${clientId}`);

    const getBaseProgress = () => {
        const storeState = useCharacterStore.getState();
        return storeState.generationProgress || {
            promptId: promptId,
            currentNodeId: null,
            currentNodeTitle: null,
            step: 0,
            maxSteps: 0,
            message: "",
            queuePosition: null,
            clientId: storeState.clientId,
        };
    };

    socket.onopen = () => {
      console.log(`ComfyUI WebSocket connected for clientId: ${clientId}, promptId: ${promptId}`);
      const currentProgress = getBaseProgress();
      setGenerationProgress({ ...currentProgress, message: "Connected to generation server." });
    };

    socket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data as string) as ComfyUIWebSocketMessage;

        if (message.data && (message.data as any).prompt_id && (message.data as any).prompt_id !== promptId) {
            return;
        }
        
        const currentProgress = getBaseProgress();

        switch (message.type) {
          case 'status':
            const statusData = message.data as ComfyUIStatusData;
            setGenerationProgress({
              ...currentProgress,
              promptId: currentProgress.promptId || promptId, // Ensure promptId is set
              message: `Queue position: ${statusData.status.exec_info.queue_remaining}`,
              queuePosition: statusData.status.exec_info.queue_remaining,
            });
            break;
          case 'execution_start':
            setGenerationProgress({ ...currentProgress, promptId: currentProgress.promptId || promptId, message: "Generation started..." });
            break;
          case 'executing':
            const executingData = message.data as ComfyUIExecutingData;
            if (executingData.node === null) {
                // Potentially handle end of execution if needed, though 'executed' is primary
            } else {
                setGenerationProgress({
                ...currentProgress,
                promptId: currentProgress.promptId || promptId,
                currentNodeId: executingData.node,
                currentNodeTitle: executingData.node, // Placeholder, can be improved with node title mapping
                message: `Executing node: ${executingData.node}`,
                });
            }
            break;
          case 'progress':
            const progressData = message.data as ComfyUIProgressData;
            setGenerationProgress({
              ...currentProgress,
              promptId: currentProgress.promptId || promptId,
              step: progressData.value,
              maxSteps: progressData.max,
              // Use progressData.node if available, otherwise fallback to currentProgress.currentNodeTitle
              message: `Processing node ${progressData.node || currentProgress?.currentNodeTitle || 'current node'}: Step ${progressData.value}/${progressData.max}`,
            });
            break;
          case 'executed':
            const executedData = message.data as ComfyUIExecutedData;
            if (executedData.prompt_id === promptId) {
              console.log('[ComfyService] Received "executed" message for promptId:', promptId);
              console.log('[ComfyService] Using outputNodeId:', outputNodeId);
              console.log('[ComfyService] Full executedData.output:', JSON.stringify(executedData.output, null, 2));

              const localImages: ImageOption[] = [];
              let imagesToProcess: ComfyUIImageOutput[] | undefined = undefined;

              // First, try to get images from the specific output node ID
              const specificNodeOutput = executedData.output[outputNodeId];
              if (specificNodeOutput && specificNodeOutput.images && Array.isArray(specificNodeOutput.images)) {
                imagesToProcess = specificNodeOutput.images;
                console.log(`[ComfyService] Found ${imagesToProcess.length} images in specificNodeOutput (nodeId: ${outputNodeId}).`);
              } else if (executedData.output.images && Array.isArray(executedData.output.images)) {
                // Fallback: if not found under specific node, check if 'images' is directly under 'executedData.output'
                // This handles cases where the output node is the primary/only output.
                imagesToProcess = executedData.output.images;
                console.log(`[ComfyService] Found ${imagesToProcess.length} images directly under executedData.output.`);
              }

              if (imagesToProcess) {
                imagesToProcess.forEach((imgOut: ComfyUIImageOutput, index: number) => {
                  if (imgOut.filename && imgOut.type) { // Basic validation
                    const imageUrl = `${comfyuiApiUrl}/view?filename=${encodeURIComponent(imgOut.filename)}&subfolder=${encodeURIComponent(imgOut.subfolder || '')}&type=${imgOut.type}`;
                    console.log(`[ComfyService] Constructed image URL: ${imageUrl}`);
                    localImages.push({
                      id: `${promptId}-img-${index}`,
                      url: imageUrl,
                      alt: `${workflowType} image ${index + 1}`
                    });
                  } else {
                    console.warn('[ComfyService] Invalid image data in output:', imgOut);
                  }
                });
              } else {
                console.warn(`[ComfyService] No images found in executedData.output for outputNodeId '${outputNodeId}' or directly under executedData.output. Full output:`, JSON.stringify(executedData.output, null, 2));
              }
              
              console.log('[ComfyService] Final localImages array to be set:', localImages);

              if (workflowType === "face") {
                setFaceOptions(localImages);
                setIsGeneratingFace(false);
              } else {
                setFullBodyOptions(localImages); // Use renamed variable
                setIsGeneratingFullBody(false);
              }
              setGenerationProgress({
                  ...currentProgress,
                  promptId: currentProgress.promptId || promptId,
                  message: "Generation complete!",
                  step: currentProgress?.maxSteps || 0, // Use last known maxSteps from currentProgress
                });
            }
            break;
          case 'execution_error':
            const errorData = message.data as ComfyUIExecutionErrorData;
            if (errorData.prompt_id === promptId) {
              console.error('ComfyUI Execution Error:', errorData);
              setError(`Execution Error on node ${errorData.node_id}: ${errorData.exception_message}`);
              if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
              setGenerationProgress(null);
            }
            break;
          case 'execution_cached':
            setGenerationProgress({ ...currentProgress, promptId: currentProgress.promptId || promptId, message: "Loading cached data..." });
            break;
          case 'execution_interrupted':
            if ((message.data as ComfyUIExecutionInterruptedData).prompt_id === promptId) {
                console.log('Execution interrupted');
                setError('Generation was interrupted.');
                if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
                setGenerationProgress(null);
            }
            break;
        }
      } catch (e) {
        console.error('Error processing WebSocket message:', e);
        setError('Error processing generation update.');
      }
    };

    socket.onerror = (errorEvent) => {
      console.error(`ComfyUI WebSocket Error for clientId: ${clientId}:`, errorEvent);
      const storeState = useCharacterStore.getState();
      // Only process 'onerror' if it's for the currently active client
      if (storeState.clientId === clientId) {
        setError('WebSocket connection error with generation server.');
        if (workflowType === "face") setIsGeneratingFace(false);
        if (workflowType === "fullbody") setIsGeneratingFullBody(false);
        setGenerationProgress(null); // Clear progress on WebSocket error
        setClientId(null); // Clear the client ID
      } else {
        console.warn(`ComfyUI WebSocket Error for an OLD clientId: ${clientId}. Current store clientId: ${storeState.clientId}. Ignoring.`);
      }
    };

    socket.onclose = (event) => {
      console.log(`ComfyUI WebSocket closed for clientId: ${clientId}. Code: ${event.code}, Reason: ${event.reason}`);
      const storeState = useCharacterStore.getState();

      // Only process 'onclose' if it's for the currently active client
      if (storeState.clientId === clientId) {
        const stillGenerating = (workflowType === "face" && storeState.isGeneratingFace) ||
                                (workflowType === "fullbody" && storeState.isGeneratingFullBody);

        if (stillGenerating) {
          // Check if an error was already set for this promptId to avoid overwriting a more specific error
          // Also, ensure this promptId matches the one in progress, if any.
          const currentProgressPromptId = storeState.generationProgress?.promptId;
          if (!storeState.error || (currentProgressPromptId && currentProgressPromptId === promptId)) {
             setError('Connection to generation server closed unexpectedly.');
          }
        }
        
        // Reset generating flags only if this was the active client and it's still marked as generating
        // This prevents an old socket's onclose from resetting a new generation's flag
        if (workflowType === "face" && storeState.isGeneratingFace) setIsGeneratingFace(false);
        if (workflowType === "fullbody" && storeState.isGeneratingFullBody) setIsGeneratingFullBody(false);

        const finalProgress = storeState.generationProgress;
        if (finalProgress && finalProgress.promptId === promptId) {
            setGenerationProgress({ ...finalProgress, message: "Disconnected." });
        } else if (storeState.generationProgress === undefined || storeState.generationProgress === null || !storeState.generationProgress.promptId) {
            // If progress was null or undefined, or had no promptId (e.g. very early disconnect)
            setGenerationProgress(null);
        }
        // If finalProgress.promptId is different, a new generation has already updated the progress, so we don't touch it.

        setClientId(null); // Clear the client ID as this generation is now finished or failed for this client.
      } else {
        console.log(`ComfyUI WebSocket closed for an OLD clientId: ${clientId}. Current store clientId: ${storeState.clientId}. Ignoring.`);
      }
    };

    return { promptId, clientId, closeSocket: () => socket.close() };

  } catch (error) {
    console.error('Error initiating image generation:', error);
    setError(`Failed to initiate generation: ${error instanceof Error ? error.message : String(error)}`);
    if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
    setGenerationProgress(null);
    setClientId(null);
    return null;
  }
}

/**
 * Uploads an image file to ComfyUI's /upload/image endpoint.
 * @param imageFile The image file to upload.
 * @param subfolder Optional subfolder to upload the image to.
 * @param type The type of image ('input', 'temp').
 * @returns A promise resolving with the server's response (filename, subfolder, type).
 */
export async function uploadImageToComfyUI(
    imageFile: File,
    subfolder?: string,
    type: 'input' | 'temp' = 'input'
): Promise<{ filename: string; name: string; subfolder?: string; type?: string } | null> {
    const formData = new FormData();
    formData.append('image', imageFile);
    formData.append('overwrite', 'true'); // Or handle existing files as needed
    if (subfolder) {
        formData.append('subfolder', subfolder);
    }
    formData.append('type', type);

    try {
        const response = await fetch(`${comfyuiApiUrl}/upload/image`, {
            method: 'POST',
            body: formData, // Tauri's fetch should handle FormData correctly
        });

        if (!response.ok) {
            const errorBody = await response.text();
            console.error(`ComfyUI Image Upload Error: ${response.status} - ${response.statusText}`, errorBody);
            throw new Error(`ComfyUI Image Upload Error: ${response.status}. Body: ${errorBody}`);
        }
        const responseData = await response.json();
        // Example response: { name: "example.png", subfolder: "optional_subfolder", type: "input" }
        // The 'name' field is what ComfyUI uses as 'filename' in LoadImage nodes.
        return { ...responseData, filename: responseData.name };
    } catch (error) {
        console.error('Error uploading image to ComfyUI:', error);
        useCharacterStore.getState().setError(`Failed to upload image: ${error instanceof Error ? error.message : String(error)}`);
        return null;
    }
}