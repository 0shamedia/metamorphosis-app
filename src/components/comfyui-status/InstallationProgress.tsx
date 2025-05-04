import React from 'react';

interface InstallationProgressProps {
  currentStep: string;
  progress: number; // Assuming progress is a percentage (0-100)
  estimatedTime?: string | null;
}

const InstallationProgress: React.FC<InstallationProgressProps> = ({ currentStep, progress, estimatedTime }) => {
  return (
    <div className="mt-2">
      <div className="font-semibold">Installation Progress:</div>
      <div>Current Step: {currentStep}</div>
      <div className="w-full bg-gray-300 rounded-full h-2.5 dark:bg-gray-700">
        <div
          className="bg-blue-600 h-2.5 rounded-full"
          style={{ width: `${progress}%` }}
        ></div>
      </div>
      {estimatedTime && <div>Estimated Time Remaining: {estimatedTime}</div>}
    </div>
  );
};

export default InstallationProgress;