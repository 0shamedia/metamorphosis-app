'use client';

import React from 'react';

interface StyledToggleProps {
  isActive: boolean; // True for 'face' (left), false for 'body' (right)
  onToggle: (isActive: boolean) => void;
  // Removed option1Label and option2Label
  // selectedOption prop is replaced by isActive
}

const StyledToggle: React.FC<StyledToggleProps> = ({
  isActive,
  onToggle
}) => {
  // const isOption1Selected = selectedOption === 'option1'; // Old logic

  return (
    // Removed outer buttons, container centers just the switch
    <div className="flex items-center justify-center my-3">
      <div
        className="relative w-12 h-7 bg-gray-700 rounded-full cursor-pointer p-0.5 flex items-center transition-all duration-300 ease-in-out" // Slightly smaller
        onClick={() => onToggle(!isActive)}
      >
        {/* Track background - can be one element that changes or two that fade */}
        {/* Active (e.g., Face) state color - Blue/Purple */}
        <div className={`absolute inset-0.5 rounded-full transition-opacity duration-300 ease-in-out
                      ${isActive
                        ? 'bg-gradient-to-r from-blue-500 to-purple-600 opacity-100'
                        : 'opacity-0'}`}
        />
        {/* Inactive (e.g., Body) state color - Purple/Pink */}
        <div className={`absolute inset-0.5 rounded-full transition-opacity duration-300 ease-in-out
                      ${!isActive
                        ? 'bg-gradient-to-r from-purple-600 to-pink-500 opacity-100'
                        : 'opacity-0'}`}
        />
        {/* Handle */}
        <div
          className={`absolute w-5 h-5 bg-gradient-to-br from-gray-100 to-gray-300 rounded-full shadow-lg transform transition-transform duration-300 ease-in-out ring-1 ring-black/10`}
          style={{
            top: 'calc(50% - 10px)',
            left: isActive ? '3px' : 'calc(100% - 20px - 3px)',
          }}
        />
      </div>
    </div>
  );
};

export default StyledToggle;