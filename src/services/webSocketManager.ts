import {
  ComfyUIWebSocketMessage,
  ComfyUIStatusData,
  ComfyUIExecutingData,
  ComfyUIProgressData,
  ComfyUIExecutedData,
  ComfyUIExecutionErrorData,
  ImageOption
} from './comfyui/types';
import useCharacterStore from '../store/characterStore';
import { GenerationMode } from '../types/generation';
import { invoke } from '@tauri-apps/api/core';

export const comfyuiWsUrl = 'ws://127.0.0.1:8188/ws';

interface WebSocketManagerOptions {
  clientId: string;
  promptId: string;
  inputSeed: number;
  onOpen?: (event: Event) => void;
  onError?: (event: Event) => void;
  onClose?: (event: CloseEvent) => void;
}

export function createWebSocketManager(options: WebSocketManagerOptions) {
  const {
    clientId,
    promptId,
    inputSeed,
    onOpen,
    onError,
    onClose,
  } = options;

  const socket = new WebSocket(`${comfyuiWsUrl}?clientId=${clientId}`);
  socket.binaryType = 'arraybuffer';

  const {
    setGenerationProgress,
    setIsGeneratingFace,
    setIsGeneratingFullBody,
    setError: setStoreError,
    addImageOption,
    lastGenerationMode,
  } = useCharacterStore.getState();

  const FINAL_SAVE_NODE_ID = '329';
  let lastExecutedNodeId: string | null = null;
  let pendingImageData: ArrayBuffer | null = null;

  const processImage = (imageData: ArrayBuffer) => {
    console.log(`[WSManager] Processing image from node: ${lastExecutedNodeId}`);
    
    // This logic handles intermediate images as previews
    if (lastExecutedNodeId !== FINAL_SAVE_NODE_ID) {
      console.log('[WSManager] Displaying preview image.');
      const blob = new Blob([imageData], { type: 'image/png' });
      const url = URL.createObjectURL(blob);
      const imageType = lastGenerationMode === GenerationMode.FaceFromPrompt ? 'face' : 'fullbody';
      
      const newImageOption: ImageOption = {
        id: `temp-id-${Date.now()}-${Math.random()}`,
        url: url,
        alt: `Generated ${imageType} preview`,
        seed: inputSeed,
      };
      addImageOption(imageType, newImageOption);
      // For previews, we might not want to stop the generation process entirely
      // depending on the desired UX. For now, we'll let the final node control this.
      return;
    }

    // This logic handles the final image save
    console.log('[WSManager] Saving final image to disk...');
    const reader = new FileReader();
    reader.onload = async () => {
      try {
        const base64String = (reader.result as string).split(',')[1];
        const characterId = useCharacterStore.getState().characterId;
        const imageType = lastGenerationMode === GenerationMode.FaceFromPrompt ? 'face' : 'fullbody';

        if (!characterId) {
          throw new Error("Cannot save image, characterId is null.");
        }

        const savedImageDetails = await invoke<ImageOption>('save_image_to_disk', {
          base64Data: base64String,
          imageType: imageType,
          seed: inputSeed,
          characterId: characterId,
        });

        if (savedImageDetails) {
          addImageOption(imageType, savedImageDetails);
          console.log(`[WSManager] Added new final ${imageType} image option to the store.`);
        }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        console.error('[WSManager] Error saving final image:', errorMessage);
        setStoreError(`Failed to save final image: ${errorMessage}`);
      } finally {
        // Only stop generation after the final image is processed.
        setIsGeneratingFace(false);
        setIsGeneratingFullBody(false);
        setGenerationProgress(null);
      }
    };
    const blob = new Blob([imageData], { type: 'image/png' });
    reader.readAsDataURL(blob);
  };

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

  socket.onmessage = async (event) => {
    if (event.data instanceof ArrayBuffer) {
      console.log(`[WSManager] Received binary image data. Last executed node: ${lastExecutedNodeId}`);

      if (lastExecutedNodeId !== FINAL_SAVE_NODE_ID) {
        console.log(`[WSManager] Ignoring binary data because it's not from the final save node (${FINAL_SAVE_NODE_ID}).`);
        return;
      }

      console.log('[WSManager] Processing final image from SaveImageWebsocket node.');
      const blob = new Blob([event.data], { type: 'image/png' });
      
      const reader = new FileReader();
      reader.onload = async () => {
        try {
          const base64String = (reader.result as string).split(',')[1];
          const characterId = useCharacterStore.getState().characterId;
          const imageType = lastGenerationMode === GenerationMode.FaceFromPrompt ? 'face' : 'fullbody';

          if (!characterId) {
            throw new Error("Cannot save image, characterId is null. This indicates an image was received before a character was created.");
          }

          const savedImageDetails = await invoke<ImageOption>('save_image_to_disk', {
            base64Data: base64String,
            imageType: imageType,
            seed: inputSeed,
            characterId: characterId,
          });

          if (savedImageDetails) {
            addImageOption(imageType, savedImageDetails);
            console.log(`[WSManager] Added new ${imageType} image option to the store.`);
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error);
          console.error('[WSManager] Error saving final image:', errorMessage);
          setStoreError(`Failed to save final image: ${errorMessage}`);
        } finally {
          setIsGeneratingFace(false);
          setIsGeneratingFullBody(false);
          setGenerationProgress(null);
        }
      };
      reader.readAsDataURL(blob);
      return;
    }

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
            message: `Queue position: ${statusData.status.exec_info.queue_remaining}`,
            queuePosition: statusData.status.exec_info.queue_remaining,
          });
          break;
        case 'execution_start':
          setGenerationProgress({ ...currentProgress, message: "Generation started..." });
          break;
        case 'executing':
          const executingData = message.data as ComfyUIExecutingData;
          if (executingData.node === null) {
            setGenerationProgress({
              ...currentProgress,
              message: "Finalizing image...",
              step: currentProgress.maxSteps,
            });
          } else {
            setGenerationProgress({
              ...currentProgress,
              currentNodeId: executingData.node,
              message: `Executing node: ${executingData.node}`,
            });
          }
          break;
        case 'progress':
          const progressData = message.data as ComfyUIProgressData;
          setGenerationProgress({
            ...currentProgress,
            step: progressData.value,
            maxSteps: progressData.max,
            message: `Processing: Step ${progressData.value}/${progressData.max}`,
          });
          break;
        case 'executed':
           const executedData = message.data as ComfyUIExecutedData;
           lastExecutedNodeId = executedData.node ?? null;
           console.log(`[WSManager] Node ${lastExecutedNodeId} executed.`);

           if (lastExecutedNodeId === FINAL_SAVE_NODE_ID) {
             console.log('[WSManager] Final save node executed. Awaiting final image data via WebSocket.');
             setGenerationProgress({
               ...currentProgress,
               message: "Finalizing image...",
               step: currentProgress.maxSteps,
             });
           } else {
              setGenerationProgress({
               ...currentProgress,
               message: `Node ${lastExecutedNodeId} complete. Continuing workflow...`,
             });
           }
          break;
        case 'execution_error':
          const errorData = message.data as ComfyUIExecutionErrorData;
          if (errorData.prompt_id === promptId) {
            console.error('[WSManager] ComfyUI Execution Error:', errorData);
            setStoreError(`Execution Error on node ${errorData.node_id}: ${errorData.exception_message}`);
            setIsGeneratingFace(false);
            setIsGeneratingFullBody(false);
            setGenerationProgress(null);
          }
          break;
        default:
          break;
      }
    } catch (err) {
      console.error('[WSManager] Error processing JSON WebSocket message:', err, 'Raw data:', event.data);
    }
  };

  socket.onerror = (event) => {
    console.error(`[WSManager] ComfyUI WebSocket error for clientId: ${clientId}, promptId: ${promptId}:`, event);
    setStoreError('WebSocket connection error.');
    setIsGeneratingFace(false);
    setIsGeneratingFullBody(false);
    setGenerationProgress(null);
    if (onError) onError(event);
  };

  socket.onclose = (event) => {
    console.log(`[WSManager] ComfyUI WebSocket closed for clientId: ${clientId}, promptId: ${promptId}. Code: ${event.code}, Reason: ${event.reason}`);
    const finalProgress = useCharacterStore.getState().generationProgress;
    if (finalProgress && finalProgress.message !== "Generation complete! Awaiting image data..." && !event.wasClean) {
        setStoreError('WebSocket connection closed unexpectedly.');
        setIsGeneratingFace(false);
        setIsGeneratingFullBody(false);
        setGenerationProgress(null);
    }
    if (onClose) onClose(event);
  };

  return {
    socket,
    close: () => socket.close(),
  };
}