'use client';

import React from 'react';

interface SetupCompletionDisplayProps {
  onComplete: () => void;
}

export default function SetupCompletionDisplay({ onComplete }: SetupCompletionDisplayProps) {
  return (
    <div className="bg-green-50 rounded-lg p-6 text-center">
      <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-green-100 text-green-500 mb-4">
        <svg className="w-10 h-10" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
        </svg>
      </div>
      <h3 className="text-xl font-semibold text-green-800 mb-2">Setup Complete!</h3>
      <p className="text-green-600">
        Metamorphosis is ready to use.
        {typeof onComplete === 'function' ? " Click 'Start Game' to proceed." : " Preparing to transition..."}
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
  );
}