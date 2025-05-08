/**
 * Tauri API integration helpers
 * 
 * This file provides functions to safely use Tauri APIs in a way that works
 * in both development and production environments, with enhanced diagnostics.
 */

// Logging function with timestamps
const log = (message: string) => {
  console.log(`[TauriHelper ${new Date().toISOString()}] ${message}`);
};

// Check if running in a Tauri environment
export const isTauriApp = (): boolean => {
  const tauriAvailable = typeof window !== 'undefined' && window.__TAURI__ !== undefined;
  log(`Tauri availability check: ${tauriAvailable ? 'YES' : 'NO'}`);
  return tauriAvailable;
};

// Detailed check of Tauri API components
export const getTauriDetails = (): Record<string, boolean> => {
  if (!isTauriApp()) {
    return {
      available: false,
      invoke: false,
      event: false,
      app: false,
      path: false,
    };
  }

  const details = {
    available: true,
    invoke: typeof window.__TAURI__?.invoke === 'function',
    event: typeof window.__TAURI__?.event === 'object',
    app: typeof window.__TAURI__?.app === 'object',
    path: typeof window.__TAURI__?.path === 'object',
  };

  log(`Tauri API details: ${JSON.stringify(details)}`);
  return details;
};

// Safely invoke a Tauri command with enhanced error handling
export const invokeTauri = async <T>(command: string, args?: any): Promise<T | null> => {
  if (!isTauriApp()) {
    log(`Cannot invoke command "${command}": Tauri environment not detected`);
    return null;
  }

  if (typeof window.__TAURI__?.invoke !== 'function') {
    log(`Cannot invoke command "${command}": Tauri invoke function not available`);
    return null;
  }

  try {
    log(`Invoking Tauri command: ${command}`);
    const result = await window.__TAURI__.invoke(command, args);
    log(`Command ${command} succeeded`);
    return result as T;
  } catch (error) {
    log(`Error invoking Tauri command ${command}: ${error instanceof Error ? error.message : String(error)}`);
    return null;
  }
};

// Get Tauri app information with better error handling
export const getTauriInfo = async () => {
  if (!isTauriApp()) {
    return {
      available: false,
      version: 'N/A',
      appName: 'N/A',
    };
  }

  try {
    // Check if app API is available
    if (typeof window.__TAURI__?.app?.getVersion !== 'function' || 
        typeof window.__TAURI__?.app?.getName !== 'function') {
      log('Tauri app API functions not available');
      return {
        available: true,
        version: 'API Not Available',
        appName: 'API Not Available',
      };
    }

    const version = await window.__TAURI__.app.getVersion();
    const appName = await window.__TAURI__.app.getName();
    
    log(`Tauri app info: ${appName} ${version}`);
    return {
      available: true,
      version,
      appName,
    };
  } catch (error) {
    log(`Error getting Tauri app info: ${error instanceof Error ? error.message : String(error)}`);
    return {
      available: true,
      version: 'Error',
      appName: 'Error',
    };
  }
};

// Wait for Tauri to be available with timeout
export const waitForTauri = (timeoutMs = 5000): Promise<boolean> => {
  log(`Waiting for Tauri to be available (timeout: ${timeoutMs}ms)`);
  
  return new Promise((resolve) => {
    // Check immediately
    if (isTauriApp()) {
      log('Tauri already available');
      resolve(true);
      return;
    }
    
    // Set a timeout
    const timeoutId = setTimeout(() => {
      log('Timed out waiting for Tauri');
      resolve(false);
    }, timeoutMs);
    
    // Check periodically
    const checkInterval = 100; // ms
    const intervalId = setInterval(() => {
      if (isTauriApp()) {
        clearTimeout(timeoutId);
        clearInterval(intervalId);
        log('Tauri became available');
        resolve(true);
      }
    }, checkInterval);
  });
};

// Initialize Tauri event listeners
export const initTauriListeners = () => {
  if (!isTauriApp()) {
    log('Cannot initialize event listeners: Tauri environment not detected');
    return;
  }

  try {
    // Check if event API is available
    if (typeof window.__TAURI__?.event?.listen !== 'function') {
      log('Tauri event API not available');
      return;
    }
    
    log('Initializing Tauri event listeners');
    
    // Add any global event listeners here
    
  } catch (error) {
    log(`Error initializing Tauri event listeners: ${error instanceof Error ? error.message : String(error)}`);
  }
};

// Register a Tauri ready callback
let isReady = false;
const readyCallbacks: Array<() => void> = [];

export const onTauriReady = (callback: () => void) => {
  if (isReady) {
    // If Tauri is already ready, call the callback immediately
    callback();
  } else {
    // Otherwise, queue the callback
    readyCallbacks.push(callback);
  }
};

// Initialize Tauri when this module is imported
if (typeof window !== 'undefined') {
  // Only run in browser environment
  log('TauriHelper module initialized in browser environment');
  
  // Check on DOMContentLoaded
  window.addEventListener('DOMContentLoaded', () => {
    log('DOM loaded, checking for Tauri availability');
    
    // Start waiting for Tauri with a longer timeout
    waitForTauri(10000).then((available) => {
      if (available) {
        log('Tauri is ready');
        isReady = true;
        
        // Initialize listeners
        initTauriListeners();
        
        // Execute all queued callbacks
        readyCallbacks.forEach(callback => callback());
        
        // Clear the queue
        readyCallbacks.length = 0;
      } else {
        log('Tauri did not become available within timeout');
      }
    });
  });
}

// Add TypeScript types for Tauri
declare global {
  interface Window {
    __TAURI__?: {
      invoke: <T>(command: string, args?: any) => Promise<T>;
      event: {
        listen: (event: string, callback: (data: any) => void) => Promise<() => void>;
        emit: (event: string, data?: any) => Promise<void>;
      };
      app: {
        getVersion: () => Promise<string>;
        getName: () => Promise<string>;
      };
      path: {
        appDir: () => Promise<string>;
        appLocalDataDir: () => Promise<string>;
      };
    };
  }
}