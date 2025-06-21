const fs = require('fs');
const path = require('path');
let kill;
if (process.platform !== 'win32') {
  kill = require('tree-kill'); // For non-Windows systems
}
const { exec } = require('child_process'); // For Windows taskkill

const appDataDir = process.env.LOCALAPPDATA || process.env.APPDATA;
if (!appDataDir) {
  console.error('Error: Could not determine application data directory.');
  process.exit(1);
}
const appIdentifier = 'com.metamorphosis.app'; // From tauri.conf.json
const pidFilePath = path.join(appDataDir, appIdentifier, 'dev-server.pid');

console.log(`[KILL_DEV_SERVER] PID file path: ${pidFilePath}`);

if (fs.existsSync(pidFilePath)) {
  const pid = parseInt(fs.readFileSync(pidFilePath, 'utf-8'));
  console.log(`Attempting to kill process with PID: ${pid}`);

if (process.platform === 'win32') {
    const PORT = 3000; // Default Next.js dev server port
    console.log(`Searching for process listening on port ${PORT}...`);

    exec(`netstat -ano | findstr :${PORT}`, (err, stdout, stderr) => {
        if (err) {
            console.error(`Failed to run netstat:`, err);
            console.error(`Stdout: ${stdout}`);
            console.error(`Stderr: ${stderr}`);
            if (fs.existsSync(pidFilePath)) {
                fs.unlinkSync(pidFilePath);
                console.log(`PID file removed: ${pidFilePath}`);
            }
            return;
        }

        const lines = stdout.split('\n');
        let foundProcess = false;

        for (const line of lines) {
            const parts = line.trim().split(/\s+/);
            if (parts.length >= 5 && parts[1].endsWith(`:${PORT}`)) {
                const pid = parseInt(parts[4]);
                if (!isNaN(pid)) {
                    console.log(`Found process with PID ${pid} listening on port ${PORT}.`);
                    foundProcess = true;

                    // Attempt to kill the identified process
                    exec(`taskkill /F /PID ${pid}`, (killErr, killStdout, killStderr) => {
                        if (killErr) {
                            console.error(`Failed to kill process ${pid}:`, killErr);
                            console.error(`Kill Stdout: ${killStdout}`);
                            console.error(`Kill Stderr: ${killStderr}`);
                        } else {
                            console.log(`Process ${pid} killed successfully.`);
                        }
                        // Remove the PID file after attempting to kill the process
                        if (fs.existsSync(pidFilePath)) {
                            fs.unlinkSync(pidFilePath);
                            console.log(`PID file removed: ${pidFilePath}`);
                        }
                    });
                    break; // Assuming only one process listens on this port for Next.js dev server
                }
            }
        }

        if (!foundProcess) {
            console.log(`No process found listening on port ${PORT}.`);
            // If no process was found, still remove the PID file if it exists
            if (fs.existsSync(pidFilePath)) {
                fs.unlinkSync(pidFilePath);
                console.log(`PID file removed: ${pidFilePath}`);
            }
        }
    });
} else {
    // For non-Windows systems, still use tree-kill with the PID from the file
    // This assumes tree-kill is more reliable on Unix-like systems for child processes
    kill(pid, 'SIGTERM', (err) => {
        if (err) {
            console.error(`Failed to kill process ${pid}:`, err);
        } else {
            console.log(`Process ${pid} and its children killed.`);
            fs.unlinkSync(pidFilePath);
        }
    });
}
} else {
  console.log('No dev-server.pid file found. No process to kill.');
}