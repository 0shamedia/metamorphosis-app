/**
 * TypeScript declarations for Tauri APIs
 * 
 * This file contains the global type definitions for Tauri APIs
 * to ensure consistent typing across the application.
 */

// Extend the Window interface to include Tauri APIs
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