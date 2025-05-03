import React from 'react';

interface TextInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  error?: string; // Add optional error prop
}

const TextInput: React.FC<TextInputProps> = ({ label, value, onChange, error }) => {
  return (
    <div>
      <label>{label}</label>
      <input type="text" value={value} onChange={(e) => onChange(e.target.value)} />
      {error && <p style={{ color: 'red' }}>{error}</p>} {/* Display error message */}
    </div>
  );
};

export default TextInput;