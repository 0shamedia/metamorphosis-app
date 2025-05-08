'use client';

import { useState, useEffect, useCallback } from 'react'; // Import useCallback
import { useRouter } from 'next/navigation'; // Import useRouter

// CSS for custom animations (from new example)
const styles = `
  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-10px); }
  }
  
  @keyframes glow {
    0%, 100% { 
      text-shadow: 0 0 5px rgba(168, 85, 247, 0.5),
                   0 0 15px rgba(168, 85, 247, 0.3); 
    }
    50% { 
      text-shadow: 0 0 10px rgba(168, 85, 247, 0.7),
                   0 0 20px rgba(168, 85, 247, 0.5),
                   0 0 30px rgba(168, 85, 247, 0.3);
    }
  }
  
  @keyframes pulse-light {
    0%, 100% { opacity: 0.7; filter: blur(3px); }
    50% { opacity: 1; filter: blur(5px); }
  }
  
  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  
  @keyframes scaleIn {
    from { 
      opacity: 0; 
      transform: scale(0.9);
    }
    to { 
      opacity: 1;
      transform: scale(1);
    }
  }
  
  @keyframes particle-float {
    0% {
      transform: translateY(0) translateX(0) rotate(0);
      opacity: 0.8;
    }
    33% {
      transform: translateY(-30px) translateX(20px) rotate(45deg);
      opacity: 0.5;
    }
    66% {
      transform: translateY(-50px) translateX(-10px) rotate(90deg);
      opacity: 0.2;
    }
    100% {
      transform: translateY(-70px) translateX(0) rotate(180deg);
      opacity: 0;
    }
  }
  
  .animate-float {
    animation: float 6s ease-in-out infinite;
  }
  
  .animate-glow {
    animation: glow 3s ease-in-out infinite;
  }
  
  .animate-pulse-light {
    animation: pulse-light 3s ease-in-out infinite;
  }
  
  .animate-fade-in-up { /* Base class if needed, though delays are used directly */
    animation: fadeInUp 1s ease-out forwards;
  }
  
  .animate-fade-in-up-delay-1 {
    opacity: 0;
    animation: fadeInUp 1s ease-out forwards 0.2s;
  }
  
  .animate-fade-in-up-delay-2 {
    opacity: 0;
    animation: fadeInUp 1s ease-out forwards 0.4s;
  }
  
  .animate-fade-in-up-delay-3 {
    opacity: 0;
    animation: fadeInUp 1s ease-out forwards 0.6s;
  }
  
  .animate-fade-in {
    animation: fadeIn 2s ease-out forwards;
  }
  
  .animate-scale-in {
    animation: scaleIn 0.5s ease-out forwards;
  }
  
  .animate-particle {
    animation: particle-float 4s ease-out forwards;
  }

  /* Added from existing logo - might need adjustment */
  .bg-pattern { 
    /* Define your pattern background here if needed */
    /* Example: background-image: url('/path/to/texture.png'); */
  }
`;

// Define Particle State interface
interface ParticleState {
  id: number;
  x: number;
  y: number;
  size: number;
  speed: number;
  hue: number;
}

// Floating particles component (from new example)
const BackgroundParticles = () => {
  const [particles, setParticles] = useState<ParticleState[]>([]); // Use ParticleState[] type
  
  const generateParticles = useCallback((count = 10) => {
    const newParticles: ParticleState[] = []; // Explicitly type newParticles
    const now = Date.now();
    for (let i = 0; i < count; i++) {
      newParticles.push({
        id: now + i,
        x: Math.random() * 100,
        y: Math.random() * 100,
        size: Math.random() * 5 + 2,
        speed: Math.random() * 2 + 2,
        hue: Math.floor(Math.random() * 60) + 280 // Purple to pink range
      });
    }
    
    setParticles(prev => [...prev.slice(-50), ...newParticles]); // Keep max 50 + new particles
    
    // Clean up oldest particles after animation completes
    // This needs adjustment if generateParticles is called frequently
    // A better approach might be to filter based on ID/timestamp in the render
  }, []);

  useEffect(() => {
    generateParticles(20); // Initial burst
    const interval = setInterval(() => {
      generateParticles(1); // Add slowly
    }, 500); // Adjusted interval
    
    return () => clearInterval(interval);
  }, [generateParticles]);
  
  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none">
      {particles.map(particle => (
        <div 
          key={particle.id}
          className="absolute rounded-full animate-particle"
          style={{
            left: `${particle.x}%`,
            top: `${particle.y}%`,
            width: `${particle.size}px`,
            height: `${particle.size}px`,
            backgroundColor: `hsl(${particle.hue}, 70%, 70%)`,
            animationDuration: `${particle.speed}s`
          }}
        />
      ))}
    </div>
  );
};


// Chrysalis logo component (from existing, with added animations)
const ChrysalisLogo = () => (
  // Added animate-float from new example
  <div className="relative w-64 h-64 mb-4 animate-float"> 
    {/* Main chrysalis shape with gradient */}
    <div className="absolute inset-0 flex items-center justify-center">
      <div className="relative w-40 h-56">
        {/* Chrysalis outline */}
        <div className="absolute w-full h-full border-2 border-white/40 rounded-full transform scale-y-[1.8] scale-x-[0.9]"></div>
        
        {/* Internal glow - Added animate-pulse-light from new example */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-12 h-12 bg-white rounded-full blur-xl opacity-80 animate-pulse-light"></div>
        
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
        className="absolute w-1 h-16 bg-white blur-sm opacity-20"
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

// Menu Button (from existing, added animation class application)
const MenuButton = ({ children, delay, primary = false, onClick }: { 
  children: React.ReactNode; 
  delay?: string; // Made delay optional and string for class name
  primary?: boolean; 
  onClick: () => void;
}) => (
  <button
    // Added animation class based on delay prop
    className={`relative overflow-hidden w-64 py-3 rounded-lg font-semibold text-lg transform transition-transform duration-200 hover:scale-105 active:scale-95 ${
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

export default function TitleScreenComponent() { // Renamed component
  const [showSplash, setShowSplash] = useState(true); // From new example
  const router = useRouter(); // Use Next.js router

  // From new example: Hide the splash after animation completes
  useEffect(() => {
    const timer = setTimeout(() => {
      setShowSplash(false);
    }, 2500); // Adjust timing as needed
    
    return () => clearTimeout(timer);
  }, []);

  // Updated navigation handlers
  const handleCreateCharacter = () => {
    console.log('Create character clicked');
    router.push('/character-creation'); // Navigate using router
  };
  
  const handleLoadCharacter = () => {
    console.log('Load character clicked');
    // Add navigation or modal logic here
    // router.push('/load-character'); 
  };
  
  const handleModuleLibrary = () => {
    console.log('Module library clicked');
    // Add navigation logic here
    // router.push('/modules');
  };

  // Background stars logic (from existing)
  const stars = Array.from({ length: 100 }, (_, i) => ({
    id: i,
    size: Math.random() * 2 + 1,
    top: Math.random() * 100,
    left: Math.random() * 100,
    opacity: Math.random() * 0.5 + 0.3
  }));
  
  return (
    <>
      <style>{styles}</style>
      <div className="h-screen w-screen overflow-hidden">
        {/* Background with gradient and effects (from existing) */}
        <div className="absolute inset-0 bg-gradient-to-b from-purple-900 via-purple-800 to-pink-800">
          {/* Star-like dots (from existing) */}
          <div className="absolute inset-0 opacity-30">
            {stars.map(star => (
              <div
                key={star.id}
                className="absolute rounded-full bg-white"
                style={{
                  width: `${star.size}px`,
                  height: `${star.size}px`,
                  top: `${star.top}%`,
                  left: `${star.left}%`,
                  opacity: star.opacity
                }}
              />
            ))}
          </div>
          
          {/* Gradient orbs (from existing) */}
          <div className="absolute top-1/4 left-1/4 w-1/2 h-1/2 rounded-full bg-purple-500/20 blur-3xl"></div>
          <div className="absolute bottom-1/4 right-1/4 w-1/2 h-1/2 rounded-full bg-pink-500/20 blur-3xl"></div>
        </div>
        
        {/* Animated particles (from new example) */}
        <BackgroundParticles />
        
        {/* Main content (structure from existing) */}
        <div className="relative z-10 h-full w-full flex flex-col items-center justify-center p-6">
          {/* Logo and title */}
          <div className="flex flex-col items-center mb-12">
            <ChrysalisLogo /> 
            
            {/* Added animate-glow from new example */}
            <h1 className="text-5xl sm:text-6xl font-bold text-white animate-glow">
              Metamorphosis
            </h1>
            {/* Added animate-fade-in from new example */}
            <p className="text-purple-200 mt-2 animate-fade-in opacity-80">
              A Character Creation Experience
            </p>
          </div>
          
          {/* Menu buttons (structure from existing, added delay props for animation) */}
          <div className="flex flex-col space-y-4 items-center">
            <MenuButton primary={true} onClick={handleCreateCharacter}>
              Create Character
            </MenuButton>
            
            <MenuButton delay="2" onClick={handleLoadCharacter}>
              Load Character
            </MenuButton>
            
            <MenuButton delay="3" onClick={handleModuleLibrary}>
              Module Library
            </MenuButton>
          </div>
          
          {/* Developer note (from existing) */}
          <div className="absolute bottom-8 text-xs text-white/50 text-center max-w-xs">
            <p>A modular framework for character creation</p>
            <p className="mt-1">Build your own experiences with the Metamorphosis engine</p>
          </div>
          
          {/* Version number (from existing) */}
          <div className="absolute bottom-4 right-4 text-xs text-white/40">
            Version 0.1.0
          </div>
        </div>
        
        {/* Initial splash animation (from new example) */}
        {showSplash && (
          <div className="absolute inset-0 bg-purple-900 z-50 flex items-center justify-center animate-fade-in">
            {/* Added animate-scale-in */}
            <div className="animate-scale-in"> 
              <ChrysalisLogo />
            </div>
          </div>
        )}
      </div>
    </>
  );
}
