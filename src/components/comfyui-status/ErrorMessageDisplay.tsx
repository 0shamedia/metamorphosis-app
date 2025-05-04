import React from 'react';

interface ErrorMessageDisplayProps {
  message: string;
  action?: {
    text: string;
    onClick: () => void;
  } | null;
}

const ErrorMessageDisplay: React.FC<ErrorMessageDisplayProps> = ({ message, action }) => {
  return (
    <div className="bg-red-500 text-white p-3 rounded-md mt-2">
      <div className="font-semibold">Error:</div>
      <div>{message}</div>
      {action && (
        <button
          className="mt-2 px-3 py-1 bg-white text-red-500 rounded hover:bg-red-100"
          onClick={action.onClick}
        >
          {action.text}
        </button>
      )}
    </div>
  );
};

export default ErrorMessageDisplay;