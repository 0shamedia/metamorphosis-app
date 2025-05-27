import React from 'react';

// Chrysalis logo component (from existing, with added animations)
const ChrysalisLogo = () => (
  // Added animate-float from new example
  <div className="relative w-24 h-24 sm:w-28 sm:h-28 md:w-36 md:h-36 lg:w-44 lg:h-44 xl:w-52 xl:h-52 mb-1 sm:mb-1 md:mb-2 lg:mb-2 xl:mb-3 animate-float">
    {/* Main chrysalis shape with gradient */}
    <div className="absolute inset-0 flex items-center justify-center">
      <div className="relative w-full h-full">
        {/* Chrysalis outline */}
        <div className="absolute w-full h-full border-2 border-white/40 rounded-full transform scale-y-[1.8] scale-x-[0.9]"></div>
        
        {/* Internal glow - Added animate-pulse-light from new example */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-6 h-6 sm:w-7 sm:h-7 md:w-9 md:h-9 lg:w-11 lg:h-11 xl:w-12 xl:h-12 bg-white rounded-full blur-xl opacity-80 animate-pulse-light"></div>
        
        {/* Purple-pink gradient fill */}
        <div className="absolute inset-0 bg-gradient-to-br from-purple-600 to-pink-500 rounded-full transform scale-y-[1.8] scale-x-[0.9] opacity-40"></div>
        
        {/* Overlay texture for chrysalis - Added bg-pattern class */}
        <div className="absolute inset-0 overflow-hidden rounded-full transform scale-y-[1.8] scale-x-[0.9]">
          <div className="absolute inset-0 opacity-30 bg-pattern"></div>
        </div>
      </div>
    </div>
    
    {/* Additional light beams (from existing) */}
    {[...Array(8)].map((_, i) => (
      <div 
        key={i}
        className="absolute w-px sm:w-1 h-6 sm:h-7 md:h-9 lg:h-11 xl:h-13 bg-white blur-sm opacity-20"
        style={{
          left: '50%',
          top: '50%',
          transformOrigin: 'center',
          transform: `translate(-50%, -50%) rotate(${i * 45}deg)`
        }}
      />
    ))}
    
    {/* Ambient glow (from existing) */}
    <div className="absolute inset-0 rounded-full bg-purple-500/30 blur-3xl"></div>
  </div>
);

export default ChrysalisLogo;