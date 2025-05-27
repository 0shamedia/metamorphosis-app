import { useState, useEffect, useCallback } from 'react';

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

export default BackgroundParticles;