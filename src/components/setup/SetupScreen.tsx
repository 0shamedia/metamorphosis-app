'use client';

import React, { useEffect, useState, useRef, useCallback } from 'react';
import SetupModelDownloader from './SetupModelDownloader';
import SetupCustomNodeInstallerStatus from './SetupCustomNodeInstallerStatus';
import { SetupPhase, SetupProgress } from './ui/setupUITypes'; // Import shared types
import SetupOverallProgressDisplay from './ui/SetupOverallProgressDisplay';
import SetupPhaseTracker from './ui/SetupPhaseTracker';
import SetupStepDetailsDisplay from './ui/SetupStepDetailsDisplay';
import SetupCurrentPhaseProgress from './ui/SetupCurrentPhaseProgress';
import SetupCompletionDisplay from './ui/SetupCompletionDisplay';
import SetupErrorDisplay from './ui/SetupErrorDisplay';

import type {
  CustomNodeCloneStartPayload,
  CustomNodeCloneSuccessPayload,
  CustomNodeCloneFailedPayload,
  CustomNodeAlreadyExistsPayload,
  PythonVersionDetectedPayload,
  InsightfaceWheelDownloadStartPayload,
  InsightfaceWheelDownloadProgressPayload,
  PackageInstallStartPayload,
  PackageInstallSuccessPayload,
  PackageInstallFailedPayload,
  PackageAlreadyInstalledPayload,
  PipUpdateStartPayload,
  PipUpdateSuccessPayload,
  PipUpdateFailedPayload,
} from '../../types/events';

// CSS for custom animations
const styles = `
  @keyframes shimmer {
    0% { background-position: -1000px 0; }
    100% { background-position: 1000px 0; }
  }
  .animate-shimmer {
    background: linear-gradient(90deg, rgba(255,255,255,0) 0%, rgba(255,255,255,0.5) 50%, rgba(255,255,255,0) 100%);
    background-size: 1000px 100%;
    animation: shimmer 2s infinite linear;
  }
  @keyframes pulse-ring {
    0% { transform: scale(0.95); opacity: 0.7; }
    50% { transform: scale(1.05); opacity: 0.3; }
    100% { transform: scale(0.95); opacity: 0.7; }
  }
  .animate-pulse-ring { animation: pulse-ring 2s infinite; }
`;

// CustomNodeInstallState remains here as it's specific to this screen's logic
interface CustomNodeInstallState {
  cloneStatus: 'idle' | 'cloning' | 'success' | 'failed' | 'exists';
  cloneNodeName?: string;
  cloneError?: string;
  pythonVersion?: string;
  wheelDownloadStatus: 'idle' | 'downloading' | 'complete' | 'failed';
  wheelDownloadProgress: number;
  wheelDownloadedBytes?: number;
  wheelTotalBytes?: number;
  wheelDownloadUrl?: string;
  wheelError?: string;
  pipUpdateStatus: 'idle' | 'updating' | 'success' | 'failed';
  pipUpdateError?: string;
  onnxruntimeInstallStatus: 'idle' | 'installing' | 'success' | 'failed' | 'exists';
  onnxruntimeInstallError?: string;
  onnxruntimeOsHint?: string;
  insightfaceInstallStatus: 'idle' | 'installing' | 'success' | 'failed' | 'exists';
  insightfaceInstallError?: string;
  insightfaceOsHint?: string;
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
  const hasCompletionBeenHandledRef = useRef(false); // For navigation fix

  // Map phases to display names
  const phaseNames: { [key in SetupPhase]: string } = {
    checking: 'System Check',
    installing_comfyui: 'ComfyUI Setup', // Retained for type completeness, though not in active display order
    python_setup: 'Python Setup',
    installing_custom_nodes: 'Custom Nodes',
    verifying_dependencies: 'Verifying Setup', // Added new phase
    downloading_models: 'Model Downloads',
    finalizing: 'Finalizing',
    complete: 'Complete',
    error: 'Error'
  };

  // Calculate overall progress percentage
  const calculateOverallProgress = useCallback(() => {
    const weights: {[key in SetupPhase]?: number} = {
      checking: 5,
      python_setup: 20,
      installing_custom_nodes: 20,
      verifying_dependencies: 10,
      downloading_models: 40,
      finalizing: 5
      // 'installing_comfyui' is removed from weighted phases
    };
    const phaseOrder: SetupPhase[] = ['checking', 'python_setup', 'installing_custom_nodes', 'verifying_dependencies', 'downloading_models', 'finalizing', 'complete']; // Added 'complete'
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
            
            setSetupProgress(prev => {
              const currentPhaseIndex = phaseDisplayOrder.indexOf(prev.phase);
              const receivedPhaseIndex = phaseDisplayOrder.indexOf(payload.phase);

              let newProgress = prev;
              // Update if:
              // 1. The new phase is 'complete' or 'error'.
              // 2. Or, the new phase is the same as the current phase (allowing updates to step/detail/progress within the same phase).
              // 3. Or, the new phase is a subsequent phase in the display order.
              if (payload.phase === 'complete' || payload.phase === 'error' ||
                  (receivedPhaseIndex !== -1 && receivedPhaseIndex >= currentPhaseIndex)
              ) {
                // If the phase is the same or later (or special 'complete'/'error'), accept the new payload.
                // This ensures that changes to currentStep, detailMessage, or progress within the same phase are reflected.
                console.log(`[SetupScreen] Updating progress. New: phase=${payload.phase}, step='${payload.currentStep}', prog=${payload.progress}, detail='${payload.detailMessage}'. Prev: phase=${prev.phase}, step='${prev.currentStep}', prog=${prev.progress}, detail='${prev.detailMessage}'`);
                newProgress = payload;
              } else {
                console.log(`[SetupScreen] Ignoring progress update (phase out of order). Received: ${payload.phase} (${payload.progress}%) (idx ${receivedPhaseIndex}). Current: ${prev.phase} (${prev.progress}%) (idx ${currentPhaseIndex}).`);
              }
              
              // Navigation fix: Call onComplete only once when transitioning to 'complete'
              if (newProgress.phase === 'complete' && !hasCompletionBeenHandledRef.current) {
                if (typeof onComplete === 'function') {
                  console.log('[SetupScreen] Transitioning to complete state. Calling onComplete().');
                  onComplete();
                  hasCompletionBeenHandledRef.current = true; // Mark as handled
                  console.log('[SetupScreen] onComplete() has been called and completion handled.');
                } else {
                  console.error('[SetupScreen] onComplete is NOT a function! Type:', typeof onComplete);
                }
              } else if (newProgress.phase === 'complete' && hasCompletionBeenHandledRef.current) {
                console.log('[SetupScreen] Completion already handled, not calling onComplete again for phase:', newProgress.phase);
              }
              return newProgress;
            });
            // console.log(`[SetupScreen] Current phase after setSetupProgress: ${payload.phase}`); // Log updated phase, be careful with stale closures here
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

        // Listener for granular pip output
        const unlistenPipOutput = await listen('pip-output', (event) => {
          console.log('[SetupScreen] pip-output event received.');
          const payload = event.payload as { packageName: string; output: string; stream: 'stdout' | 'stderr' };
          // Update the detail message with the pip output line
          setSetupProgress(prev => ({
            ...prev,
            detailMessage: `${payload.packageName} (${payload.stream}): ${payload.output}`
          }));
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
          unlistenPipOutput(); // Cleanup the new listener
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
      // setModels([]); // Reset models
      setCustomNodeInstallState(initialCustomNodeInstallState);
      setupStartedRef.current = false;
      hasCompletionBeenHandledRef.current = false; // Reset completion handler on retry
      await invoke('retry_application_setup');
    } catch (error) {
      console.error('[SetupScreen] Retry command failed:', error);
      setSetupProgress(prev => ({ ...prev, phase: 'error', error: 'Retry failed. Using simulation.' }));
      simulateSetupProcess();
    }
  };
  
  const phaseDisplayOrder: SetupPhase[] = ['checking', 'python_setup', 'installing_custom_nodes', 'verifying_dependencies', 'downloading_models', 'finalizing'];
  const currentPhaseVisualIndex = phaseDisplayOrder.indexOf(setupProgress.phase);

  return (
    <>
      <style>{styles}</style>
      <div className="min-h-screen w-screen bg-gradient-to-br from-purple-50 via-white to-pink-50 flex flex-col items-center justify-center p-6">
        <div className="w-full max-w-3xl">
          <SetupOverallProgressDisplay overallProgressDisplay={overallProgressDisplay} />
          
          <div className="bg-white rounded-xl shadow-lg overflow-hidden">
            <SetupPhaseTracker
              phaseDisplayOrder={phaseDisplayOrder}
              currentPhaseVisualIndex={currentPhaseVisualIndex}
              setupProgress={setupProgress}
              phaseNames={phaseNames}
            />
            
            <div className="p-6">
              <SetupStepDetailsDisplay
                currentStep={setupProgress.currentStep}
                detailMessage={setupProgress.detailMessage}
              />
              
              <SetupCurrentPhaseProgress setupProgress={setupProgress} />

              {(setupProgress.phase === 'python_setup' || setupProgress.phase === 'installing_custom_nodes') && (
                <SetupCustomNodeInstallerStatus installState={customNodeInstallState} />
              )}
              
              {setupProgress.phase === 'downloading_models' && (
                <div className="mt-4">
                  <SetupModelDownloader />
                </div>
              )}
              
              {setupProgress.phase === 'complete' && (
                <SetupCompletionDisplay onComplete={onComplete} />
              )}

              {setupProgress.phase === 'error' && (
                <SetupErrorDisplay error={setupProgress.error} handleRetry={handleRetry} />
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
