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
    <div className="form-group flex flex-col gap-2"> {/* Corresponds to form-group and gap from mockup */}
      <label className="form-label text-sm font-medium text-white/80">{label}</label>
      <div className="relative"> {/* Wrapper for custom arrow */}
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="form-select w-full py-3 px-4 bg-white/10 border border-white/20 rounded-lg text-white text-base transition-all duration-200 ease-in-out appearance-none cursor-pointer focus:outline-none focus:bg-white/15 focus:border-pink-500/70 focus:shadow-pink-focus"
          // The focus:shadow-pink-focus would be a custom shadow, e.g. focus:shadow-[0_0_0_3px_rgba(236,72,153,0.2)]
        >
          {options.map((option, index) => (
            <option
              key={index}
              value={option}
              disabled={option === ""}
              className="text-black bg-purple-200" // Mockup shows dark text on light bg for options
            >
              {option === "" ? `Select ${label}...` : option}
            </option>
          ))}
        </select>
        {/* Custom Arrow Placeholder - Tailwind doesn't easily style select arrows directly */}
        <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3 text-white/50">
          <svg className="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
            <path d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" />
          </svg>
        </div>
      </div>
      {error && <p className="text-pink-400 text-xs mt-1">{error}</p>}
    </div>
  );
};

export default DropdownSelect;