import React from 'react';

interface ComfyUIStatusIndicatorProps {
  status: 'Idle' | 'Setting Up' | 'Installing Dependencies' | 'Starting Sidecar' | 'Running' | 'Stopped' | 'Error';
  errorMessage?: string | null;
}

const ComfyUIStatusIndicator: React.FC<ComfyUIStatusIndicatorProps> = ({ status, errorMessage }) => {
  let statusColor = 'gray';
  let statusText = status;

  switch (status) {
    case 'Running':
      statusColor = 'green';
      break;
    case 'Error':
      statusColor = 'red';
      break;
    case 'Setting Up':
    case 'Installing Dependencies':
    case 'Starting Sidecar':
      statusColor = 'yellow';
      break;
    case 'Stopped':
      statusColor = 'gray';
      break;
    case 'Idle':
    default:
      statusColor = 'gray';
      break;
  }

  return (
    <div className={`text-${statusColor}-600 font-semibold`}>
      ComfyUI Status: {statusText}
    </div>
  );
};

export default ComfyUIStatusIndicator;