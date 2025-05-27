"use client";

import React, { useEffect, useState } from "react";
import "./globals.css";
import { Quicksand } from 'next/font/google';

const quicksand = Quicksand({
  subsets: ['latin'],
  display: 'swap',
  weight: ['300', '400', '500', '600', '700'], // Adjust weights as needed
  variable: '--font-quicksand',
});

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const [tauriInitialized, setTauriInitialized] = useState(false);
  
  useEffect(() => {
    // Small utility to log with timestamps
    const log = (message: string) => {
      console.log(`[Layout ${new Date().toISOString()}] ${message}`);
    };
    
    log('Root layout mounted');
    
    // Check for Tauri availability
    if (typeof window !== 'undefined') {
      log('Window is defined, checking for Tauri...');
      
      if (window.__TAURI__) {
        log('Tauri is available on initial render');
        setTauriInitialized(true);
      } else {
        log('Tauri not initially available, waiting...');
        
        // Try to dynamically import the Tauri helper
        const checkTauri = () => {
          if (window.__TAURI__) {
            log('Tauri became available');
            setTauriInitialized(true);
            return true;
          }
          return false;
        };
        
        // Check immediately in case it appears between render and effect
        if (checkTauri()) return;
        
        // Set periodic check
        const intervalId = setInterval(() => {
          if (checkTauri()) {
            clearInterval(intervalId);
          }
        }, 500);
        
        // Cleanup
        return () => clearInterval(intervalId);
      }
    } else {
      log('Window is not defined (server-side rendering)');
    }
  }, []);
  
  return (
    // Remove quicksand.variable from html, will apply quicksand.className to body
    <html lang="en">
      {/* Ensure no whitespace before <head> */}
      <head>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="description" content="Metamorphosis character creation platform" />
        <title>Metamorphosis</title>
      </head>
      {/* Ensure no whitespace before <body> */}
      {/* Apply quicksand.className directly to body, remove font-sans as className handles it */}
      <body className={quicksand.className}>
        {/* Data attribute for Tauri status tracking in CSS/devtools */}
        <div data-tauri-initialized={tauriInitialized}>
          {children}
        </div>
      </body>
    </html>
  );
}