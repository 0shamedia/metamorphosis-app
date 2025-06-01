'use client';

interface SetupOverallProgressDisplayProps {
  overallProgressDisplay: number;
}

export default function SetupOverallProgressDisplay({ overallProgressDisplay }: SetupOverallProgressDisplayProps) {
  return (
    <div className="mb-8 text-center">
      <h1 className="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-purple-700 to-pink-600 mb-2">
        Setting Up Metamorphosis
      </h1>
      <p className="text-gray-600 mb-4">
        Please wait while we prepare your experience
      </p>
      <div className="w-full h-1.5 bg-gray-100 rounded-full overflow-hidden relative">
        <div
          className="h-full bg-gradient-to-r from-purple-500 to-pink-500 transition-all duration-300 ease-out absolute"
          style={{ width: `${overallProgressDisplay}%` }}
        ></div>
        <div className="absolute inset-0 animate-shimmer"></div>
      </div>
      <p className="text-sm text-gray-500 mt-1">
        Overall Progress: {overallProgressDisplay}%
      </p>
    </div>
  );
}