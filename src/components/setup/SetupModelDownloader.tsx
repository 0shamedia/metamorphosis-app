import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  ModelDownloadProgressPayload,
  ModelDownloadCompletePayload,
  ModelDownloadFailedPayload,
  OverallModelDownloadProgressPayload,
} from '../../types/events';

interface ModelStatus {
  name: string;
  status: 'downloading' | 'completed' | 'failed';
  progress?: number; // Percentage 0-100
  downloadedBytes?: number;
  totalBytes?: number | null;
  error?: string;
  path?: string;
  durationSeconds?: number;
}

interface OverallProgress {
  completedModels: number;
  totalModels: number;
  progress: number; // Percentage 0-100
}

const SetupModelDownloader: React.FC = () => {
  const [modelStatuses, setModelStatuses] = useState<Record<string, ModelStatus>>({});
  const [overallProgress, setOverallProgress] = useState<OverallProgress | null>(null);

  useEffect(() => {
    const unlistenModelProgress = listen<ModelDownloadProgressPayload>(
      'model-download-progress',
      (event) => {
        const { modelName, downloadedBytes, totalBytes, progress } = event.payload;
        setModelStatuses((prev) => ({
          ...prev,
          [modelName]: {
            name: modelName,
            status: 'downloading',
            progress,
            downloadedBytes,
            totalBytes,
          },
        }));
      }
    );

    const unlistenModelComplete = listen<ModelDownloadCompletePayload>(
      'model-download-complete',
      (event) => {
        const { modelName, path, durationSeconds } = event.payload;
        setModelStatuses((prev) => ({
          ...prev,
          [modelName]: {
            ...prev[modelName],
            status: 'completed',
            path,
            durationSeconds,
            progress: 100, // Ensure progress is 100 on complete
          },
        }));
      }
    );

    const unlistenModelFailed = listen<ModelDownloadFailedPayload>(
      'model-download-failed',
      (event) => {
        const { modelName, error } = event.payload;
        setModelStatuses((prev) => ({
          ...prev,
          [modelName]: {
            ...prev[modelName],
            status: 'failed',
            error,
          },
        }));
      }
    );

    const unlistenOverallProgress = listen<OverallModelDownloadProgressPayload>(
      'overall-model-download-progress',
      (event) => {
        setOverallProgress(event.payload);
      }
    );

    return () => {
      unlistenModelProgress.then(f => f());
      unlistenModelComplete.then(f => f());
      unlistenModelFailed.then(f => f());
      unlistenOverallProgress.then(f => f());
    };
  }, []);

  return (
    <div className="model-downloader-setup">
      <h3>Downloading AI Models...</h3>
      {overallProgress !== null && ( // Check if overallProgress is not null
        <div className="overall-progress">
          <p>
            Overall Progress: {overallProgress.completedModels} / {overallProgress.totalModels} models completed
          </p>
          <progress value={overallProgress.progress} max="100" />
          <span>{overallProgress.progress?.toFixed(2)}%</span>
        </div>
      )}
      <ul className="model-list">
        {Object.values(modelStatuses).map((model) => (
          <li key={model.name} className={`model-item model-status-${model.status}`}>
            <h4>{model.name}</h4>
            {model.status === 'downloading' && model.progress !== undefined && (
              <div>
                <progress value={model.progress} max="100" />
                <span>{model.progress?.toFixed(2)}%</span>
                {model.downloadedBytes !== undefined && model.totalBytes !== undefined && model.totalBytes !== null && (
                  <span>
                    {' '}({(model.downloadedBytes / (1024 * 1024))?.toFixed(2)} MB / {(model.totalBytes / (1024 * 1024))?.toFixed(2)} MB)
                  </span>
                )}
                 {model.downloadedBytes !== undefined && model.totalBytes === null && (
                  <span>
                    {' '}({(model.downloadedBytes / (1024 * 1024))?.toFixed(2)} MB downloaded)
                  </span>
                )}
              </div>
            )}
            {model.status === 'completed' && (
              <p className="status-message">
                Download complete! (Took: {model.durationSeconds?.toFixed(1)}s)
              </p>
            )}
            {model.status === 'failed' && (
              <p className="status-message error-message">
                Download failed: {model.error}
              </p>
            )}
          </li>
        ))}
      </ul>
      {/* Basic styling - can be moved to a CSS file */}
      <style jsx>{`
        .model-downloader-setup {
          padding: 20px;
          border: 1px solid #ccc;
          border-radius: 8px;
          background-color: #f9f9f9;
        }
        .overall-progress {
          margin-bottom: 20px;
        }
        .overall-progress progress {
          width: 100%;
          height: 20px;
        }
        .overall-progress span {
          margin-left: 10px;
        }
        .model-list {
          list-style-type: none;
          padding: 0;
        }
        .model-item {
          padding: 10px;
          border-bottom: 1px solid #eee;
        }
        .model-item:last-child {
          border-bottom: none;
        }
        .model-item h4 {
          margin-top: 0;
          margin-bottom: 5px;
        }
        .model-item progress {
          width: 80%;
          height: 15px;
          margin-right: 10px;
        }
        .status-message {
          font-style: italic;
        }
        .error-message {
          color: red;
        }
        .model-status-downloading {
          /* Add specific styles if needed */
        }
        .model-status-completed {
          background-color: #e6ffed;
        }
        .model-status-failed {
          background-color: #ffe6e6;
        }
      `}</style>
    </div>
  );
};

export default SetupModelDownloader;