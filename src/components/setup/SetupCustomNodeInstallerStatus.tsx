'use client';

import React from 'react';

// Matches the interface in SetupScreen.tsx
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

interface SetupCustomNodeInstallerStatusProps {
  installState: CustomNodeInstallState;
}

const StatusItem: React.FC<{ label: string; status: string; details?: string | React.ReactNode; error?: string; hint?: string; progress?: number }> = ({
  label,
  status,
  details,
  error,
  hint,
  progress,
}) => {
  let statusColor = 'text-gray-600';
  let statusText = status.charAt(0).toUpperCase() + status.slice(1); // Capitalize

  if (status === 'success' || status === 'exists' || status === 'complete') {
    statusColor = 'text-green-600';
  } else if (status === 'failed') {
    statusColor = 'text-red-600';
  } else if (status === 'installing' || status === 'cloning' || status === 'downloading' || status === 'updating') {
    statusColor = 'text-blue-600';
    statusText = `${statusText}...`;
  }


  return (
    <div className="py-2 px-3 mb-2 bg-slate-50 rounded-md border border-slate-200">
      <div className="flex justify-between items-center">
        <span className="text-sm font-medium text-gray-700">{label}</span>
        <span className={`text-xs font-semibold ${statusColor}`}>{statusText}</span>
      </div>
      {details && <p className="text-xs text-gray-500 mt-0.5">{details}</p>}
      {progress !== undefined && (status === 'downloading' || status === 'installing') && (
        <div className="w-full h-1.5 bg-gray-200 rounded-full overflow-hidden mt-1">
          <div
            className="h-full bg-blue-500 transition-all duration-150 ease-linear"
            style={{ width: `${progress}%` }}
          ></div>
        </div>
      )}
      {error && <p className="text-xs text-red-500 mt-0.5">Error: {error}</p>}
      {hint && <p className="text-xs text-yellow-600 bg-yellow-50 p-1 rounded mt-0.5">Hint: {hint}</p>}
    </div>
  );
};


const SetupCustomNodeInstallerStatus: React.FC<SetupCustomNodeInstallerStatusProps> = ({ installState }) => {
  if (
    installState.cloneStatus === 'idle' &&
    installState.pipUpdateStatus === 'idle' &&
    installState.onnxruntimeInstallStatus === 'idle' &&
    installState.insightfaceInstallStatus === 'idle' &&
    installState.wheelDownloadStatus === 'idle' &&
    !installState.currentActionMessage // Only show if there's no general message
  ) {
    // If all statuses are idle and no general message, don't render anything yet or a placeholder
    return null; 
  }

  return (
    <div className="mt-4 p-4 bg-white rounded-lg shadow">
      <h3 className="text-md font-semibold text-gray-700 mb-3">Custom Node & Dependencies Installation</h3>
      
      {installState.currentActionMessage && (
        <p className="text-sm text-purple-700 mb-3 font-medium">{installState.currentActionMessage}</p>
      )}

      {installState.cloneNodeName && (installState.cloneStatus !== 'idle' || installState.cloneError) && (
        <StatusItem 
          label={`Clone ${installState.cloneNodeName}`} 
          status={installState.cloneStatus}
          error={installState.cloneError}
        />
      )}

      {installState.pythonVersion && (
         <StatusItem label="Python Version Detected" status="success" details={installState.pythonVersion} />
      )}

      {installState.pipUpdateStatus !== 'idle' && (
        <StatusItem
          label="Pip Update"
          status={installState.pipUpdateStatus}
          error={installState.pipUpdateError}
        />
      )}
      
      {installState.wheelDownloadStatus !== 'idle' && (
        <StatusItem 
          label="Insightface Wheel Download (Windows)"
          status={installState.wheelDownloadStatus}
          progress={installState.wheelDownloadProgress}
          details={installState.wheelDownloadStatus === 'downloading' && installState.wheelDownloadedBytes && installState.wheelTotalBytes ? 
            `${(installState.wheelDownloadedBytes / (1024*1024)).toFixed(2)}MB / ${(installState.wheelTotalBytes / (1024*1024)).toFixed(2)}MB` : 
            installState.wheelDownloadUrl ? `From: ${installState.wheelDownloadUrl}` : undefined
          }
          error={installState.wheelError}
        />
      )}

      {installState.onnxruntimeInstallStatus !== 'idle' && (
        <StatusItem 
          label="ONNX Runtime Installation"
          status={installState.onnxruntimeInstallStatus}
          error={installState.onnxruntimeInstallError}
          hint={installState.onnxruntimeOsHint}
        />
      )}

      {installState.insightfaceInstallStatus !== 'idle' && (
        <StatusItem 
          label="Insightface Installation"
          status={installState.insightfaceInstallStatus}
          error={installState.insightfaceInstallError}
          hint={installState.insightfaceOsHint}
        />
      )}
    </div>
  );
};

export default SetupCustomNodeInstallerStatus;