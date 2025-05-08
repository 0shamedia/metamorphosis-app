/**
 * Enhanced logger for Metamorphosis UI debugging
 * 
 * This utility enhances console logging with timestamps and component information
 * while also saving logs to localStorage for later retrieval.
 */

// Maximum number of log entries to keep in localStorage
const MAX_LOG_ENTRIES = 1000;

// Storage key for logs
const LOG_STORAGE_KEY = 'metamorphosis_debug_logs';

// Log levels
export type LogLevel = 'info' | 'warn' | 'error' | 'debug';

// Log entry interface
interface LogEntry {
  timestamp: string;
  level: LogLevel;
  component: string;
  message: string;
  data?: any;
}

/**
 * Retrieves all stored logs
 */
export function getLogs(): LogEntry[] {
  if (typeof window === 'undefined') return [];
  
  try {
    const storedLogs = localStorage.getItem(LOG_STORAGE_KEY);
    return storedLogs ? JSON.parse(storedLogs) : [];
  } catch (e) {
    console.error('Error retrieving logs from localStorage:', e);
    return [];
  }
}

/**
 * Clears all stored logs
 */
export function clearLogs(): void {
  if (typeof window === 'undefined') return;
  
  try {
    localStorage.removeItem(LOG_STORAGE_KEY);
  } catch (e) {
    console.error('Error clearing logs from localStorage:', e);
  }
}

/**
 * Exports logs to a downloadable text file
 */
export function exportLogs(): void {
  if (typeof window === 'undefined') return;
  
  try {
    const logs = getLogs();
    const content = logs.map(log => 
      `[${log.timestamp}] [${log.level.toUpperCase()}] [${log.component}] ${log.message}${
        log.data ? '\n' + JSON.stringify(log.data, null, 2) : ''
      }`
    ).join('\n');
    
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    
    const now = new Date();
    const filename = `metamorphosis_logs_${now.toISOString().replace(/[:.]/g, '-')}.txt`;
    
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    link.click();
    
    setTimeout(() => URL.revokeObjectURL(url), 100);
  } catch (e) {
    console.error('Error exporting logs:', e);
  }
}

/**
 * Stores a log entry in localStorage
 */
function storeLog(entry: LogEntry): void {
  if (typeof window === 'undefined') return;
  
  try {
    const logs = getLogs();
    logs.push(entry);
    
    // Keep only the most recent logs
    while (logs.length > MAX_LOG_ENTRIES) {
      logs.shift();
    }
    
    localStorage.setItem(LOG_STORAGE_KEY, JSON.stringify(logs));
  } catch (e) {
    console.error('Error storing log entry:', e);
  }
}

/**
 * Creates a formatted log message
 */
function createLogEntry(level: LogLevel, component: string, message: string, data?: any): LogEntry {
  return {
    timestamp: new Date().toISOString(),
    level,
    component,
    message,
    data
  };
}

/**
 * Creates a logger for a specific component
 */
export function createLogger(component: string) {
  return {
    info: (message: string, data?: any) => {
      const entry = createLogEntry('info', component, message, data);
      console.info(`[${entry.timestamp}] [${component}] ${message}`, data || '');
      storeLog(entry);
    },
    
    warn: (message: string, data?: any) => {
      const entry = createLogEntry('warn', component, message, data);
      console.warn(`[${entry.timestamp}] [${component}] ${message}`, data || '');
      storeLog(entry);
    },
    
    error: (message: string, data?: any) => {
      const entry = createLogEntry('error', component, message, data);
      console.error(`[${entry.timestamp}] [${component}] ${message}`, data || '');
      storeLog(entry);
    },
    
    debug: (message: string, data?: any) => {
      const entry = createLogEntry('debug', component, message, data);
      console.debug(`[${entry.timestamp}] [${component}] ${message}`, data || '');
      storeLog(entry);
    },
    
    // Helper for timing operations
    timing: (operation: string, func: () => any) => {
      const start = performance.now();
      try {
        const result = func();
        const duration = performance.now() - start;
        console.info(`[${component}] ${operation} completed in ${duration.toFixed(2)}ms`);
        return result;
      } catch (e) {
        const duration = performance.now() - start;
        console.error(`[${component}] ${operation} failed after ${duration.toFixed(2)}ms:`, e);
        throw e;
      }
    }
  };
}

// Export a default instance for general logging
export const logger = createLogger('App');

// Add utility to capture uncaught errors
export function setupGlobalErrorHandling(): void {
  if (typeof window === 'undefined') return;
  
  window.addEventListener('error', (event) => {
    logger.error(`Uncaught error: ${event.message}`, {
      error: event.error?.toString(),
      stack: event.error?.stack,
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno
    });
  });
  
  window.addEventListener('unhandledrejection', (event) => {
    logger.error(`Unhandled promise rejection: ${event.reason}`, {
      reason: event.reason?.toString(),
      stack: event.reason?.stack
    });
  });
  
  logger.info('Global error handlers installed');
}
