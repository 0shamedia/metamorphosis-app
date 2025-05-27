import React from 'react';
import { GenerationProgress } from '@/store/characterStore'; // Assuming GenerationProgress is exported from store or types

interface AstralMirrorDisplayProps {
  isGenerating: boolean;
  progress: GenerationProgress | null;
  previewImageUrl?: string | null; // For showing the first generated image or selected one
  currentStepName: 'Face' | 'Full Body' | 'Attributes'; // To customize messages
}

const AstralMirrorDisplay: React.FC<AstralMirrorDisplayProps> = ({
  isGenerating,
  progress,
  previewImageUrl,
  currentStepName,
}) => {
  let content;

  if (isGenerating) {
    const progressPercentage = progress && progress.maxSteps > 0 
      ? Math.round((progress.step / progress.maxSteps) * 100) 
      : 0;

    content = (
      <div className="flex flex-col items-center justify-center h-full text-center p-4">
        <div className="relative w-24 h-24 mb-4">
          <div className="absolute inset-0 border-4 border-purple-400 rounded-full animate-ping opacity-50"></div>
          <div className="absolute inset-0 border-4 border-pink-500 rounded-full animate-pulse"></div>
          {progress && progress.maxSteps > 0 && (
            <div className="absolute inset-0 flex items-center justify-center text-xl font-semibold text-white">
              {progressPercentage}%
            </div>
          )}
        </div>
        <p className="text-xl font-semibold text-purple-200 mb-1">
          {progress?.message || `Generating ${currentStepName}...`}
        </p>
        {progress?.currentNodeTitle && progress.maxSteps > 0 && (
          <p className="text-sm text-purple-300">
            {progress.currentNodeTitle}: {progress.step}/{progress.maxSteps}
          </p>
        )}
         {progress?.queuePosition !== null && typeof progress?.queuePosition === 'number' && progress.queuePosition > 0 && (
          <p className="text-xs text-pink-400 mt-1">Queue Position: {progress.queuePosition}</p>
        )}
      </div>
    );
  } else if (previewImageUrl) {
    // Determine aspect ratio for the image container
    const aspectRatioClass = currentStepName === 'Full Body' ? 'aspect-[10/17]' : 'aspect-square';
    content = (
      <div className={`w-full h-full flex items-center justify-center ${aspectRatioClass}`}>
        <img
          src={previewImageUrl}
          alt={`${currentStepName} Preview`}
          className="max-w-full max-h-full object-contain rounded-2xl" // Use max-w/h here
        />
      </div>
    );
  } else {
    let placeholderText = "The Astral Mirror awaits your command.";
    if (currentStepName === 'Attributes') {
        placeholderText = "Define attributes to generate your reflection.";
    } else if (currentStepName === 'Face') {
        placeholderText = "Awaiting face generation...";
    } else if (currentStepName === 'Full Body') {
        placeholderText = "Awaiting full body generation...";
    }
    content = (
      <div className="flex items-center justify-center h-full">
        <p className="text-white/40 text-lg italic p-4 text-center">{placeholderText}</p>
      </div>
    );
  }

  return (
    <div className="mirror w-full h-full bg-black/40 border-2 border-pink-500/40 rounded-2xl flex items-center justify-center relative overflow-hidden shadow-inner-pink-lg backdrop-blur-sm"> {/* Reverted: Removed conditional aspect ratio */}
      {content}
    </div>
  );
};

export default AstralMirrorDisplay;