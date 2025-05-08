/**
 * Safely load Tauri imports
 * 
 * This file manages dynamic imports of Tauri modules to ensure they're only
 * loaded when the application is running in a Tauri environment.
 */

// Dynamic import helpers
const safeImport = async (importFn: () => Promise<any>, fallback: any = null) => {
  try {
    return await importFn();
  } catch (error) {
    console.error('Failed to import Tauri module:', error);
    return fallback;
  }
};

// Check if we're in a Tauri environment
const isTauriEnvironment = typeof window !== 'undefined' && window.__TAURI__ !== undefined;

// Dynamically import Tauri modules only when in a Tauri environment
export const loadTauriModules = async () => {
  if (!isTauriEnvironment) {
    console.warn('Not running in Tauri environment, Tauri APIs will not be available');
    return {
      available: false,
      fs: null,
      http: null,
      // Add other modules as needed
    };
  }
  
  // Dynamically import the Tauri modules
  try {
    console.log('Loading Tauri modules');
    
    // We'll use Promise.allSettled to attempt to load all modules
    // even if some fail
    const [fsResult, httpResult] = await Promise.allSettled([
      safeImport(() => import('@tauri-apps/plugin-fs')),
      safeImport(() => import('@tauri-apps/plugin-http')),
      // Add other modules as needed
    ]);
    
    return {
      available: true,
      fs: fsResult.status === 'fulfilled' ? fsResult.value : null,
      http: httpResult.status === 'fulfilled' ? httpResult.value : null,
      // Add other modules as needed
    };
  } catch (error) {
    console.error('Error loading Tauri modules:', error);
    return {
      available: false,
      fs: null,
      http: null,
      // Add other modules as needed
    };
  }
};

// Type definitions for runtime checking
declare global {
  interface Window {
    __TAURI__?: any;
  }
}