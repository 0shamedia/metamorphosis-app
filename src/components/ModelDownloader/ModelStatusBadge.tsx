import React from 'react';

export type DownloadStatus = 'Not Downloaded' | 'Downloading' | 'Downloaded' | 'Error';

interface ModelStatusBadgeProps {
  status: DownloadStatus;
  progress?: number; // Optional progress percentage for 'Downloading' status
  errorMessage?: string; // Optional error message for 'Error' status
}

const ModelStatusBadge: React.FC<ModelStatusBadgeProps> = ({ status, progress, errorMessage }) => {
  let badgeColor = 'bg-gray-400';
  let textColor = 'text-gray-800';
  const text = status;

  switch (status) {
    case 'Not Downloaded':
      badgeColor = 'bg-yellow-200';
      textColor = 'text-yellow-800';
      break;
    case 'Downloading':
      badgeColor = 'bg-blue-200';
      textColor = 'text-blue-800';
      // Keep text as 'Downloading' to match the type, display progress separately
      break;
    case 'Downloaded':
      badgeColor = 'bg-green-200';
      textColor = 'text-green-800';
      break;
    case 'Error':
      badgeColor = 'bg-red-200';
      textColor = 'text-red-800';
      // Keep text as 'Error' to match the type, display error message separately
      break;
  }

  return (
    <span className={`inline-block px-2 py-1 text-xs font-semibold rounded-full ${badgeColor} ${textColor}`}>
      {status === 'Downloading' && progress !== undefined
        ? `Downloading (${progress}%)`
        : status === 'Error' && errorMessage
        ? `Error: ${errorMessage}`
        : text}
    </span>
  );
};

export default ModelStatusBadge;