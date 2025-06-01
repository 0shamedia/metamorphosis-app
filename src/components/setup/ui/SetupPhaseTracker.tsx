'use client';

import { PhaseIcon, SetupPhase, SetupProgress } from './setupUITypes'; // Import from the new .tsx file

interface SetupPhaseTrackerProps {
  phaseDisplayOrder: SetupPhase[];
  currentPhaseVisualIndex: number;
  setupProgress: SetupProgress; // To determine active/complete for styling
  phaseNames: { [key in SetupPhase]: string };
}

// Re-define PhaseIcon here if not easily importable or move it to a shared utils file
// For now, assuming PhaseIcon can be imported or will be moved.
// If PhaseIcon remains in SetupScreen.tsx, it needs to be exported from there.
// Alternatively, its definition can be copied here.
// For simplicity, let's assume it's made available.

export default function SetupPhaseTracker({
  phaseDisplayOrder,
  currentPhaseVisualIndex,
  setupProgress,
  phaseNames,
}: SetupPhaseTrackerProps) {
  return (
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
              }`}
              >
                <PhaseIcon phase={phase} /> {/* This needs to be resolvable */}
              </div>
              <span className={`ml-2 text-sm font-medium ${
                isComplete ? 'text-green-800' :
                isActive ? 'text-purple-900' :
                'text-gray-500'
              }`}
              >
                {phaseNames[phase]}
              </span>
              {index < phaseDisplayOrder.length - 1 && (
                <div className={`w-8 h-px mx-1 ${
                  index < currentPhaseVisualIndex ? 'bg-green-400' : 'bg-gray-200'
                }`}
                ></div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}