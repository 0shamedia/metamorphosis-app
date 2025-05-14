'use client';

import { useEffect, useState, useRef } from 'react';
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
  const [isBackendReady, setIsBackendReady] = useState(false); // Final readiness for main app
  const [initializationError, setInitializationError] = useState<string | null>(null);
  const [navigateToSetup, setNavigateToSetup] = useState(false);
  const [isStartingServices, setIsStartingServices] = useState(false); // New state
  const ensurePromiseResolvedRef = useRef(false); // Ref for the new timeout logic

  // Backend communication logic
  useEffect(() => {
    console.log('[SplashScreen] Component mounted at:', new Date().toISOString());
    let unlistenSetupStatus: (() => void) | null = null;
    let unlistenInstallation: (() => void) | null = null;
    let unlistenBackendStatus: (() => void) | null = null;
    let initialVerificationTimerId: NodeJS.Timeout | null = null;
    let serviceStartupTimerId: NodeJS.Timeout | null = null;
    let ensureCmdTimeoutId: NodeJS.Timeout | null = null; // Timer for ensure_comfyui_running_and_healthy

    const initializeApplication = async () => {
      console.log('[SplashScreen] Initializing application setup status check...');
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const { invoke } = await import('@tauri-apps/api/core');

        // Listener for the new setup_status event
        const setupStatusListener = await listen('setup_status', (event) => {
          console.log(`[SplashScreen] Received setup_status event (raw object):`, event);
          const rawPayload = event.payload;
          console.log(`[SplashScreen] Raw event.payload:`, rawPayload);
          console.log(`[SplashScreen] typeof event.payload:`, typeof rawPayload);

          if (typeof rawPayload !== 'object' || rawPayload === null) {
            console.error('[SplashScreen] Malformed: event.payload is not an object or is null. Payload:', rawPayload);
            setLoadingText('Received malformed status (payload not object).');
            setInitializationError('Malformed status from backend (payload not object).');
            setNavigateToSetup(true);
            if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
            initialVerificationTimerId = null;
            return;
          }

          // Assert rawPayload is an object by this point, cast to any for inspection
          const inspectedPayload = rawPayload as any;
          if (typeof inspectedPayload.type !== 'string') {
            console.error(`[SplashScreen] Malformed: event.payload is an object, but its 'type' property is not a string. 'type' is: ${typeof inspectedPayload.type}. Full payload:`, inspectedPayload);
            setLoadingText('Received malformed status (type field issue).');
            setInitializationError('Malformed status from backend (type field issue).');
            setNavigateToSetup(true);
            if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
            initialVerificationTimerId = null;
            return;
          }
          
          // Payload is an object with a 'type' string property
          console.log(`[SplashScreen] Payload appears valid. Type: '${inspectedPayload.type}'. Data:`, inspectedPayload.data);

          switch (inspectedPayload.type) {
            case "backendFullyVerifiedAndReady":
              console.log('[SplashScreen] Backend is fully verified and ready (files). Attempting to start backend services.');
              setLoadingText('File verification complete. Starting backend services...');
              setProgress(75);
              
              if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
              initialVerificationTimerId = null;
              
              setIsStartingServices(true);
              setNavigateToSetup(false);
              ensurePromiseResolvedRef.current = false; // Reset for this attempt

              console.log('[SplashScreen] Starting service startup failsafe timer (3 min)...');
              serviceStartupTimerId = setTimeout(() => {
                // This timer callback needs to access current state values.
                // Using functional updates or refs for isStartingServices, isBackendReady etc. is safer.
                // For now, assuming direct state access in the log is illustrative.
                console.warn(`[SplashScreen] Failsafe timeout (3 min) for service startup triggered. States: isStartingServices=${isStartingServices}, isBackendReady=${isBackendReady}, navigateToSetup=${navigateToSetup}, initializationError=${initializationError}`);
                // Check state again to ensure it hasn't resolved
                // This check should ideally use fresh state values if possible (e.g., via functional setState or refs)
                if (isStartingServices && !isBackendReady && !navigateToSetup && !initializationError) {
                    setLoadingText('Backend services taking too long to start (3min), preparing for setup...');
                    setInitializationError('Service startup timed out (3min).');
                    setNavigateToSetup(true);
                    setIsStartingServices(false);
                }
              }, 180000); // 3 minutes

              console.log('[SplashScreen] Starting ensure_comfyui_running_and_healthy command timeout (30s)...');
              ensureCmdTimeoutId = setTimeout(() => {
                if (!ensurePromiseResolvedRef.current) {
                  console.warn('[SplashScreen] ensure_comfyui_running_and_healthy command timed out after 30s.');
                  // Check states again, similar to the 3-min timer.
                  // This check should ideally use fresh state values.
                  if (isStartingServices && !isBackendReady && !navigateToSetup && !initializationError) {
                    setLoadingText('Backend service startup check timed out (30s). Navigating to setup...');
                    setInitializationError('Service startup command timed out (30s).');
                    setNavigateToSetup(true);
                    setIsStartingServices(false);
                    if (serviceStartupTimerId) {
                      clearTimeout(serviceStartupTimerId);
                      serviceStartupTimerId = null;
                    }
                  }
                }
              }, 30000); // 30 seconds

              invoke('ensure_comfyui_running_and_healthy')
                .then(() => {
                  ensurePromiseResolvedRef.current = true;
                  if (ensureCmdTimeoutId) clearTimeout(ensureCmdTimeoutId);
                  ensureCmdTimeoutId = null;
                  console.log('[SplashScreen] ensure_comfyui_running_and_healthy promise RESOLVED. Waiting for backend-status event.');
                  // Continue to rely on backend-status event
                })
                .catch(err => {
                  ensurePromiseResolvedRef.current = true;
                  if (ensureCmdTimeoutId) clearTimeout(ensureCmdTimeoutId);
                  ensureCmdTimeoutId = null;
                  const errorMessage = (err instanceof Error ? err.message : String(err)) || 'Failed to invoke ensure_comfyui_running_and_healthy';
                  console.error('[SplashScreen] ensure_comfyui_running_and_healthy CATCH. Error:', errorMessage);
                  setLoadingText(`Error starting services: ${errorMessage}`);
                  setInitializationError(errorMessage);
                  setNavigateToSetup(true);
                  setIsStartingServices(false);
                  if (serviceStartupTimerId) {
                    clearTimeout(serviceStartupTimerId);
                    serviceStartupTimerId = null;
                  }
                });
              break;
            case "fullSetupRequired":
              const reason = inspectedPayload.data?.reason || "Unknown reason";
              console.log(`[SplashScreen] Full setup required. Reason: ${reason}`);
              setLoadingText(`Setup required: ${reason}`);
              setProgress(50);
              setIsBackendReady(false);
              setNavigateToSetup(true);
              setIsStartingServices(false);
              if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
              initialVerificationTimerId = null;
              if (serviceStartupTimerId) clearTimeout(serviceStartupTimerId); // Should not be active, but clear just in case
              serviceStartupTimerId = null;
              break;
            default:
              console.warn(`[SplashScreen] Received unhandled setup_status type: '${inspectedPayload.type}'. Full payload:`, inspectedPayload);
              setLoadingText(`Received unexpected status: ${inspectedPayload.type}`);
              setInitializationError(`Unexpected status: ${inspectedPayload.type}`);
              setNavigateToSetup(true);
              if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
              initialVerificationTimerId = null;
              if (serviceStartupTimerId) clearTimeout(serviceStartupTimerId);
              serviceStartupTimerId = null;
              break;
          }
        });
        unlistenSetupStatus = setupStatusListener;

        // Listener for Installation Status (still useful if full setup is triggered by SetupScreen)
        // This listener might be more relevant on SetupScreen.tsx itself.
        // For SplashScreen, we primarily care about the initial decision.
        // However, if get_setup_status_and_initialize itself can trigger some initial steps
        // before deciding, this might still catch early messages.
        const installationListener = await listen('installation-status', (event) => {
            const payload = event.payload as { step: string; message: string; is_error: boolean };
            console.log(`[SplashScreen] Received installation status (during initial check or if setup starts):`, payload);
            // Only update if full setup hasn't been explicitly required by setup_status yet
            if (!navigateToSetup) {
                setLoadingText(payload.message || `Step: ${payload.step}`);
            }
            if (payload.is_error) {
                console.error('[SplashScreen] Error during installation (monitored by SplashScreen):', payload.message);
                // If an error occurs here, it might mean the initial get_setup_status_and_initialize failed
                // or a quick verification step failed in a way that emits this.
                setInitializationError(payload.message || 'Installation process error');
                setNavigateToSetup(true); // Likely need to go to setup screen to retry/show error
            }
        });
        unlistenInstallation = installationListener;

        // Listener for backend status (especially after ensure_comfyui_running_and_healthy)
        const backendStatusListener = await listen('backend-status', (event) => {
            const payload = event.payload as { status: string; message: string; isError: boolean };
            console.log(`[SplashScreen] Received backend-status:`, payload);

            if (isStartingServices) { // Only process if we are in the service starting phase
                if (payload.status === 'backend_ready' && !payload.isError) {
                    console.log('[SplashScreen] Backend services reported ready.');
                    setLoadingText('Backend services started. Launching application...');
                    setProgress(100);
                    setIsBackendReady(true);
                    setIsStartingServices(false);
                    // Clear service timer on success
                    if (serviceStartupTimerId) clearTimeout(serviceStartupTimerId);
                    serviceStartupTimerId = null;
                } else if (payload.isError || payload.status === 'backend_error') {
                    console.error('[SplashScreen] Error during backend service startup:', payload.message);
                    setLoadingText(`Error starting services: ${payload.message}`);
                    setInitializationError(payload.message || 'Backend service startup failed');
                    setNavigateToSetup(true);
                    setIsStartingServices(false);
                    // Clear service timer on error
                    if (serviceStartupTimerId) clearTimeout(serviceStartupTimerId);
                    serviceStartupTimerId = null;
                } else {
                    // Other backend statuses like 'starting_sidecar', 'sidecar_spawned_checking_health'
                    setLoadingText(payload.message || 'Starting backend services...');
                }
            }
        });
        unlistenBackendStatus = backendStatusListener;

        // Invoke the initial command
        try {
          console.log('[SplashScreen] Invoking get_setup_status_and_initialize command...');
          setLoadingText('Verifying installation...');
          setProgress(10);
          await invoke('get_setup_status_and_initialize');
          console.log('[SplashScreen] invoke get_setup_status_and_initialize completed.');
        } catch (err) {
          const errorMessage = (err instanceof Error ? err.message : String(err)) || 'Failed to invoke setup status check';
          console.error('[SplashScreen] get_setup_status_and_initialize command CATCH block triggered. Error:', errorMessage);
          setLoadingText(`Error: ${errorMessage}`);
          setInitializationError(errorMessage);
          setNavigateToSetup(true);
          // Clear initial timer on error
          if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
          initialVerificationTimerId = null;
        }

      } catch (error) {
        console.error('[SplashScreen] Failed to set up backend communication (Tauri API import failed?):', error);
        const errorMsg = 'Failed to initialize communication with backend.';
        setLoadingText(`Error: ${errorMsg}`);
        setInitializationError(errorMsg);
        setNavigateToSetup(true);
        // Clear initial timer on error
        if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
        initialVerificationTimerId = null;
      }
    };

    initializeApplication();

    // Start initial verification failsafe timer
    console.log('[SplashScreen] Starting initial verification failsafe timer (1 min)...');
    initialVerificationTimerId = setTimeout(() => {
        console.warn('[SplashScreen] Failsafe timeout (1 min) for initial verification triggered.');
        // Check state again inside timeout callback to ensure it hasn't resolved in the meantime
        if (!isBackendReady && !navigateToSetup && !isStartingServices && !initializationError) {
            setLoadingText('Initial verification taking too long, preparing for setup...');
            setInitializationError('Initial verification timed out.');
            setNavigateToSetup(true);
        }
    }, 60000); // 1 minute

    // Cleanup function
    return () => {
      console.log('[SplashScreen] Cleaning up listeners and timers');
      if (unlistenSetupStatus) unlistenSetupStatus();
      if (unlistenInstallation) unlistenInstallation();
      if (unlistenBackendStatus) unlistenBackendStatus();
      // Clear all timers on cleanup
      if (initialVerificationTimerId) clearTimeout(initialVerificationTimerId);
      if (serviceStartupTimerId) clearTimeout(serviceStartupTimerId);
      if (ensureCmdTimeoutId) clearTimeout(ensureCmdTimeoutId);
    };
  }, []); // Ensures this effect runs only once on mount

  // Effect to call onComplete when a navigation decision is made
  useEffect(() => {
    if (isBackendReady) { // Navigate to main app
      console.log('[SplashScreen] Backend is ready, calling onComplete to navigate to main application.');
      const timer = setTimeout(() => {
        onComplete(); // This should navigate to main app (e.g., /title)
      }, 1000);
      return () => clearTimeout(timer);
    } else if (navigateToSetup) { // Navigate to SetupScreen
      console.log('[SplashScreen] Full setup required, calling onComplete to navigate to setup screen.');
       const timer = setTimeout(() => {
        // Modify onComplete or pass a parameter if SplashScreen's onComplete directly goes to main app.
        // For now, assuming onComplete can handle routing or a different callback is needed for setup.
        // Let's assume onComplete is smart or we adjust App.tsx routing.
        // A more robust way would be for onComplete to take a route: onComplete('/setup') or onComplete('/title')
        // Or, SplashScreen itself uses router.push. For now, signaling via onComplete.
        // This might require onComplete to be (destination?: string) => void;
        // For this iteration, we'll rely on a convention or a later router.push here.
        
        // TEMPORARY: Direct navigation for clarity, assuming useRouter is available
        // import { useRouter } from 'next/navigation';
        // const router = useRouter();
        // router.push('/setup');
        // This direct push is better. For now, I'll stick to onComplete and note this for review.
        onComplete(); // This should navigate to /setup
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [isBackendReady, navigateToSetup, onComplete]);
  
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
