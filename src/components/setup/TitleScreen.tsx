'use client';

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import BackgroundParticles from '../background/BackgroundParticles';
import ChrysalisLogo from '../branding/ChrysalisLogo';
import MenuButton from '../ui/MenuButton';
import VisualSettingsModal from '../settings/VisualSettingsModal';

// CSS for custom animations
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
  
  .animate-fade-in-up {
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

  .bg-pattern { 
    /* Define your pattern background here if needed */
    /* Example: background-image: url('/path/to/texture.png'); */
  }
`;

export default function TitleScreenComponent() {
  const [showSplash, setShowSplash] = useState(true);
  const [showSettings, setShowSettings] = useState(false);
  const router = useRouter();

  useEffect(() => {
    const timer = setTimeout(() => {
      setShowSplash(false);
    }, 100); // Significantly reduced timing
    
    return () => clearTimeout(timer);
  }, []);

  const handleCreateCharacter = () => {
    console.log('Create character clicked');
    router.push('/character-creation');
  };
  
  const handleLoadCharacter = () => {
    console.log('Load character clicked');
    // Add navigation or modal logic here
    // router.push('/load-character'); 
  };
  
  const handleOptionsClick = () => {
    console.log('Options clicked');
    setShowSettings(true);
  };

  const handleCloseSettings = () => {
    setShowSettings(false);
  };

  // Background stars logic
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
      <div className="h-screen w-screen overflow-hidden"> {/* Added overflow-hidden here as well */}
        {/* Background with gradient and effects */}
        <div className="absolute inset-0 bg-gradient-to-b from-purple-900 via-purple-800 to-pink-800">
          {/* Star-like dots */}
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
          
          {/* Gradient orbs */}
          <div className="absolute top-1/4 left-1/4 w-1/2 h-1/2 rounded-full bg-purple-500/20 blur-3xl"></div>
          <div className="absolute bottom-1/4 right-1/4 w-1/2 h-1/2 rounded-full bg-pink-500/20 blur-3xl"></div>
        </div>
        
        <BackgroundParticles />
        
        {/* Main content */}
        <div className="relative z-10 h-full w-full flex flex-col items-center justify-between p-2 sm:p-2 md:p-3 lg:p-4 xl:p-6 overflow-hidden">
          {/* Logo and title */}
          <div className="flex flex-col items-center mb-1 sm:mb-2 md:mb-2 lg:mb-3 xl:mb-8">
            <ChrysalisLogo />
            
            <h1 className="relative z-10 text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-bold text-white animate-glow -mt-8 sm:-mt-10 md:-mt-12 lg:-mt-14 xl:-mt-16">
              Metamorphosis
            </h1>
            <p className="text-xs sm:text-sm md:text-base lg:text-lg xl:text-xl text-purple-200 mt-1 sm:mt-1 md:mt-2 lg:mt-2 xl:mt-3 animate-fade-in opacity-80">
              A Character Creation Experience
            </p>
          </div>
          
          {/* Menu buttons */}
          <div className="flex flex-col space-y-1 sm:space-y-2 md:space-y-2 lg:space-y-3 xl:space-y-4 items-center mt-1 sm:mt-2 md:mt-2 lg:mt-3 xl:mt-8">
            <MenuButton primary={true} onClick={handleCreateCharacter}>
              Create Character
            </MenuButton>
            
            <MenuButton delay="2" onClick={handleLoadCharacter}>
              Load Character
            </MenuButton>
            
            <MenuButton delay="3" onClick={handleOptionsClick}>
              Options
            </MenuButton>
          </div>
          
          {/* Developer note */}
          <div className="mt-auto text-[0.55rem] sm:text-[0.65rem] md:text-xs lg:text-sm text-white/50 text-center max-w-[150px] sm:max-w-[170px] md:max-w-xs lg:max-w-sm pt-4"> {/* Added pt-4 for spacing from buttons */}
            <p>A modular framework for character creation</p>
            <p className="mt-1">Build your own experiences with the Metamorphosis engine</p>
          </div>
          
          {/* Version number */}
          <div className="text-[0.55rem] sm:text-[0.65rem] md:text-xs text-white/40 mt-2 self-end mr-2 sm:mr-3 md:mr-4"> {/* Adjusted to be part of flow, self-end for right alignment */}
            Version 0.1.0
          </div>
          </div>
        
        {/* Initial splash animation */}
        {showSplash && (
          <div className="absolute inset-0 bg-purple-900 z-50 flex items-center justify-center animate-fade-in">
            <div className="animate-scale-in"> 
              <ChrysalisLogo />
            </div>
          </div>
        )}
      </div>

      <VisualSettingsModal show={showSettings} onClose={handleCloseSettings} />
    </>
  );
}
