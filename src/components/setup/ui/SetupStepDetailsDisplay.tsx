'use client';

import React from 'react';

interface SetupStepDetailsDisplayProps {
  currentStep: string;
  detailMessage?: string;
}

export default function SetupStepDetailsDisplay({ currentStep, detailMessage }: SetupStepDetailsDisplayProps) {
  return (
    <div className="mb-6">
      <h2 className="text-lg font-semibold text-gray-800">
        {currentStep}
      </h2>
      <p className="text-gray-600 mt-1">
        {detailMessage}
      </p>
    </div>
  );
}