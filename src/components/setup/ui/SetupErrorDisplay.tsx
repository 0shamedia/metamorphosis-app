'use client';

import React from 'react';

interface SetupErrorDisplayProps {
  error?: string;
  handleRetry: () => Promise<void> | void; // Allow sync or async retry
}

export default function SetupErrorDisplay({ error, handleRetry }: SetupErrorDisplayProps) {
  return (
    <div className="bg-red-50 rounded-lg p-6 text-center">
      <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-100 text-red-500 mb-4">
        <svg className="w-10 h-10" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
        </svg>
      </div>
      <h3 className="text-xl font-semibold text-red-800 mb-2">Setup Error</h3>
      <p className="text-red-600 mb-4">
        An error occurred during setup: {error || 'Unknown error.'}
      </p>
      <button
        onClick={handleRetry}
        className="bg-red-600 text-white px-6 py-2 rounded-full font-medium hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
      >
        Retry Setup
      </button>
    </div>
  );
}