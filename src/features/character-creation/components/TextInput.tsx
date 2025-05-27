import React from 'react';

interface TextInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  error?: string; // Add optional error prop
}

const TextInput: React.FC<TextInputProps> = ({ label, value, onChange, error }) => {
  return (
    <div className="form-group flex flex-col gap-2"> {/* Corresponds to form-group and gap from mockup */}
      <label className="form-label text-sm font-medium text-white/80">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="form-input w-full py-3 px-4 bg-white/10 border border-white/20 rounded-lg text-white text-base transition-all duration-200 ease-in-out focus:outline-none focus:bg-white/15 focus:border-pink-500/70 focus:shadow-pink-focus"
        placeholder={`Enter ${label.toLowerCase()}`}
        // The focus:shadow-pink-focus would be a custom shadow, e.g. focus:shadow-[0_0_0_3px_rgba(236,72,153,0.2)]
      />
      {error && <p className="text-pink-400 text-xs mt-1">{error}</p>}
    </div>
  );
};

export default TextInput;