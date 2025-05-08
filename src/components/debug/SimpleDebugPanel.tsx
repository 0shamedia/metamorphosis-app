'use client';

import { useState, useEffect } from 'react';

/**
 * A very simple debug panel for testing keyboard shortcuts
 */
export default function SimpleDebugPanel() {
  const [isVisible, setIsVisible] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);

  // Setup keyboard shortcut to toggle visibility (Ctrl+Shift+D)
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.ctrlKey && e.shiftKey && e.key === 'd' || e.key === 'D') {
        e.preventDefault();
        console.log('[SimpleDebugPanel] Toggle shortcut triggered');
        setIsVisible(prev => !prev);
      }
    }

    console.log('[SimpleDebugPanel] Adding keydown event listener');
    window.addEventListener('keydown', handleKeyDown);
    
    return () => {
      console.log('[SimpleDebugPanel] Removing keydown event listener');
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // Add a log entry every second when visible
  useEffect(() => {
    if (!isVisible) return;
    
    console.log('[SimpleDebugPanel] Panel is now visible');
    
    const timer = setInterval(() => {
      const timestamp = new Date().toISOString();
      const message = `Log entry at ${timestamp}`;
      setLogs(prev => [...prev.slice(-19), message]);
    }, 1000);
    
    return () => {
      console.log('[SimpleDebugPanel] Panel is now hidden');
      clearInterval(timer);
    };
  }, [isVisible]);

  // Render nothing if not visible
  if (!isVisible) return null;

  return (
    <div 
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        zIndex: 9999,
        backgroundColor: 'rgba(0, 0, 0, 0.8)',
        color: 'white',
        padding: '10px',
        fontFamily: 'monospace',
        fontSize: '12px',
        maxHeight: '50vh',
        overflow: 'auto',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '10px' }}>
        <h2 style={{ margin: 0, fontSize: '14px' }}>Simple Debug Panel</h2>
        <button 
          onClick={() => setIsVisible(false)}
          style={{
            backgroundColor: '#ff4444',
            border: 'none',
            color: 'white',
            padding: '4px 8px',
            borderRadius: '4px',
            cursor: 'pointer',
            fontSize: '12px',
          }}
        >
          Close
        </button>
      </div>
      
      <div>
        <p>Press Ctrl+Shift+D to toggle this panel</p>
        <p>Browser info: {navigator.userAgent}</p>
        <p>Window size: {window.innerWidth}x{window.innerHeight}</p>
        <p>Timestamp: {new Date().toISOString()}</p>
      </div>
      
      <div style={{ marginTop: '10px' }}>
        <h3 style={{ margin: '0 0 5px 0', fontSize: '13px' }}>Live Logs:</h3>
        <div style={{ 
          backgroundColor: 'rgba(0, 0, 0, 0.3)', 
          padding: '5px', 
          borderRadius: '4px',
          maxHeight: '200px',
          overflow: 'auto'
        }}>
          {logs.map((log, i) => (
            <div key={i} style={{ padding: '2px 0' }}>{log}</div>
          ))}
        </div>
      </div>
    </div>
  );
}