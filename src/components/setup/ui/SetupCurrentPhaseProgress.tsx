'use client';

import React from 'react';
import type { SetupProgress } from './setupUITypes'; // Import from shared types

interface SetupCurrentPhaseProgressProps {
  setupProgress: SetupProgress;
}

export default function SetupCurrentPhaseProgress({ setupProgress }: SetupCurrentPhaseProgressProps) {
  if (setupProgress.phase === 'complete' || setupProgress.phase === 'error' || setupProgress.phase === 'downloading_models') {
    return null; // Don't show this for these phases (downloader has its own UI)
  }

  return (
    <div className="bg-gray-50 rounded-lg p-4 mb-6">
      <h3 className="text-md font-semibold text-gray-700 mb-3">Current Phase Progress</h3>
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
  );
}