import React, { useState } from 'react';
import { PhysicalSize, PhysicalPosition, Window, primaryMonitor } from '@tauri-apps/api/window'; // Import primaryMonitor and PhysicalPosition

interface VisualSettingsModalProps {
  show: boolean;
  onClose: () => void;
  // onApplySettings: (resolution: { width: number; height: number }, displayMode: string) => void;
}

const VisualSettingsModal: React.FC<VisualSettingsModalProps> = ({ show, onClose }) => {
  const [selectedResolution, setSelectedResolution] = useState({ width: 1280, height: 720 }); // Default to 1280x720
  const [selectedDisplayMode, setSelectedDisplayMode] = useState('windowed'); // Default to windowed

  const handleResolutionChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const [width, height] = event.target.value.split('x').map(Number);
    setSelectedResolution({ width, height });
  };

  const handleDisplayModeChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedDisplayMode(event.target.value);
  };

  const handleApplySettings = async () => {
    console.log(`Applying settings: Resolution=${selectedResolution.width}x${selectedResolution.height}, Display Mode=${selectedDisplayMode}`);
    
    try {
      const currentWindow = Window.getCurrent();
      console.log(`Current window decorations before applying: ${await currentWindow.isDecorated()}`);
      console.log(`Current window fullscreen state before applying: ${await currentWindow.isFullscreen()}`);

      // Set display mode and size
      switch (selectedDisplayMode) {
        case 'windowed':
          console.log('Setting display mode to windowed...');
          await currentWindow.setFullscreen(false);
          await currentWindow.setDecorations(true); // Show decorations for windowed
          console.log(`Applying windowed size: ${selectedResolution.width}x${selectedResolution.height}`);
          await currentWindow.setSize(new PhysicalSize(selectedResolution.width, selectedResolution.height));
          break;
        case 'borderless':
          console.log('Setting display mode to borderless windowed...');
          await currentWindow.setFullscreen(false);
          await currentWindow.setDecorations(false); // Hide decorations for borderless
          const monitor = await primaryMonitor();
          if (monitor) {
            console.log(`Primary monitor details: Name=${monitor.name}, Size=${monitor.size.width}x${monitor.size.height}, Position=${monitor.position.x},${monitor.position.y}, ScaleFactor=${monitor.scaleFactor}`);
            // Use monitor.size and monitor.position for borderless windowed mode on desktop
            await currentWindow.setPosition(new PhysicalPosition(monitor.position.x, monitor.position.y));
            await currentWindow.setSize(new PhysicalSize(monitor.size.width, monitor.size.height));
            console.log(`Applied borderless size: ${monitor.size.width}x${monitor.size.height} at position ${monitor.position.x},${monitor.position.y}`);
          } else {
            console.warn('Primary monitor not found, cannot set borderless window size. Falling back to selected resolution.');
            await currentWindow.setSize(new PhysicalSize(selectedResolution.width, selectedResolution.height));
            console.log(`Applied fallback borderless size: ${selectedResolution.width}x${selectedResolution.height}`);
          }
          break;
        case 'fullscreen':
          console.log('Setting display mode to fullscreen...');
          await currentWindow.setDecorations(false); // Hide decorations in fullscreen
          await currentWindow.setFullscreen(true);
          console.log('Fullscreen mode applied.');
          break;
      }

      console.log(`Window decorations after applying: ${await currentWindow.isDecorated()}`);
      console.log(`Window fullscreen state after applying: ${await currentWindow.isFullscreen()}`);
      console.log('Settings applied successfully. Closing modal.');

    } catch (error) {
      console.error('Failed to apply settings:', error);
      // TODO: Show an error message to the user
    }

    onClose(); // Close the settings modal after applying
  };

  if (!show) {
    return null;
  }

  return (
    <div className="absolute inset-0 bg-black/70 z-40 flex items-center justify-center">
      <div className="bg-gray-800 p-8 rounded-lg shadow-lg text-white">
        <h2 className="text-2xl font-bold mb-4">Visual Settings</h2>

        {/* Resolution Selection */}
        <div className="mb-4">
          <label htmlFor="resolution" className="block text-sm font-medium text-gray-300">Resolution</label>
          <select
            id="resolution"
            className="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-600 focus:outline-none focus:ring-purple-500 focus:border-purple-500 sm:text-sm rounded-md bg-gray-700 text-white"
            value={`${selectedResolution.width}x${selectedResolution.height}`}
            onChange={handleResolutionChange}
          >
            <option value="1280x720">1280x720 (720p)</option>
            <option value="1920x1080">1920x1080 (1080p)</option>
            <option value="2560x1440">2560x1440 (1440p)</option>
            {/* Add more resolutions as needed */}
          </select>
        </div>

        {/* Display Mode Selection */}
        <div className="mb-4">
          <label htmlFor="displayMode" className="block text-sm font-medium text-gray-300">Display Mode</label>
          <select
            id="displayMode"
            className="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-600 focus:outline-none focus:ring-purple-500 focus:border-purple-500 sm:text-sm rounded-md bg-gray-700 text-white"
            value={selectedDisplayMode}
            onChange={handleDisplayModeChange}
          >
            <option value="windowed">Windowed</option>
            <option value="borderless">Borderless Windowed</option>
            <option value="fullscreen">Fullscreen</option>
          </select>
        </div>

        <button
          className="mt-6 px-4 py-2 bg-purple-600 rounded hover:bg-purple-700"
          onClick={handleApplySettings}
        >
          Apply Settings
        </button>
        <button
          className="mt-6 ml-4 px-4 py-2 bg-gray-600 rounded hover:bg-gray-700"
          onClick={onClose}
        >
          Cancel
        </button>
      </div>
    </div>
  );
};

export default VisualSettingsModal;