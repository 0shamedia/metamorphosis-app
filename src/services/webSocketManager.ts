import {
  ComfyUIWebSocketMessage,
  ComfyUIStatusData,
  ComfyUIExecutingData,
  ComfyUIProgressData,
  ComfyUIExecutedData,
  ComfyUIExecutionErrorData,
  ImageOption,
  isSaveImageOutput,
} from './comfyui/types';
import useCharacterStore from '../store/characterStore';
import { GenerationMode } from '../types/generation';
import { invoke } from '@tauri-apps/api/core';

export const comfyuiWsUrl = 'ws://127.0.0.1:8188/ws';
export const comfyuiUrl = 'http://127.0.0.1:8188';



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
  // Binary data is no longer processed client-side.
  // socket.binaryType = 'arraybuffer';

  const {
    setGenerationProgress,
    setIsGeneratingFace,
    setIsGeneratingFullBody,
    setError: setStoreError,
    addImageOption,
    lastGenerationMode,
  } = useCharacterStore.getState();

  let lastExecutedNodeId: string | null = null;

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
      console.warn('[WSManager] Received unexpected binary data. This path is deprecated and will be ignored.');
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
              message: "Finalizing...",
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
          
          if (lastExecutedNodeId === '335' && isSaveImageOutput(executedData.output)) {
            console.log('[WSManager] SaveImage node (335) executed. Processing output...');
            for (const image of executedData.output.images) {
              try {
                // The image is already saved by ComfyUI. Get it as a data URL to bypass asset protocol issues.
                const assetUrl = await invoke<string>('get_image_as_data_url', {
                  filename: image.filename,
                  subfolder: image.subfolder || '',
                });

                const imageType = lastGenerationMode === GenerationMode.FaceFromPrompt || lastGenerationMode === GenerationMode.RegenerateFace ? 'face' : 'fullbody';
                
                const newImageOption: ImageOption = {
                  id: await invoke('generate_uuid'),
                  url: assetUrl, // Use the permanent asset URL from the backend
                  seed: inputSeed,
                  alt: `Generated ${imageType} image with seed ${inputSeed}`,
                  filename: image.filename,
                };

                addImageOption(imageType, newImageOption);
                console.log(`[WSManager] Added new ${imageType} image option to gallery:`, newImageOption);
                console.log(`[WSManager] Data URL length: ${assetUrl.length}, starts with:`, assetUrl.substring(0, 50) + '...');

              } catch (error) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                console.error('[WSManager] Error processing image path:', errorMessage);
                setStoreError(`Failed to process generated image path: ${errorMessage}`);
              }
            }
            setIsGeneratingFace(false);
            setIsGeneratingFullBody(false);
            setGenerationProgress(null);
          } else {
            setGenerationProgress({
             ...currentProgress,
             message: `Node ${lastExecutedNodeId} complete. Continuing...`,
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
    if (finalProgress && !event.wasClean) {
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