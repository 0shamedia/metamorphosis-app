'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { fetch } from '@tauri-apps/plugin-http';
import ModelStatusBadge, { DownloadStatus } from './ModelStatusBadge';

// Declare __TAURI__ on the Window interface
declare global {
  interface Window {
    __TAURI__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      // Add other Tauri APIs you might use and need types for
    };
  }
}

// Dynamically import Tauri API functions
interface TauriFsApi {
  resolveResource: (path: string) => Promise<string>;
  exists: (path: string) => Promise<boolean>;
  createDir: (path: string, options?: { recursive: boolean }) => Promise<void>;
  writeBinaryFile: (path: string, contents: Uint8Array) => Promise<void>;
  // Add other fs functions if needed
}

// Dynamically import Tauri API functions
let tauriFs: Promise<TauriFsApi | null>;

if (typeof window !== 'undefined' && window.__TAURI__) {
  // Type assertion here to tell TypeScript that the imported module
  // will conform to TauriFsApi when available.
  tauriFs = import('@tauri-apps/api/fs') as Promise<TauriFsApi>;
} else {
  tauriFs = Promise.resolve(null);
}

import ProgressBar from './ProgressBar';

interface ModelDefinition {
  id: string;
  name: string;
  url: string;
  targetSubdir: string; // Relative to comfyui/models/
  targetFilename: string;
}

interface ModelState extends ModelDefinition {
  status: DownloadStatus;
  progress?: number;
  errorMessage?: string;
}

// Define required models here
// Using a smaller test file URL for faster testing initially
// Replace with actual model URLs later
const requiredModels: ModelDefinition[] = [
  {
    id: 'sd-v1-5-pruned',
    name: 'Stable Diffusion v1.5 Pruned Emaonly',
    // Placeholder URL - Using a small ~5MB safetensors file for testing
    url: 'https://huggingface.co/runwayml/stable-diffusion-v1-5/resolve/main/v1-5-pruned-emaonly.safetensors?download=true',
    // Real URL (approx 4GB) - uncomment later
    // url: 'https://huggingface.co/runwayml/stable-diffusion-v1-5/resolve/main/v1-5-pruned-emaonly.safetensors',
    targetSubdir: 'checkpoints',
    targetFilename: 'v1-5-pruned-emaonly.safetensors',
  },
  // Add other required models here (LoRAs, VAEs, etc.)
  // {
  //   id: 'vae-ft-mse-840000',
  //   name: 'VAE ft-mse 840000',
  //   url: 'https://huggingface.co/stabilityai/sd-vae-ft-mse-original/resolve/main/vae-ft-mse-840000-ema-pruned.safetensors',
  //   targetSubdir: 'vae',
  //   targetFilename: 'vae-ft-mse-840000-ema-pruned.safetensors',
  // },
];

const ModelDownloader: React.FC = () => {
  const [models, setModels] = useState<ModelState[]>(
    requiredModels.map((m) => ({ ...m, status: 'Not Downloaded' }))
  );
  const [baseResourcePath, setBaseResourcePath] = useState<string | null>(null);

  // Function to get the base path for ComfyUI models
  const getComfyUIBasePath = useCallback(async (): Promise<string | null> => {
    if (baseResourcePath) return baseResourcePath;
    const fs = await tauriFs;
    if (!fs) {
      console.warn('Tauri FS API not available.');
      return null;
    }
    try {
      // Assuming ComfyUI is in vendor/comfyui relative to resource dir
      const resourcePath = await fs.resolveResource('vendor/comfyui');
      // Need to remove the leading '\\?\' on Windows if present
      const cleanedPath = resourcePath.startsWith('\\\\?\\') ? resourcePath.substring(4) : resourcePath;
      setBaseResourcePath(cleanedPath);
      return cleanedPath;
    } catch (error) {
      console.error('Error resolving resource directory:', error);
      // Handle error appropriately - maybe set a global error state
      return null;
    }
  }, [baseResourcePath]);


  // Function to check if a model file exists
  const checkModelExists = useCallback(async (model: ModelDefinition): Promise<boolean> => {
    const comfyUIBase = await getComfyUIBasePath();
    if (!comfyUIBase) return false;

    const fs = await tauriFs;
    if (!fs) {
      console.warn('Tauri FS API not available.');
      return false;
    }

    const modelPath = `${comfyUIBase}/models/${model.targetSubdir}/${model.targetFilename}`;
    try {
      return await fs.exists(modelPath);
    } catch {
      // exists throws error if path is invalid *or* file doesn't exist on some platforms/setups
      // console.warn(`Error checking existence for ${modelPath}:`, error);
      return false;
    }
  }, [getComfyUIBasePath]);


  // Check initial status of models on mount
  useEffect(() => {
    const checkAllModels = async () => {
      const comfyUIBase = await getComfyUIBasePath();
      if (!comfyUIBase) {
        // Handle error: Cannot determine resource path
        console.error("Could not determine ComfyUI base path. Downloads disabled.");
        setModels(prev => prev.map(m => ({...m, status: 'Error', errorMessage: 'Resource path error'})));
        return;
      }

      const updatedModels = await Promise.all(
        models.map(async (model) => {
          const modelExists = await checkModelExists(model);
          return {
            ...model,
            status: modelExists ? 'Downloaded' as DownloadStatus : 'Not Downloaded' as DownloadStatus,
          };
        })
      );
      setModels(updatedModels);
    };
    checkAllModels();
    // Run only once on mount, dependencies are stable or handled internally
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [checkModelExists]); // Add checkModelExists dependency


  const updateModelStatus = (id: string, status: DownloadStatus, progress?: number, errorMessage?: string) => {
    setModels((prevModels) =>
      prevModels.map((m) =>
        m.id === id ? { ...m, status, progress, errorMessage } : m
      )
    );
  };

  const handleDownload = async (model: ModelState) => {
    if (model.status === 'Downloading' || model.status === 'Downloaded') return;

    const comfyUIBase = await getComfyUIBasePath();
    if (!comfyUIBase) {
        updateModelStatus(model.id, 'Error', undefined, 'Resource path error');
        return;
    }

    const fs = await tauriFs;
    if (!fs) {
      console.warn('Tauri FS API not available. Cannot download.');
      updateModelStatus(model.id, 'Error', undefined, 'Tauri API not available');
      return;
    }

    const targetDir = `${comfyUIBase}/models/${model.targetSubdir}`;
    const targetPath = `${targetDir}/${model.targetFilename}`;

    updateModelStatus(model.id, 'Downloading', 0);

    try {
      // 1. Ensure directory exists
      await fs.createDir(targetDir, { recursive: true });
      console.log(`Ensured directory exists: ${targetDir}`);

      // 2. Download the file
      console.log(`Starting download from ${model.url} to ${targetPath}`);
      const response = await fetch(model.url, {
        method: 'GET',
        // Note: Tauri's fetch plugin currently doesn't expose easy progress reporting in JS.
        // Progress tracking would require backend involvement or more complex chunked downloads.
        // For now, we just show "Downloading" without percentage.
      });

      console.log(`Download response status: ${response.status}`);
      if (!response.ok) {
        throw new Error(`Download failed: ${response.status} ${response.statusText}`);
      }

      // 3. Write the file
      const data = await response.arrayBuffer();
      console.log(`Writing ${data.byteLength} bytes to ${targetPath}`);
      await fs.writeBinaryFile(targetPath, new Uint8Array(data));
      console.log(`Successfully wrote file: ${targetPath}`);

      // 4. Update status
      updateModelStatus(model.id, 'Downloaded');

    } catch (error: unknown) {
      console.error(`Error downloading model ${model.name}:`, error);
      // Safely access error message if it's an Error instance
      const errorMessage = error instanceof Error ? error.message : 'Download failed';
      updateModelStatus(model.id, 'Error', undefined, errorMessage);
    }
  };

  return (
    <div className="p-4 border rounded-lg shadow-md">
      <h2 className="text-xl font-semibold mb-4">AI Model Downloads</h2>
      {baseResourcePath === null && <p className="text-red-500">Loading resource path...</p>}
      <ul className="space-y-3">
        {models.map((model) => (
          <li key={model.id} className="flex items-center justify-between p-3 bg-gray-50 rounded">
            <div className="flex-1 mr-4">
              <span className="font-medium">{model.name}</span>
              {model.status === 'Downloading' && model.progress !== undefined && (
                <div className="mt-1">
                  <ProgressBar progress={model.progress} />
                </div>
              )}
            </div>
            <div className="flex items-center space-x-2">
              <ModelStatusBadge
                status={model.status}
                progress={model.progress}
                errorMessage={model.errorMessage}
              />
              {model.status !== 'Downloaded' && model.status !== 'Downloading' && (
                <button
                  onClick={() => handleDownload(model)}
                  disabled={baseResourcePath === null} // Button is only rendered if not Downloading or Downloaded
                  className={`px-3 py-1 text-sm font-medium rounded text-white ${
                    baseResourcePath === null
                      ? 'bg-gray-400 cursor-not-allowed'
                      : 'bg-blue-600 hover:bg-blue-700'
                  }`}
                >
                  Download
                </button>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
};

export default ModelDownloader;