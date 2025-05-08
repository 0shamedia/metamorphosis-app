// Script to ensure proper Tauri integration with Next.js
const fs = require('fs');
const path = require('path');

console.log('Setting up Tauri integration with Next.js...');

// 1. Modify next.config.js for optimal Tauri compatibility
const nextConfigPath = path.join(__dirname, 'next.config.js');
console.log(`Checking Next.js configuration at: ${nextConfigPath}`);

try {
  let nextConfigContent = `/** @type {import('next').NextConfig} */
const nextConfig = {
  // Export as static site for Tauri
  output: 'export',
  
  // Disable image optimization for static export
  images: {
    unoptimized: true,
  },
  
  // Configure webpack for Tauri compatibility
  webpack: (config) => {
    // Make @tauri-apps modules external to avoid bundling issues
    config.externals = [
      ...config.externals || [],
      /^@tauri-apps\\/(?!api\\//).*/,
      ({ context, request }, callback) => {
        if (/^@tauri-apps\\/api($|\\/)/i.test(request)) {
          // We want to keep @tauri-apps/api in the bundle
          return callback();
        }
        // Externalize all other @tauri-apps/ modules
        if (/^@tauri-apps\\//i.test(request)) {
          return callback(null, 'commonjs ' + request);
        }
        callback();
      },
    ];
    
    return config;
  },
  
  // Disable React strict mode for development
  reactStrictMode: false,
  
  // Add environment variables
  env: {
    TAURI_ENABLED: true
  }
};

module.exports = nextConfig;`;

  fs.writeFileSync(nextConfigPath, nextConfigContent);
  console.log('Successfully updated Next.js configuration.');
} catch (err) {
  console.error('Error updating Next.js configuration:', err);
}

// 2. Create a helper file for Tauri API usage
const tauriHelperPath = path.join(__dirname, 'src', 'utils');
const tauriHelperFilePath = path.join(tauriHelperPath, 'tauri-helper.ts');

try {
  if (!fs.existsSync(tauriHelperPath)) {
    fs.mkdirSync(tauriHelperPath, { recursive: true });
  }

  let tauriHelperContent = `/**
 * Tauri API integration helpers
 * 
 * This file provides functions to safely use Tauri APIs in a way that works
 * in both development and production environments.
 */

// Check if running in a Tauri environment
export const isTauriApp = () => {
  return typeof window !== 'undefined' && window.__TAURI__ !== undefined;
};

// Safely invoke a Tauri command
export const invokeTauri = async <T>(command: string, args?: any): Promise<T | null> => {
  if (!isTauriApp()) {
    console.warn('Tauri environment not detected. Command not invoked:', command);
    return null;
  }

  try {
    const result = await window.__TAURI__.invoke(command, args);
    return result as T;
  } catch (error) {
    console.error(\`Error invoking Tauri command \${command}:\`, error);
    return null;
  }
};

// Get Tauri app information
export const getTauriInfo = async () => {
  if (!isTauriApp()) {
    return {
      available: false,
      version: 'N/A',
      appName: 'N/A',
    };
  }

  try {
    const version = await window.__TAURI__.app.getVersion();
    const appName = await window.__TAURI__.app.getName();
    
    return {
      available: true,
      version,
      appName,
    };
  } catch (error) {
    console.error('Error getting Tauri app info:', error);
    return {
      available: true,
      version: 'Error',
      appName: 'Error',
    };
  }
};

// Initialize Tauri event listeners
export const initTauriListeners = () => {
  if (!isTauriApp()) {
    console.warn('Tauri environment not detected. Event listeners not initialized.');
    return;
  }

  try {
    // Log that Tauri is available
    console.log('Tauri API is available. Initializing event listeners...');
    
    // Add any global event listeners here
    
  } catch (error) {
    console.error('Error initializing Tauri event listeners:', error);
  }
};

// Initialize Tauri when this module is imported
if (typeof window !== 'undefined') {
  // Only run in browser environment
  window.addEventListener('DOMContentLoaded', () => {
    console.log('DOM loaded, checking for Tauri availability...');
    if (isTauriApp()) {
      console.log('Tauri detected on DOMContentLoaded');
      initTauriListeners();
    } else {
      console.warn('Tauri not detected on DOMContentLoaded');
    }
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
`;

  fs.writeFileSync(tauriHelperFilePath, tauriHelperContent);
  console.log('Successfully created Tauri helper utility.');
} catch (err) {
  console.error('Error creating Tauri helper utility:', err);
}

// 3. Update page.tsx to use the Tauri helper
const pageFilePath = path.join(__dirname, 'src', 'app', 'page.tsx');

try {
  if (fs.existsSync(pageFilePath)) {
    console.log('Updating page.tsx to use Tauri helper...');
    
    // We'll add the import at the start of the file
    let pageContent = fs.readFileSync(pageFilePath, 'utf8');
    
    // Update imports to include Tauri helper
    if (!pageContent.includes('tauri-helper')) {
      pageContent = pageContent.replace(
        `'use client';`,
        `'use client';

import { isTauriApp, getTauriInfo } from '@/utils/tauri-helper';`
      );
      
      fs.writeFileSync(pageFilePath, pageContent);
      console.log('Successfully updated page.tsx with Tauri helper imports.');
    } else {
      console.log('page.tsx already includes Tauri helper imports.');
    }
  }
} catch (err) {
  console.error('Error updating page.tsx:', err);
}

console.log('Tauri integration setup complete!');
