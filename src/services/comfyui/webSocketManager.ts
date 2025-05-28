import {
  ComfyUIWebSocketMessage,
  ComfyUIStatusData,
  ComfyUIExecutingData,
  ComfyUIProgressData,
  ComfyUIExecutedData,
  ComfyUIExecutionErrorData,
  ComfyUIImageOutput,
  ImageOption
} from './types';
import useCharacterStore from '../../store/characterStore'; // Import the store
import { getImageUrl } from './apiClient'; // For constructing image URLs

export const comfyuiWsUrl = 'ws://127.0.0.1:8188/ws';

interface WebSocketManagerOptions {
  clientId: string;
  promptId: string;
  outputNodeId: string;
  workflowType: "face" | "fullbody" | "fullbody_detailer"; // To know which store slice to update
inputSeed?: string | number; // Added to pass the seed used in the prompt
  onOpen?: (event: Event) => void;
  onMessage?: (message: ComfyUIWebSocketMessage) => void;
  onError?: (event: Event) => void;
  onClose?: (event: CloseEvent) => void;
  onImageGenerated?: (images: ImageOption[]) => void; // Callback for when images are ready
  onGenerationComplete?: () => void;
  onGenerationError?: (error: ComfyUIExecutionErrorData) => void;
}

export function createWebSocketManager(options: WebSocketManagerOptions) {
  const {
    clientId,
    promptId,
    outputNodeId,
    workflowType,
    onOpen,
    onMessage,
    onError,
    onClose,
    onImageGenerated,
    onGenerationComplete,
    onGenerationError,
inputSeed, // Destructure the new option
  } = options;

  const socket = new WebSocket(`${comfyuiWsUrl}?clientId=${clientId}`);

  const {
    setGenerationProgress,
    setFaceOptions,
    setFullBodyOptions,
    setIsGeneratingFace,
    setIsGeneratingFullBody,
    setError: setStoreError, // Renamed to avoid conflict
    // getClientId, // Not directly needed here, passed in options
  } = useCharacterStore.getState();


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
        clientId: clientId,
    };
  };

  socket.onopen = (event) => {
    console.log(`[WSManager] ComfyUI WebSocket connected for clientId: ${clientId}, promptId: ${promptId}`);
    const currentProgress = getBaseProgress();
    setGenerationProgress({ ...currentProgress, message: "Connected to generation server." });
    if (onOpen) onOpen(event);
  };

  socket.onmessage = (event) => {
    try {
      const message = JSON.parse(event.data as string) as ComfyUIWebSocketMessage;

      // Ignore messages for other prompts
      if (message.data && (message.data as any).prompt_id && (message.data as any).prompt_id !== promptId) {
          return;
      }
      
      const currentProgress = getBaseProgress();

      switch (message.type) {
        case 'status':
          const statusData = message.data as ComfyUIStatusData;
          setGenerationProgress({
            ...currentProgress,
            message: `Queue position: ${statusData.status.exec_info.queue_remaining}`,
            queuePosition: statusData.status.exec_info.queue_remaining,
          });
          break;
        case 'execution_start':
          setGenerationProgress({ ...currentProgress, message: "Generation started..." });
          break;
        case 'executing':
          const executingData = message.data as ComfyUIExecutingData;
          setGenerationProgress({
            ...currentProgress,
            currentNodeId: executingData.node,
            currentNodeTitle: executingData.node, // Placeholder
            message: `Executing node: ${executingData.node || 'finalizing'}`,
          });
          break;
        case 'progress':
          const progressData = message.data as ComfyUIProgressData;
          setGenerationProgress({
            ...currentProgress,
            step: progressData.value,
            maxSteps: progressData.max,
            message: `Processing node ${progressData.node || currentProgress?.currentNodeTitle || 'current node'}: Step ${progressData.value}/${progressData.max}`,
          });
          break;
        case 'executed':
          const executedData = message.data as ComfyUIExecutedData;
          if (executedData.prompt_id === promptId) {
            console.log('[WSManager] Received "executed" message for promptId:', promptId, 'Output Node ID:', outputNodeId);
            console.log('[WSManager] Full executedData.output:', JSON.stringify(executedData.output, null, 2));

            const localImages: ImageOption[] = [];
            let imagesToProcess: ComfyUIImageOutput[] | undefined = undefined;

            const specificNodeOutput = executedData.output[outputNodeId];
            if (specificNodeOutput && specificNodeOutput.images && Array.isArray(specificNodeOutput.images)) {
              imagesToProcess = specificNodeOutput.images;
            } else if (executedData.output.images && Array.isArray(executedData.output.images)) {
              imagesToProcess = executedData.output.images as ComfyUIImageOutput[]; // Cast if necessary
            }

            if (imagesToProcess) {
              imagesToProcess.forEach((imgOut: ComfyUIImageOutput, index: number) => {
                if (imgOut.filename && imgOut.type) {
                  const imageUrl = getImageUrl(imgOut.filename, imgOut.subfolder || '', imgOut.type as 'output' | 'temp' | 'input');
                  localImages.push({
                    id: `${promptId}-img-${index}`,
                    url: imageUrl,
                    alt: `${workflowType} image ${index + 1}`,
                    filename: imgOut.filename,
                    subfolder: imgOut.subfolder,
seed: inputSeed, // Assign the inputSeed here
                    type: imgOut.type
                  });
                }
              });
            } else {
                console.warn(`[WSManager] No images found in executedData.output for outputNodeId '${outputNodeId}' or directly under executedData.output.`);
            }
            
            console.log('[WSManager] Final localImages array:', localImages);

            if (onImageGenerated) onImageGenerated(localImages);

            if (workflowType === "face") {
              setFaceOptions(localImages);
              setIsGeneratingFace(false);
            } else { // fullbody or fullbody_detailer
              setFullBodyOptions(localImages);
              setIsGeneratingFullBody(false);
            }
            setGenerationProgress({
                ...currentProgress,
                message: "Generation complete!",
                step: currentProgress?.maxSteps || 0, // Ensure step is at max
              });
            if (onGenerationComplete) onGenerationComplete();
          }
          break;
        case 'execution_error':
          const errorData = message.data as ComfyUIExecutionErrorData;
          if (errorData.prompt_id === promptId) {
            console.error('[WSManager] ComfyUI Execution Error:', errorData);
            setStoreError(`Execution Error on node ${errorData.node_id}: ${errorData.exception_message}`);
            if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
            setGenerationProgress(null);
            if (onGenerationError) onGenerationError(errorData);
          }
          break;
        case 'execution_cached':
            console.log('[WSManager] Execution cached for prompt:', promptId, message.data);
            // Potentially update UI to reflect cached execution
            setGenerationProgress({ ...currentProgress, message: "Using cached results..." });
            break;
        case 'execution_interrupted':
            console.warn('[WSManager] Execution interrupted for prompt:', promptId, message.data);
            setStoreError('Image generation was interrupted.');
            if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
            setGenerationProgress(null);
            // if (onGenerationError) onGenerationError({ ...message.data, exception_message: 'Interrupted' }); // Adapt if needed
            break;
        default:
          // console.log('[WSManager] Received unhandled message type:', message.type, message.data);
          break;
      }
      if (onMessage) onMessage(message);
    } catch (err) {
      console.error('[WSManager] Error processing WebSocket message:', err);
      // Potentially call onError callback or set a general error state
    }
  };

  socket.onerror = (event) => {
    console.error(`[WSManager] ComfyUI WebSocket error for clientId: ${clientId}, promptId: ${promptId}:`, event);
    setStoreError('WebSocket connection error.');
    if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
    setGenerationProgress(null);
    if (onError) onError(event);
  };

  socket.onclose = (event) => {
    console.log(`[WSManager] ComfyUI WebSocket closed for clientId: ${clientId}, promptId: ${promptId}. Code: ${event.code}, Reason: ${event.reason}`);
    // Don't reset progress if it was a clean close after 'executed'
    const finalProgress = useCharacterStore.getState().generationProgress;
    if (finalProgress && finalProgress.message !== "Generation complete!" && !event.wasClean) {
        // If not clean and not already complete, then it's likely an unexpected closure
        setStoreError('WebSocket connection closed unexpectedly.');
        if (workflowType === "face") setIsGeneratingFace(false); else setIsGeneratingFullBody(false);
        setGenerationProgress(null);
    }
    if (onClose) onClose(event);
  };

  return {
    socket, // Expose the socket if direct manipulation is needed (e.g., sending custom messages)
    close: () => socket.close(),
  };
}