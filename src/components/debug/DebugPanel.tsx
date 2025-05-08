'use client';

import { useState, useEffect } from 'react';
import { getLogs, clearLogs, exportLogs, LogLevel } from '../../utils/logger';

/**
 * Debug Panel Component
 * 
 * A hidden panel that can be toggled to view application logs and diagnostics
 * Toggle with Ctrl+Shift+D keyboard shortcut
 */
export default function DebugPanel() {
  const [isVisible, setIsVisible] = useState(false);
  const [logs, setLogs] = useState<any[]>([]);
  const [filter, setFilter] = useState<LogLevel | 'all'>('all');
  const [tauriInfo, setTauriInfo] = useState<any>(null);
  const [refreshInterval, setRefreshInterval] = useState<number | null>(null);

  // Setup keyboard shortcut to toggle visibility (Ctrl+Shift+D)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === 'D') {
        e.preventDefault();
        setIsVisible(prev => !prev);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Fetch logs when panel becomes visible
  useEffect(() => {
    if (isVisible) {
      refreshLogs();
      
      // Set up automatic refresh
      const interval = window.setInterval(refreshLogs, 2000);
      setRefreshInterval(interval);
      
      // Check for Tauri environment
      checkTauriEnvironment();
    } else if (refreshInterval) {
      // Clear refresh interval when hiding panel
      window.clearInterval(refreshInterval);
      setRefreshInterval(null);
    }
  }, [isVisible]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (refreshInterval) {
        window.clearInterval(refreshInterval);
      }
    };
  }, [refreshInterval]);

  // Refresh logs from localStorage
  const refreshLogs = () => {
    try {
      const storedLogs = getLogs();
      setLogs(storedLogs);
    } catch (e) {
      console.error('Error loading logs:', e);
    }
  };

  // Check if we're running in Tauri environment
  const checkTauriEnvironment = async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        setTauriInfo({
          available: true,
          version: await window.__TAURI__.app.getVersion(),
          appName: await window.__TAURI__.app.getName(),
          os: await getOsInfo()
        });
      } else {
        setTauriInfo({ available: false });
      }
    } catch (e) {
      console.error('Error checking Tauri environment:', e);
      setTauriInfo({ available: false, error: String(e) });
    }
  };

  // Get OS information if in Tauri
  const getOsInfo = async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const os = await window.__TAURI__.os.type();
        const platform = await window.__TAURI__.os.platform();
        const arch = await window.__TAURI__.os.arch();
        return { os, platform, arch };
      }
      return 'Unknown';
    } catch (e) {
      return `Error: ${e}`;
    }
  };

  // Handle exporting logs
  const handleExport = () => {
    exportLogs();
  };

  // Handle clearing logs
  const handleClear = () => {
    clearLogs();
    setLogs([]);
  };

  // Filter logs by level
  const filteredLogs = filter === 'all' 
    ? logs 
    : logs.filter(log => log.level === filter);

  // Render nothing if not visible
  if (!isVisible) return null;

  return (
    <div 
      className="fixed bottom-0 left-0 right-0 bg-gray-900 text-white z-50 h-1/2 overflow-auto"
      style={{ 
        opacity: 0.95, 
        fontFamily: 'monospace', 
        fontSize: '12px'
      }}
    >
      <div className="flex justify-between items-center p-2 bg-gray-800 sticky top-0">
        <div className="font-bold">Metamorphosis Debug Panel</div>
        
        <div className="flex space-x-2">
          <select 
            className="text-xs bg-gray-700 border border-gray-600 rounded px-2 py-1"
            value={filter}
            onChange={(e) => setFilter(e.target.value as LogLevel | 'all')}
          >
            <option value="all">All Levels</option>
            <option value="info">Info</option>
            <option value="warn">Warnings</option>
            <option value="error">Errors</option>
            <option value="debug">Debug</option>
          </select>
          
          <button 
            className="text-xs bg-blue-600 hover:bg-blue-700 px-2 py-1 rounded"
            onClick={refreshLogs}
          >
            Refresh
          </button>
          
          <button 
            className="text-xs bg-green-600 hover:bg-green-700 px-2 py-1 rounded"
            onClick={handleExport}
          >
            Export
          </button>
          
          <button 
            className="text-xs bg-red-600 hover:bg-red-700 px-2 py-1 rounded"
            onClick={handleClear}
          >
            Clear
          </button>
          
          <button 
            className="text-xs bg-gray-600 hover:bg-gray-700 px-2 py-1 rounded"
            onClick={() => setIsVisible(false)}
          >
            Close
          </button>
        </div>
      </div>
      
      <div className="flex">
        {/* Left panel - system info */}
        <div className="w-1/4 p-2 border-r border-gray-700 overflow-auto">
          <h3 className="font-bold mb-2">System Info</h3>
          
          <div className="mb-3">
            <div className="text-xs text-gray-400">Tauri Environment</div>
            <pre className="text-xs whitespace-pre-wrap">
              {tauriInfo ? JSON.stringify(tauriInfo, null, 2) : 'Loading...'}
            </pre>
          </div>
          
          <div className="mb-3">
            <div className="text-xs text-gray-400">Window</div>
            <pre className="text-xs">
              Width: {typeof window !== 'undefined' ? window.innerWidth : 'N/A'}<br />
              Height: {typeof window !== 'undefined' ? window.innerHeight : 'N/A'}
            </pre>
          </div>
          
          <div className="mb-3">
            <div className="text-xs text-gray-400">User Agent</div>
            <pre className="text-xs whitespace-pre-wrap">
              {typeof navigator !== 'undefined' ? navigator.userAgent : 'N/A'}
            </pre>
          </div>
          
          <div className="mb-3">
            <div className="text-xs text-gray-400">Log Summary</div>
            <pre className="text-xs">
              Total: {logs.length}<br />
              Info: {logs.filter(l => l.level === 'info').length}<br />
              Warn: {logs.filter(l => l.level === 'warn').length}<br />
              Error: {logs.filter(l => l.level === 'error').length}<br />
              Debug: {logs.filter(l => l.level === 'debug').length}
            </pre>
          </div>
        </div>
        
        {/* Right panel - logs */}
        <div className="w-3/4 overflow-auto">
          <table className="w-full text-xs">
            <thead className="bg-gray-800 sticky top-0">
              <tr>
                <th className="p-1 text-left">Time</th>
                <th className="p-1 text-left">Level</th>
                <th className="p-1 text-left">Component</th>
                <th className="p-1 text-left">Message</th>
              </tr>
            </thead>
            <tbody>
              {filteredLogs.map((log, index) => (
                <tr 
                  key={index} 
                  className={`border-b border-gray-800 ${
                    log.level === 'error' ? 'bg-red-900/30' :
                    log.level === 'warn' ? 'bg-yellow-900/30' :
                    index % 2 === 0 ? 'bg-gray-800/30' : ''
                  }`}
                >
                  <td className="p-1 whitespace-nowrap">
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </td>
                  <td className="p-1 whitespace-nowrap">
                    <span className={`
                      ${log.level === 'error' ? 'text-red-400' : 
                        log.level === 'warn' ? 'text-yellow-400' :
                        log.level === 'debug' ? 'text-blue-400' :
                        'text-green-400'
                      }
                    `}>
                      {log.level.toUpperCase()}
                    </span>
                  </td>
                  <td className="p-1 whitespace-nowrap">{log.component}</td>
                  <td className="p-1">
                    <div>{log.message}</div>
                    {log.data && (
                      <pre className="text-gray-400 text-xs mt-1 whitespace-pre-wrap">
                        {typeof log.data === 'object' 
                          ? JSON.stringify(log.data, null, 2)
                          : String(log.data)
                        }
                      </pre>
                    )}
                  </td>
                </tr>
              ))}
              
              {filteredLogs.length === 0 && (
                <tr>
                  <td colSpan={4} className="p-4 text-center text-gray-500">
                    No logs available
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

// TypeScript declaration for Tauri globals
declare global {
  interface Window {
    __TAURI__?: {
      app: {
        getVersion: () => Promise<string>;
        getName: () => Promise<string>;
      };
      os: {
        type: () => Promise<string>;
        platform: () => Promise<string>;
        arch: () => Promise<string>;
      };
    };
  }
}
