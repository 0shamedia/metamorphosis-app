import React from 'react';

interface SliderInputProps {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step: number;
}

const SliderInput: React.FC<SliderInputProps> = ({ label, value, onChange, min, max, step }) => {
  // Calculate percentage for gradient fill (approximate)
  // const getBackgroundSize = () => { // Original gradient logic, can be removed if not used
  //   return ((value - min) * 100) / (max - min) + '% 100%';
  // };

  const getGenderExpressionLabel = (currentValue: number): string => {
    // This function is specific to the Gender Expression slider.
    // Consider making this more generic if other sliders need such labels.
    if (label === "Gender Expression") { // Only apply special labels for Gender Expression
      // Slider range is 0 to 100 (0=masculine, 100=feminine)
      if (currentValue < 33) return "Masculine"; // 0-32
      if (currentValue <= 66) return "Androgynous";  // 33-66
      return "Feminine"; // 67-100
    }
    return currentValue.toString(); // Default: show numeric value
  };

  return (
    <div className="form-group flex flex-col gap-2">
      <label className="form-label text-sm font-medium text-white/80">
        {label}: <span className="font-semibold text-pink-400">{getGenderExpressionLabel(value)}</span>
      </label>
      <div className="flex items-center gap-4 mt-1"> {/* Reduced gap slightly, removed gender-slider class if not needed elsewhere */}
        {/* Optional: Keep min/max labels if desired for general sliders, or remove for gender expression */}
        {/* <span className="slider-label text-xs text-white/60">{min}</span> */}
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(parseFloat(e.target.value))}
          className="slider flex-1 h-2 bg-gradient-to-r from-blue-500 via-purple-600 to-pink-500 rounded-full appearance-none cursor-pointer
                     focus:outline-none focus:ring-2 focus:ring-pink-500/50
                     
                     [&::-webkit-slider-runnable-track]:h-2
                     [&::-webkit-slider-runnable-track]:rounded-full
                     
                     [&::-moz-range-track]:h-2
                     /* For Firefox, the gradient on the input element itself should style the track */
                     /* Explicitly setting track bg for FF if input bg doesn't work directly */
                     /* [&::-moz-range-track]:bg-gradient-to-r from-blue-500 via-purple-600 to-pink-500 */
                     [&::-moz-range-track]:rounded-full
                     
                     /* Thumb styles */
                     [&::-webkit-slider-thumb]:appearance-none
                     [&::-webkit-slider-thumb]:h-5
                     [&::-webkit-slider-thumb]:w-5
                     [&::-webkit-slider-thumb]:rounded-full
                     [&::-webkit-slider-thumb]:bg-gradient-to-br [&::-webkit-slider-thumb]:from-gray-100 [&::-webkit-slider-thumb]:to-gray-300
                     [&::-webkit-slider-thumb]:shadow-lg
                     [&::-webkit-slider-thumb]:cursor-grab
                     [&::-webkit-slider-thumb]:mt-[-8px] /* Adjust for h-2 track */
                     [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:duration-150 [&::-webkit-slider-thumb]:ease-in-out
                     [&::-webkit-slider-thumb]:hover:brightness-110
                     [&::-webkit-slider-thumb]:active:brightness-90 [&::-webkit-slider-thumb]:active:shadow-xl
                     
                     [&::-moz-range-thumb]:h-5
                     [&::-moz-range-thumb]:w-5
                     [&::-moz-range-thumb]:rounded-full
                     [&::-moz-range-thumb]:bg-gradient-to-br [&::-moz-range-thumb]:from-gray-100 [&::-moz-range-thumb]:to-gray-300
                     /* Firefox might need explicit background-image for gradient on thumb. If so, use inline style or a more specific CSS rule. */
                     /* For Tailwind, direct bg-gradient on pseudo-elements is tricky. The classes above are illustrative. */
                     [&::-moz-range-thumb]:shadow-lg
                     [&::-moz-range-thumb]:cursor-grab
                     [&::-moz-range-thumb]:border-none
                     [&::-moz-range-thumb]:transition-all [&::-moz-range-thumb]:duration-150 [&::-moz-range-thumb]:ease-in-out
                     [&::-moz-range-thumb]:hover:filter [&::-moz-range-thumb]:hover:brightness-110
                     [&::-moz-range-thumb]:active:filter [&::-moz-range-thumb]:active:brightness-90
                    "
          // The gradient is now applied to the input's background, which serves as the track.
          // The ::-webkit-slider-runnable-track and ::-moz-range-track are mostly for height/shape.
          // A common workaround involves a wrapper div with a gradient background and making the actual track transparent,
          // or using JavaScript to update a separate div's width.
          // For simplicity, this uses a solid track color.
          // To somewhat emulate the fill:
          // style={{ backgroundSize: getBackgroundSize() }}
        />
        {/* <span className="slider-label text-xs text-white/60">{max}</span> */}
      </div>
    </div>
  );
};

export default SliderInput;