import React from 'react';

interface DropdownSelectProps {
  label: string;
  options: string[];
  value: string;
  onChange: (value: string) => void;
  error?: string; // Add optional error prop
}

const DropdownSelect: React.FC<DropdownSelectProps> = ({ label, options, value, onChange, error }) => {
  return (
    <div>
      <label>{label}</label>
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
      {error && <p style={{ color: 'red' }}>{error}</p>} {/* Display error message */}
    </div>
  );
};

export default DropdownSelect;