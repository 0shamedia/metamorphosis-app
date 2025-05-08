'use client';

import { useEffect, useState } from 'react';
// Removed Image from 'next/image' as it's not used in the new logo component directly

// CSS for animations from the new example
const styles = `
  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-10px); }
  }
  
  @keyframes pulse {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 1; }
  }
  
  @keyframes sparkle {
    0% { transform: scale(0.8); opacity: 0.4; }
    100% { transform: scale(1.2); opacity: 0.8; }
  }
  
  @keyframes rotate {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
  
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  
  .animate-float {
    animation: float 6s ease-in-out infinite;
  }
  
  .animate-pulse-custom { /* Renamed to avoid conflict if 'animate-pulse' is a Tailwind class */
    animation: pulse 2s ease-in-out infinite;
  }
  
  .animate-sparkle {
    animation: sparkle 1.5s ease-in-out infinite alternate;
  }
  
  .animate-rotate {
    animation: rotate 20s linear infinite;
  }
  
  .animate-fade-in {
    animation: fadeIn 1.5s ease-out forwards;
  }
`;

// Logo component with butterfly theme from the new example
const ButterflyLogo = () => (
  <div className="relative w-48 h-48 animate-float">
    {/* Main butterfly shape with gradient */}
    <div className="absolute inset-0 bg-gradient-to-br from-purple-500 to-pink-500 rounded-full opacity-20 blur-xl animate-pulse-custom"></div>
    
    <div className="absolute inset-0 flex items-center justify-center">
      <svg viewBox="0 0 100 100" width="100" height="100" className="text-purple-600">
        <path 
          d="M50,30 C60,15 80,10 90,25 C95,35 85,45 75,40 C65,35 60,25 50,30 C40,25 35,35 25,40 C15,45 5,35 10,25 C20,10 40,15 50,30 Z" 
          fill="url(#butterflyGradient)" 
          stroke="white" 
          strokeWidth="1"
        />
        <defs>
          <linearGradient id="butterflyGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#c026d3" /> {/* Fuchsia 600 */}
            <stop offset="100%" stopColor="#db2777" /> {/* Pink 600 */}
          </linearGradient>
        </defs>
      </svg>
    </div>
    
    {/* Sparkles */}
    {[...Array(12)].map((_, i) => (
      <div 
        key={i}
        className="absolute w-1.5 h-1.5 rounded-full bg-purple-300"
        style={{
          left: `${50 + Math.cos(i / 12 * Math.PI * 2) * 45}%`,
          top: `${50 + Math.sin(i / 12 * Math.PI * 2) * 45}%`,
          animation: `sparkle ${1 + Math.random()}s ease-in-out infinite alternate ${Math.random() * 2}s`
        }}
      />
    ))}
    
    {/* Orbital ring */}
    <div className="absolute inset-0 border-4 border-purple-300/20 rounded-full animate-rotate"></div>
  </div>
);

interface SplashScreenProps {
  onComplete: () => void;
}

export default function SplashScreenComponent({ onComplete }: SplashScreenProps) {
  const [loadingText, setLoadingText] = useState('Initializing...');
  const [progress, setProgress] = useState(0); // For visual progress bar
  const [isBackendReady, setIsBackendReady] = useState(false); // Renamed from isReady for clarity
  const [initializationError, setInitializationError] = useState<string | null>(null);
  
  // Backend communication logic
  useEffect(() => {
    console.log('[SplashScreen] Component mounted at:', new Date().toISOString());
    let unlistenInstallation: (() => void) | null = null;
    let unlistenBackend: (() => void) | null = null;

    const setupAndRunBackend = async () => {
      console.log('[SplashScreen] Setting up backend communication...');
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const { invoke } = await import('@tauri-apps/api/core');

        // Listener for Installation Status
        const installationListener = await listen('installation-status', (event) => {
          const payload = event.payload as { step: string; message: string; is_error: boolean };
          console.log(`[SplashScreen] Received installation status:`, payload);
          setLoadingText(payload.message || `Installing dependencies... (${payload.step})`);
          // Update progress based on step? Maybe map steps to percentages?
          // For now, just show messages.
          if (payload.is_error) {
            console.error('[SplashScreen] Error during dependency installation:', payload.message);
            setInitializationError(payload.message || 'Dependency installation failed');
          }
          // Note: InstallationComplete is handled implicitly by ensure_backend_ready resolving
        });
        unlistenInstallation = installationListener; // Store unlisten function

        // Listener for General Backend Status (Sidecar start, errors)
        const backendListener = await listen('backend-status', (event) => {
          const payload = event.payload as { status: string; message: string; isError: boolean };
          console.log(`[SplashScreen] Received backend status:`, payload);

          if (payload.status === 'starting_sidecar') {
            setLoadingText(payload.message || 'Starting ComfyUI backend...');
            setProgress(75); // Example progress update
          } else if (payload.status === 'backend_ready') {
            console.log('[SplashScreen] Backend reported ready status.');
            setLoadingText('Finalizing...'); // Update text for final step
            setProgress(100); // Ensure progress bar completes
            setIsBackendReady(true); // Trigger completion
          } else if (payload.status === 'backend_error' || payload.isError) {
             console.error('[SplashScreen] Backend reported an error:', payload.message);
             const errorMsg = payload.message || 'Unknown backend error';
             setLoadingText(`Error: ${errorMsg}`);
             setInitializationError(errorMsg);
          } else {
             // Handle other potential statuses if needed
             setLoadingText(payload.message || 'Processing...');
          }
        });
        unlistenBackend = backendListener; // Store unlisten function

        // Invoke the command to ensure dependencies and start the backend
        try {
          console.log('[SplashScreen] Invoking ensure_backend_ready command...');
          setLoadingText('Checking backend dependencies...'); // Initial message
          setProgress(10); // Initial progress
          await invoke('ensure_backend_ready');
          console.log('[SplashScreen] invoke ensure_backend_ready completed successfully (or handled error internally).');
          // Success is indicated by the 'backend_ready' event, not the command resolving without error here.
        } catch (err) {
          // This catch block might still be triggered if the command itself fails catastrophically
          // before even starting the Rust logic (e.g., command not found).
          const errorMessage = (err instanceof Error ? err.message : String(err)) || 'Failed to invoke backend readiness check';
          console.error('[SplashScreen] ensure_backend_ready command CATCH block triggered. Error:', errorMessage);
          setLoadingText(`Error: ${errorMessage}`);
          setInitializationError(errorMessage);
        }

      } catch (error) {
        console.error('[SplashScreen] Failed to set up backend communication (Tauri API import failed?):', error);
        const errorMsg = 'Failed to initialize communication with backend.';
        setLoadingText(`Error: ${errorMsg}`);
        setInitializationError(errorMsg);
        // Failsafe: if Tauri API import fails, we can't proceed. Error is shown.
      }
    };

    setupAndRunBackend();

    // Failsafe timeout - adjusted to 3 minutes due to potentially long installs
    const failsafeTimer = setTimeout(() => {
      console.log(`[SplashScreen] Failsafe timer check (3 min): isBackendReady=${isBackendReady}, initializationError=${initializationError}`);
      if (!isBackendReady && !initializationError) { // Only proceed if not ready AND no error encountered
        console.warn('[SplashScreen] Failsafe timeout (3 min) triggered - forcing proceed. This might indicate an issue.');
        setLoadingText('Setup taking longer than expected, proceeding...');
        setIsBackendReady(true); // This will trigger onComplete
        setProgress(100);
      } else if (initializationError) {
        console.log('[SplashScreen] Failsafe timeout (3 min) triggered, but an error was encountered. Not proceeding.');
        // Error message should already be displayed
      } else if (isBackendReady) {
        // Already ready, failsafe not needed.
        console.log('[SplashScreen] Failsafe timeout (3 min) triggered, but backend is already ready.');
      }
    }, 180000); // 3 minutes = 180,000 ms

    // Cleanup function
    return () => {
      console.log('[SplashScreen] Cleaning up listeners and timer');
      if (unlistenInstallation) unlistenInstallation();
      if (unlistenBackend) unlistenBackend();
      clearTimeout(failsafeTimer);
    };
  }, []); // Empty dependency array: run once on mount

  // Effect to call onComplete when backend is ready
  useEffect(() => {
    if (isBackendReady) {
      console.log('[SplashScreen] Backend is ready, calling onComplete after a short delay.');
      const timer = setTimeout(() => {
        onComplete();
      }, 1500); // Delay to allow fade-out or final animation impression
      
      return () => clearTimeout(timer);
    }
  }, [isBackendReady, onComplete]);
  
  return (
    <>
      <style>{styles}</style>
      <div className="h-screen w-screen flex flex-col items-center justify-center bg-gradient-to-br from-purple-100 via-purple-50 to-pink-100 overflow-hidden">
        {/* Background pattern from new example */}
        <div className="absolute inset-0 opacity-20">
          <div className="absolute top-1/4 left-1/4 w-1/2 h-1/2 rounded-full bg-gradient-to-r from-purple-300 to-transparent blur-3xl"></div>
          <div className="absolute bottom-1/4 right-1/4 w-1/2 h-1/2 rounded-full bg-gradient-to-l from-pink-300 to-transparent blur-3xl"></div>
        </div>
        
        <div className="relative z-10 flex flex-col items-center space-y-8 animate-fade-in">
          <ButterflyLogo />
          
          <h1 className="text-4xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-purple-700 to-pink-600">
            Metamorphosis
          </h1>
          
          <div className="w-64 flex flex-col items-center space-y-2">
            <div className="w-full h-1.5 bg-white/30 rounded-full overflow-hidden">
              <div 
                className="h-full bg-gradient-to-r from-purple-500 to-pink-500 transition-all duration-300 ease-out"
                style={{ width: `${progress}%` }}
              ></div>
            </div>
            <p className="text-purple-900/70 text-sm font-medium">{loadingText}</p>
          </div>
        </div>
        
        <div className="absolute bottom-4 flex items-center space-x-1 text-xs text-purple-800/40">
          <span className="font-medium">Version</span>
          <span>0.1.0</span> {/* This could be dynamic later */}
        </div>
      </div>
    </>
  );
}
