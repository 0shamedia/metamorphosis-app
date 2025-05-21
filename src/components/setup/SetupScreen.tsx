'use client';

import { useEffect, useState, useRef, useCallback } from 'react'; // Added useCallback
import SetupModelDownloader from './SetupModelDownloader'; // Import the new component
import SetupCustomNodeInstallerStatus from './SetupCustomNodeInstallerStatus'; // Import new component
import type {
  CustomNodeCloneStartPayload,
  CustomNodeCloneSuccessPayload,
  CustomNodeCloneFailedPayload,
  CustomNodeAlreadyExistsPayload,
  PythonVersionDetectedPayload,
  InsightfaceWheelDownloadStartPayload,
  InsightfaceWheelDownloadProgressPayload,
  // InsightfaceWheelDownloadCompletePayload, // No payload
  PackageInstallStartPayload,
  PackageInstallSuccessPayload,
  PackageInstallFailedPayload,
  PackageAlreadyInstalledPayload,
  PipUpdateStartPayload,
  PipUpdateSuccessPayload,
  PipUpdateFailedPayload,
} from '../../types/events';

// CSS for custom animations (from new example)
const styles = `
  @keyframes shimmer {
    0% {
      background-position: -1000px 0;
    }
    100% {
      background-position: 1000px 0;
    }
  }
  
  .animate-shimmer {
    background: linear-gradient(90deg, rgba(255,255,255,0) 0%, rgba(255,255,255,0.5) 50%, rgba(255,255,255,0) 100%);
    background-size: 1000px 100%;
    animation: shimmer 2s infinite linear;
  }
  
  @keyframes pulse-ring {
    0% {
      transform: scale(0.95);
      opacity: 0.7;
    }
    50% {
      transform: scale(1.05);
      opacity: 0.3;
    }
    100% {
      transform: scale(0.95);
      opacity: 0.7;
    }
  }
  
  .animate-pulse-ring {
    animation: pulse-ring 2s infinite;
  }
`;


// Setup phases in order (from existing)
type SetupPhase = 'checking' | 'installing_comfyui' | 'python_setup' | 'downloading_models' | 'finalizing' | 'complete' | 'error';

interface SetupProgress { // This should align with Rust's SetupProgressPayload
  phase: SetupPhase;
  currentStep: string;
  progress: number; // Progress of the current phase (0-100)
  detailMessage?: string; // Optional
  error?: string; // Optional
}

interface CustomNodeInstallState {
  // ComfyUI_IPAdapter_plus cloning
  cloneStatus: 'idle' | 'cloning' | 'success' | 'failed' | 'exists';
  cloneNodeName?: string;
  cloneError?: string;

  // Python version
  pythonVersion?: string;

  // Insightface wheel (Windows specific)
  wheelDownloadStatus: 'idle' | 'downloading' | 'complete' | 'failed';
  wheelDownloadProgress: number; // 0-100
  wheelDownloadedBytes?: number;
  wheelTotalBytes?: number;
  wheelDownloadUrl?: string;
  wheelError?: string;

  // Pip update
  pipUpdateStatus: 'idle' | 'updating' | 'success' | 'failed';
  pipUpdateError?: string;

  // onnxruntime install
  onnxruntimeInstallStatus: 'idle' | 'installing' | 'success' | 'failed' | 'exists';
  onnxruntimeInstallError?: string;
  onnxruntimeOsHint?: string;

  // insightface install
  insightfaceInstallStatus: 'idle' | 'installing' | 'success' | 'failed' | 'exists';
  insightfaceInstallError?: string;
  insightfaceOsHint?: string;

  // General message for this section
  currentActionMessage?: string;
}

const initialCustomNodeInstallState: CustomNodeInstallState = {
  cloneStatus: 'idle',
  wheelDownloadStatus: 'idle',
  wheelDownloadProgress: 0,
  pipUpdateStatus: 'idle',
  onnxruntimeInstallStatus: 'idle',
  insightfaceInstallStatus: 'idle',
};


// Individual model download status (from existing) - Commented out as new component handles this
// This might become obsolete if `setup-progress` for 'downloading_models' is sufficient.
// interface ModelStatus {
//   id: string;
//   name: string;
//   progress: number; // Progress of this specific model
//   status: 'queued' | 'downloading' | 'verifying' | 'completed' | 'error';
//   errorMessage?: string;
// }

// Phase icons (from existing, unchanged)
const PhaseIcon = ({ phase }: { phase: SetupPhase }) => {
  switch (phase) {
    case 'checking':
      return (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M9 16.2L4.8 12L3.4 13.4L9 19L21 7L19.6 5.6L9 16.2Z" fill="currentColor"/>
        </svg>
      );
    case 'installing_comfyui':
      return (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M19 9H15V3H9V9H5L12 16L19 9ZM5 18V20H19V18H5Z" fill="currentColor"/>
        </svg>
      );
    case 'python_setup':
      return (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M9.4 16.6L4.8 12L9.4 7.4L8 6L2 12L8 18L9.4 16.6ZM14.6 16.6L19.2 12L14.6 7.4L16 6L22 12L16 18L14.6 16.6Z" fill="currentColor"/>
        </svg>
      );
    case 'downloading_models':
      return (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M19.35 10.04C18.67 6.59 15.64 4 12 4C9.11 4 6.6 5.64 5.35 8.04C2.34 8.36 0 10.91 0 14C0 17.31 2.69 20 6 20H19C21.76 20 24 17.76 24 15C24 12.36 21.95 10.22 19.35 10.04ZM17 13L12 18L7 13H10V9H14V13H17Z" fill="currentColor"/>
        </svg>
      );
    case 'finalizing':
      return (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M12 2C6.48 2 2 6.48 2 12C2 17.52 6.48 22 12 22C17.52 22 22 17.52 22 12C22 6.48 17.52 2 12 2ZM10 17L5 12L6.41 10.59L10 14.17L17.59 6.58L19 8L10 17Z" fill="currentColor"/>
        </svg>
      );
    default: // Also for 'error' and 'complete' if not handled by specific UI
      return (
        <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M12 2C6.48 2 2 6.48 2 12C2 17.52 6.48 22 12 22C17.52 22 22 17.52 22 12C22 6.48 17.52 2 12 2ZM13 17H11V15H13V17ZM13 13H11V7H13V13Z" fill="currentColor"/>
        </svg>
      );
  }
};

interface SetupScreenProps {
  onComplete: () => void;
}

export default function SetupScreenComponent({ onComplete }: SetupScreenProps) { // Renamed to avoid conflict
  const [setupProgress, setSetupProgress] = useState<SetupProgress>({
    phase: 'checking',
    currentStep: 'Checking system requirements...',
    progress: 0,
    detailMessage: 'Preparing to set up Metamorphosis...' // Default detail message
  });
  const [customNodeInstallState, setCustomNodeInstallState] = useState<CustomNodeInstallState>(initialCustomNodeInstallState);
  
  // const [models, setModels] = useState<ModelStatus[]>([]); // Commented out as new component handles this
  const [overallProgressDisplay, setOverallProgressDisplay] = useState(0); // For the main progress bar

  const setupStartedRef = useRef(false);

  // Map phases to display names (moved to component scope)
  const phaseNames: {[key in SetupPhase]: string} = {
    checking: 'System Check',
    installing_comfyui: 'ComfyUI Setup',
    python_setup: 'Python Setup',
    downloading_models: 'Model Downloads',
    finalizing: 'Finalizing',
    complete: 'Complete',
    error: 'Error'
  };

  // Calculate overall progress percentage (adapted from existing)
  const calculateOverallProgress = useCallback(() => {
    const weights: {[key in SetupPhase]?: number} = {
      checking: 5, installing_comfyui: 25, python_setup: 20, // Adjusted weights
      downloading_models: 45, finalizing: 5
    };
    const phaseOrder: SetupPhase[] = ['checking', 'installing_comfyui', 'python_setup', 'downloading_models', 'finalizing'];
    const currentPhaseIndex = phaseOrder.indexOf(setupProgress.phase);

    if (setupProgress.phase === 'complete') return 100;
    if (currentPhaseIndex === -1 || setupProgress.phase === 'error') return overallProgressDisplay; // Keep last known on error

    let calculatedProgress = 0;
    for (let i = 0; i < currentPhaseIndex; i++) {
      calculatedProgress += weights[phaseOrder[i]] || 0;
    }
    
    const currentPhaseWeight = weights[setupProgress.phase] || 0;
    // The `setupProgress.progress` from the backend now directly represents the phase's progress (0-100)
    const currentPhaseSpecificProgress = setupProgress.progress;

    // Commented out old model progress calculation logic
    // if (setupProgress.phase === 'downloading_models' && models.length > 0) {
    //     const totalModelProgress = models.reduce((sum, model) => sum + model.progress, 0);
    //     currentPhaseSpecificProgress = totalModelProgress / models.length;
    // }

    calculatedProgress += (currentPhaseSpecificProgress / 100) * currentPhaseWeight;
    return Math.round(calculatedProgress);
  }, [setupProgress.phase, setupProgress.progress]); // Removed overallProgressDisplay from dependencies

  useEffect(() => {
    setOverallProgressDisplay(calculateOverallProgress());
  }, [setupProgress, calculateOverallProgress]); // Removed models from dependencies


  // Simulate the setup process (from existing, kept for fallback)
  const simulateSetupProcess = useCallback(() => {
    console.log('[SetupScreen] Starting FALLBACK setup simulation');
    const phasesForSim: SetupPhase[] = ['checking', 'installing_comfyui', 'python_setup', 'downloading_models', 'finalizing', 'complete'];
    let phaseIndex = 0;
    let currentPhaseProgress = 0;

    // Commented out mock model data for simulation
    // const mockModelsData: ModelStatus[] = [
    //     { id: 'sd-v1-5-pruned', name: 'Stable Diffusion v1.5', progress: 0, status: 'queued' },
    //     { id: 'vae-ft-mse-840000', name: 'VAE Model', progress: 0, status: 'queued' },
    //     { id: 'lora-base', name: 'Character Base LoRA', progress: 0, status: 'queued' }
    // ];

    const interval = setInterval(() => {
      currentPhaseProgress += 20; // Simulate progress within the current phase
      
      // Commented out old model simulation logic
      // let activeModelSimulation = false; // Model simulation part of fallback can be simplified
      // if (setupProgress.phase === 'downloading_models') {
      //     activeModelSimulation = true;
      //     // Simulate model download progress
      //     setModels(prevModels => {
      //         const newModels = [...prevModels];
      //         const downloadingModelIndex = newModels.findIndex(m => m.status === 'downloading');
      //         if (downloadingModelIndex !== -1) {
      //             newModels[downloadingModelIndex].progress = Math.min(newModels[downloadingModelIndex].progress + 10, 100);
      //             if (newModels[downloadingModelIndex].progress === 100) {
      //                 newModels[downloadingModelIndex].status = 'completed';
      //                 const nextQueuedIndex = newModels.findIndex(m => m.status === 'queued');
      //                 if (nextQueuedIndex !== -1) newModels[nextQueuedIndex].status = 'downloading';
      //             }
      //         } else {
      //             const firstQueued = newModels.findIndex(m => m.status === 'queued');
      //             if (firstQueued !== -1) newModels[firstQueued].status = 'downloading';
      //         }
      //         return newModels;
      //     });
      //     // If all models are complete, then this phase's progress is 100
      //     // if (models.every(m => m.status === 'completed')) currentPhaseProgress = 100;
      // }


      if (currentPhaseProgress >= 100) {
        currentPhaseProgress = 0;
        phaseIndex++;
        if (phaseIndex >= phasesForSim.length) {
          clearInterval(interval);
          setTimeout(() => onComplete(), 3000);
          return;
        }
        // Commented out old model simulation logic
        // if (phasesForSim[phaseIndex] === 'downloading_models' && models.length === 0) {
        //     // setModels(mockModelsData); // Initialize models for simulation
        // }
      }
      
      const currentSimPhase = phasesForSim[phaseIndex];
      setSetupProgress(prev => ({
        ...prev,
        phase: currentSimPhase,
        currentStep: `${phaseNames[currentSimPhase]} in progress...`,
        progress: currentPhaseProgress, // Simplified progress for simulation
        detailMessage: `Working on ${phaseNames[currentSimPhase]}... ${currentPhaseProgress}%`
      }));

    }, 1500);
    return () => clearInterval(interval);
  }, [onComplete, phaseNames]); // Removed models, setupProgress.phase from dependency array

  useEffect(() => {
    console.log('[SetupScreen] useEffect for setup listeners running.'); // Log when effect runs
    const setupApplication = async () => {
      if (setupStartedRef.current) {
        console.log('[SetupScreen] Setup already started, skipping setupApplication.'); // Log if skipping
        return;
      }
      setupStartedRef.current = true;
      console.log('[SetupScreen] Starting actual application setup process via Tauri.');
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const { invoke } = await import('@tauri-apps/api/core');
        
        const unlistenSetup = await listen('setup-progress', (event) => {
          console.log('[SetupScreen] setup-progress event received.');
          try {
            const payload = event.payload as SetupProgress;
            setSetupProgress(payload);
            if (payload.phase === 'complete') {
              console.log('[SetupScreen] Received complete phase, calling onComplete.');
              onComplete();
            }
          } catch (error) {
            console.error('[SetupScreen] Error inside setup-progress listener:', error);
            setSetupProgress(prev => ({
              ...prev,
              phase: 'error',
              currentStep: 'Frontend Error',
              detailMessage: `An error occurred processing setup updates: ${error}`,
              error: error instanceof Error ? error.message : String(error),
            }));
          }
        });
        console.log('[SetupScreen] setup-progress listener attached.');
        
        // Commented out old 'model-download-status' listener
        // const unlistenModels = await listen('model-download-status', (event) => {
        //   const payload = event.payload as { models: ModelStatus[] };
        //   setModels(payload.models);
        // });
        
        await invoke('start_application_setup');

        // Listeners for custom node installation
        const unlistenCustomNodeCloneStart = await listen('custom-node-clone-start', (event) => {
          const payload = event.payload as CustomNodeCloneStartPayload;
          setCustomNodeInstallState(prev => ({ ...prev, cloneStatus: 'cloning', cloneNodeName: payload.nodeName, currentActionMessage: `Cloning ${payload.nodeName}...` }));
        });
        const unlistenCustomNodeCloneSuccess = await listen('custom-node-clone-success', (event) => {
          const payload = event.payload as CustomNodeCloneSuccessPayload;
          setCustomNodeInstallState(prev => ({ ...prev, cloneStatus: 'success', currentActionMessage: `${payload.nodeName} cloned successfully.` }));
        });
        const unlistenCustomNodeCloneFailed = await listen('custom-node-clone-failed', (event) => {
          const payload = event.payload as CustomNodeCloneFailedPayload;
          setCustomNodeInstallState(prev => ({ ...prev, cloneStatus: 'failed', cloneError: payload.error, currentActionMessage: `Failed to clone ${payload.nodeName}.` }));
        });
        const unlistenCustomNodeAlreadyExists = await listen('custom-node-already-exists', (event) => {
          const payload = event.payload as CustomNodeAlreadyExistsPayload;
          setCustomNodeInstallState(prev => ({ ...prev, cloneStatus: 'exists', currentActionMessage: `${payload.nodeName} already exists.` }));
        });

        const unlistenPythonVersionDetected = await listen('python-version-detected', (event) => {
          const payload = event.payload as PythonVersionDetectedPayload;
          setCustomNodeInstallState(prev => ({ ...prev, pythonVersion: payload.version }));
        });

        const unlistenInsightfaceWheelDownloadStart = await listen('insightface-wheel-download-start', (event) => {
          const payload = event.payload as InsightfaceWheelDownloadStartPayload;
          setCustomNodeInstallState(prev => ({ ...prev, wheelDownloadStatus: 'downloading', wheelDownloadUrl: payload.url, wheelDownloadProgress: 0, currentActionMessage: 'Downloading Insightface wheel...' }));
        });
        const unlistenInsightfaceWheelDownloadProgress = await listen('insightface-wheel-download-progress', (event) => {
          const payload = event.payload as InsightfaceWheelDownloadProgressPayload;
          const progress = payload.total && payload.total > 0 ? Math.round((payload.downloaded / payload.total) * 100) : 0;
          setCustomNodeInstallState(prev => ({ ...prev, wheelDownloadProgress: progress, wheelDownloadedBytes: payload.downloaded, wheelTotalBytes: payload.total, currentActionMessage: `Downloading Insightface wheel: ${progress}%`}));
        });
        const unlistenInsightfaceWheelDownloadComplete = await listen('insightface-wheel-download-complete', () => {
          setCustomNodeInstallState(prev => ({ ...prev, wheelDownloadStatus: 'complete', wheelDownloadProgress: 100, currentActionMessage: 'Insightface wheel download complete.' }));
        });
        // Assuming a 'insightface-wheel-download-failed' event might exist
        const unlistenInsightfaceWheelDownloadFailed = await listen('insightface-wheel-download-failed', (event) => {
          const payload = event.payload as { error: string }; // Define if not in events.ts
          setCustomNodeInstallState(prev => ({ ...prev, wheelDownloadStatus: 'failed', wheelError: payload.error, currentActionMessage: 'Insightface wheel download failed.' }));
        });


        const unlistenPackageInstallStart = await listen('package-install-start', (event) => {
          const payload = event.payload as PackageInstallStartPayload;
          if (payload.packageName.toLowerCase().includes('onnxruntime')) {
            setCustomNodeInstallState(prev => ({ ...prev, onnxruntimeInstallStatus: 'installing', currentActionMessage: `Installing ${payload.packageName} (${payload.method})...` }));
          } else if (payload.packageName.toLowerCase().includes('insightface')) {
            setCustomNodeInstallState(prev => ({ ...prev, insightfaceInstallStatus: 'installing', currentActionMessage: `Installing ${payload.packageName} (${payload.method})...` }));
          }
        });
        const unlistenPackageInstallSuccess = await listen('package-install-success', (event) => {
          const payload = event.payload as PackageInstallSuccessPayload;
          if (payload.packageName.toLowerCase().includes('onnxruntime')) {
            setCustomNodeInstallState(prev => ({ ...prev, onnxruntimeInstallStatus: 'success', currentActionMessage: `${payload.packageName} installed successfully.` }));
          } else if (payload.packageName.toLowerCase().includes('insightface')) {
            setCustomNodeInstallState(prev => ({ ...prev, insightfaceInstallStatus: 'success', currentActionMessage: `${payload.packageName} installed successfully.` }));
          }
        });
        const unlistenPackageInstallFailed = await listen('package-install-failed', (event) => {
          const payload = event.payload as PackageInstallFailedPayload;
          if (payload.packageName.toLowerCase().includes('onnxruntime')) {
            setCustomNodeInstallState(prev => ({ ...prev, onnxruntimeInstallStatus: 'failed', onnxruntimeInstallError: payload.error, onnxruntimeOsHint: payload.osHint, currentActionMessage: `Failed to install ${payload.packageName}.` }));
          } else if (payload.packageName.toLowerCase().includes('insightface')) {
            setCustomNodeInstallState(prev => ({ ...prev, insightfaceInstallStatus: 'failed', insightfaceInstallError: payload.error, insightfaceOsHint: payload.osHint, currentActionMessage: `Failed to install ${payload.packageName}.` }));
          }
        });
        const unlistenPackageAlreadyInstalled = await listen('package-already-installed', (event) => {
          const payload = event.payload as PackageAlreadyInstalledPayload;
          if (payload.packageName.toLowerCase().includes('onnxruntime')) {
            setCustomNodeInstallState(prev => ({ ...prev, onnxruntimeInstallStatus: 'exists', currentActionMessage: `${payload.packageName} already installed.` }));
          } else if (payload.packageName.toLowerCase().includes('insightface')) {
            setCustomNodeInstallState(prev => ({ ...prev, insightfaceInstallStatus: 'exists', currentActionMessage: `${payload.packageName} already installed.` }));
          }
        });

        const unlistenPipUpdateStart = await listen('pip-update-start', () => {
          setCustomNodeInstallState(prev => ({ ...prev, pipUpdateStatus: 'updating', currentActionMessage: 'Updating pip...' }));
        });
        const unlistenPipUpdateSuccess = await listen('pip-update-success', () => {
          setCustomNodeInstallState(prev => ({ ...prev, pipUpdateStatus: 'success', currentActionMessage: 'Pip updated successfully.' }));
        });
        const unlistenPipUpdateFailed = await listen('pip-update-failed', (event) => {
          const payload = event.payload as PipUpdateFailedPayload;
          setCustomNodeInstallState(prev => ({ ...prev, pipUpdateStatus: 'failed', pipUpdateError: payload.error, currentActionMessage: 'Pip update failed.' }));
        });

        return () => {
          unlistenSetup();
          // unlistenModels(); // Ensure old model listener cleanup is also commented
          unlistenCustomNodeCloneStart();
          unlistenCustomNodeCloneSuccess();
          unlistenCustomNodeCloneFailed();
          unlistenCustomNodeAlreadyExists();
          unlistenPythonVersionDetected();
          unlistenInsightfaceWheelDownloadStart();
          unlistenInsightfaceWheelDownloadProgress();
          unlistenInsightfaceWheelDownloadComplete();
          unlistenInsightfaceWheelDownloadFailed();
          unlistenPackageInstallStart();
          unlistenPackageInstallSuccess();
          unlistenPackageInstallFailed();
          unlistenPackageAlreadyInstalled();
          unlistenPipUpdateStart();
          unlistenPipUpdateSuccess();
          unlistenPipUpdateFailed();
        };
      } catch (error) {
        console.error('[SetupScreen] Tauri API interaction failed:', error);
        setSetupProgress(prev => ({ ...prev, phase: 'error', error: 'Failed to connect to backend. Using simulation.' }));
        simulateSetupProcess(); // Fallback to simulation
      }
    };
    
    setupApplication();
    // No explicit return for cleanup from setupApplication as listeners are cleaned up if it completes
  }, [onComplete, simulateSetupProcess]);

  const handleRetry = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      setSetupProgress({ phase: 'checking', currentStep: 'Retrying setup...', progress: 0, detailMessage: 'Attempting to restart setup process...' });
      // setModels([]); // Reset models - Commented out
      setCustomNodeInstallState(initialCustomNodeInstallState); // Reset custom node state
      setupStartedRef.current = false; // Allow setup to start again
      await invoke('retry_application_setup');
    } catch (error) {
      console.error('[SetupScreen] Retry command failed:', error);
      setSetupProgress(prev => ({ ...prev, phase: 'error', error: 'Retry failed. Using simulation.' }));
      simulateSetupProcess();
    }
  };
  
  const phaseDisplayOrder: SetupPhase[] = ['checking', 'installing_comfyui', 'python_setup', 'downloading_models', 'finalizing'];
  const currentPhaseVisualIndex = phaseDisplayOrder.indexOf(setupProgress.phase);

  return (
    <>
      <style>{styles}</style>
      <div className="min-h-screen w-screen bg-gradient-to-br from-purple-50 via-white to-pink-50 flex flex-col items-center justify-center p-6">
        <div className="w-full max-w-3xl">
          <div className="mb-8 text-center">
            <h1 className="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-purple-700 to-pink-600 mb-2">
              Setting Up Metamorphosis
            </h1>
            <p className="text-gray-600 mb-4">
              Please wait while we prepare your experience
            </p>
            <div className="w-full h-1.5 bg-gray-100 rounded-full overflow-hidden relative">
              <div 
                className="h-full bg-gradient-to-r from-purple-500 to-pink-500 transition-all duration-300 ease-out absolute"
                style={{ width: `${overallProgressDisplay}%` }}
              ></div>
              <div className="absolute inset-0 animate-shimmer"></div>
            </div>
            <p className="text-sm text-gray-500 mt-1">
              Overall Progress: {overallProgressDisplay}%
            </p>
          </div>
          
          <div className="bg-white rounded-xl shadow-lg overflow-hidden">
            <div className="bg-gray-50 px-6 py-4 border-b border-gray-100">
              <div className="flex flex-wrap justify-between gap-4">
                {phaseDisplayOrder.map((phase, index) => {
                  const isComplete = index < currentPhaseVisualIndex;
                  const isActive = index === currentPhaseVisualIndex && setupProgress.phase !== 'complete' && setupProgress.phase !== 'error';
                  
                  return (
                    <div 
                      key={phase}
                      className={`flex items-center ${
                        isComplete ? 'text-green-600' : 
                        isActive ? 'text-purple-700' : 
                        'text-gray-400'
                      }`}
                    >
                      <div className={`w-8 h-8 rounded-full flex items-center justify-center ${
                        isComplete ? 'bg-green-100' : 
                        isActive ? 'bg-purple-100' : 
                        'bg-gray-100'
                      }`}>
                        <PhaseIcon phase={phase} />
                      </div>
                      <span className={`ml-2 text-sm font-medium ${
                        isComplete ? 'text-green-800' : 
                        isActive ? 'text-purple-900' : 
                        'text-gray-500'
                      }`}>
                        {phaseNames[phase]}
                      </span>
                      {index < phaseDisplayOrder.length - 1 && (
                        <div className={`w-8 h-px mx-1 ${
                          index < currentPhaseVisualIndex ? 'bg-green-400' : 'bg-gray-200'
                        }`}></div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
            
            <div className="p-6">
              <div className="mb-6">
                <h2 className="text-lg font-semibold text-gray-800">
                  {setupProgress.currentStep}
                </h2>
                <p className="text-gray-600 mt-1">
                  {setupProgress.detailMessage}
                </p>
              </div>
              
              {setupProgress.phase !== 'complete' && setupProgress.phase !== 'error' && (
                <div className="bg-gray-50 rounded-lg p-4 mb-6">
                  <div className="flex justify-between mb-2">
                    <span className="text-sm font-medium text-gray-700">Current Step Progress</span>
                    <span className="text-sm font-medium text-purple-700">
                      {Math.round(setupProgress.progress)}%
                    </span>
                  </div>
                  <div className="w-full h-2 bg-gray-200 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-purple-600 transition-all duration-300 ease-out"
                      style={{ width: `${setupProgress.progress}%` }}
                    ></div>
                  </div>
                </div>
              )}

              {/* Custom Node Installation Status Display */}
              <SetupCustomNodeInstallerStatus installState={customNodeInstallState} />
              
              {/* Render SetupModelDownloader when in the downloading_models phase */}
              {setupProgress.phase === 'downloading_models' && (
                <div className="mt-4">
                  <SetupModelDownloader />
                </div>
              )}
              
              {/* Old model download UI section commented out */}
              {/* {setupProgress.phase === 'downloading_models' && models.length > 0 && (
                <div className="bg-gray-50 rounded-lg p-4">
                  <h3 className="font-medium text-gray-700 mb-3">Model Downloads</h3>
                  <div className="space-y-4">
                    {models.map(model => (
                      <div key={model.id} className="bg-white p-3 rounded-lg border border-gray-100 shadow-sm">
                        <div className="flex items-center justify-between mb-2">
                          <span className="font-medium text-gray-800">{model.name}</span>
                          <span className={`text-xs px-2 py-0.5 rounded-full ${
                            model.status === 'completed' ? 'bg-green-100 text-green-800' :
                            model.status === 'downloading' ? 'bg-blue-100 text-blue-800' :
                            'bg-gray-100 text-gray-800'
                          }`}>
                            {model.status === 'completed' ? 'Complete' :
                             model.status === 'downloading' ? `Downloading ${Math.round(model.progress)}%` :
                             model.status === 'error' ? `Error: ${model.errorMessage || 'Failed'}` :
                             'Queued'}
                          </span>
                        </div>
                        <div className="w-full h-1.5 bg-gray-100 rounded-full overflow-hidden">
                          <div
                            className={`h-full ${
                              model.status === 'completed' ? 'bg-green-500' :
                              model.status === 'downloading' ? 'bg-blue-500' :
                              model.status === 'error' ? 'bg-red-500' :
                              'bg-gray-300'
                            } transition-all duration-300 ease-out`}
                            style={{ width: `${model.progress}%` }}
                          ></div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )} */}
              
              {setupProgress.phase === 'complete' && (
                <div className="bg-green-50 rounded-lg p-6 text-center">
                  <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-green-100 text-green-500 mb-4">
                    <svg className="w-10 h-10" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                  </div>
                  <h3 className="text-xl font-semibold text-green-800 mb-2">Setup Complete!</h3>
                  <p className="text-green-600">
                    Metamorphosis is ready to use. Taking you to the title screen...
                  </p>
                  <div className="mt-6 inline-block">
                    <div className="relative">
                      <div className="absolute -inset-4 rounded-full bg-green-200 opacity-30 animate-pulse-ring"></div>
                      <button 
                        onClick={onComplete}
                        className="relative z-10 bg-green-600 text-white px-6 py-2 rounded-full font-medium hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2"
                      >
                        Start Game
                      </button>
                    </div>
                  </div>
                </div>
              )}

              {setupProgress.phase === 'error' && (
                <div className="bg-red-50 rounded-lg p-6 text-center">
                   <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-100 text-red-500 mb-4">
                    <svg className="w-10 h-10" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                    </svg>
                  </div>
                  <h3 className="text-xl font-semibold text-red-800 mb-2">Setup Error</h3>
                  <p className="text-red-600 mb-4">
                    An error occurred during setup: {setupProgress.error || 'Unknown error.'}
                  </p>
                  <button 
                    onClick={handleRetry}
                    className="bg-red-600 text-white px-6 py-2 rounded-full font-medium hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
                  >
                    Retry Setup
                  </button>
                </div>
              )}
            </div>
            
            <div className="px-6 py-4 bg-gray-50 border-t border-gray-100">
              <p className="text-xs text-gray-500">
                This setup process installs all required components for Metamorphosis, including ComfyUI, Python environment, and the AI models needed for character generation.
              </p>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
