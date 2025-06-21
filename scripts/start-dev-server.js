const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const appDataDir = process.env.LOCALAPPDATA || process.env.APPDATA;
if (!appDataDir) {
  console.error('Error: Could not determine application data directory.');
  process.exit(1);
}
const appIdentifier = 'com.metamorphosis.app'; // From tauri.conf.json
const pidFilePath = path.join(appDataDir, appIdentifier, 'dev-server.pid');

console.log(`[START_DEV_SERVER] PID file path: ${pidFilePath}`);

const child = spawn('npm', ['run', 'dev'], {
  cwd: path.join(__dirname, '..'), // Change to metamorphosis-app directory
  stdio: 'inherit',
  shell: true
});

fs.writeFileSync(pidFilePath, child.pid.toString());

child.on('exit', (code, signal) => {
  console.log(`Development server exited with code ${code} and signal ${signal}. PID file not removed by start-dev-server.js.`);
});

child.on('error', (err) => {
  console.error('Failed to start development server:', err);
});