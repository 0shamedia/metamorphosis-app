import React from 'react';

// Menu Button (from existing, added animation class application)
const MenuButton = ({ children, delay, primary = false, onClick }: { 
  children: React.ReactNode; 
  delay?: string; // Made delay optional and string for class name
  primary?: boolean; 
  onClick: () => void;
}) => (
  <button
    // Added animation class based on delay prop
    className={`relative overflow-hidden w-40 sm:w-44 md:w-48 lg:w-52 xl:w-56 py-1 sm:py-1.5 md:py-2 lg:py-2.5 xl:py-3 rounded-lg font-semibold text-sm sm:text-sm md:text-base lg:text-base xl:text-lg transform transition-transform duration-200 hover:scale-105 active:scale-95 ${
      primary
        ? 'bg-gradient-to-r from-purple-600 to-pink-600 text-white animate-fade-in-up-delay-1'
        : `bg-white/20 backdrop-blur-sm text-white border border-white/30 ${delay ? `animate-fade-in-up-delay-${delay}` : 'animate-fade-in-up'}` // Apply delay class
    }`}
    onClick={onClick}
  >
    <div className="relative z-10">{children}</div>
    {primary && (
      <div className="absolute inset-0 opacity-0 hover:opacity-100 transition-opacity duration-300">
        <div className="absolute inset-0 bg-gradient-to-r from-pink-600 to-purple-600"></div>
      </div>
    )}
  </button>
);

export default MenuButton;