import React, { useState, useEffect } from 'react';
import TextInput from './TextInput';
import DropdownSelect from './DropdownSelect';
import SliderInput from './SliderInput';
import useCharacterStore from '../../../store/characterStore';
// import { sendPromptToComfyUI } from '../../../services/comfyuiService'; // Removed
import { Anatomy, Ethnicity, HairColor, EyeColor, BodyType } from '../../../types/character';

const AttributeInputForm: React.FC = () => {
  const [name, setName] = useState('Player');
  const { setCharacterAttribute } = useCharacterStore(); // Removed setLoading and setError as they are not used here
  const [anatomy, setAnatomy] = useState('');
  const [genderExpression, setGenderExpression] = useState(0);
  const [ethnicity, setEthnicity] = useState('');
  const [hairColor, setHairColor] = useState('');
  const [eyeColor, setEyeColor] = useState('');
  const [bodyType, setBodyType] = useState('');

  // State for validation errors
  const [nameError, setNameError] = useState('');
  const [anatomyError, setAnatomyError] = useState('');
  const [ethnicityError, setEthnicityError] = useState('');
  const [hairColorError, setHairColorError] = useState('');
  const [eyeColorError, setEyeColorError] = useState('');
  const [bodyTypeError, setBodyTypeError] = useState('');

  // Validation logic
  const validateName = (value: string) => {
    if (!value.trim()) {
      setNameError('Name cannot be empty');
    } else {
      setNameError('');
    }
  };

  const validateDropdown = (value: string, setError: React.Dispatch<React.SetStateAction<string>>, fieldName: string) => {
    if (!value) {
      setError(`${fieldName} must be selected`);
    } else {
      setError('');
    }
  };

  // Update state and validate on change
  const handleNameChange = (value: string) => {
    setName(value);
    validateName(value);
    setCharacterAttribute('name', value as string);
  };

  const handleAnatomyChange = (value: string) => {
    setAnatomy(value);
    validateDropdown(value, setAnatomyError, 'Anatomy');
    setCharacterAttribute('anatomy', value as Anatomy);
  };

  const handleGenderExpressionChange = (value: number) => {
    setGenderExpression(value);
    setCharacterAttribute('genderExpression', value);
    // No validation needed for slider as it has a default value
  };

  const handleEthnicityChange = (value: string) => {
    setEthnicity(value);
    validateDropdown(value, setEthnicityError, 'Ethnicity');
    setCharacterAttribute('ethnicity', value as Ethnicity);
  };

  const handleHairColorChange = (value: string) => {
    setHairColor(value);
    validateDropdown(value, setHairColorError, 'Hair Color');
    setCharacterAttribute('hairColor', value as HairColor);
  };

  const handleEyeColorChange = (value: string) => {
    setEyeColor(value);
    validateDropdown(value, setEyeColorError, 'Eye Color');
    setCharacterAttribute('eyeColor', value as EyeColor);
  };

  const handleBodyTypeChange = (value: string) => {
    setBodyType(value);
    validateDropdown(value, setBodyTypeError, 'Body Type');
    setCharacterAttribute('bodyType', value as BodyType);
  };

  // Initial validation on mount (for default values)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    validateName(name);
    validateDropdown(anatomy, setAnatomyError, 'Anatomy');
    validateDropdown(ethnicity, setEthnicityError, 'Ethnicity');
    validateDropdown(hairColor, setHairColorError, 'Hair Color');
    validateDropdown(eyeColor, setEyeColorError, 'Eye Color');
    validateDropdown(bodyType, setBodyTypeError, 'Body Type');
  }, []); // Empty dependency array ensures this runs only once on mount

  // Determine if the button should be enabled
  const isFormValid = !nameError && !anatomyError && !ethnicityError && !hairColorError && !eyeColorError && !bodyTypeError &&
                      name.trim() !== '' && anatomy !== '' && ethnicity !== '' && hairColor !== '' && eyeColor !== '' && bodyType !== '';


  // const handleGenerate = async () => { // Removed this function
  //   setLoading(true);
  //   setError(null);

  //   const characterAttributes = useCharacterStore.getState().attributes;

  //   // Construct positive prompt
  //   try {
  //     // Call sendPromptToComfyUI with character data, tags (empty for now), and workflow type
  //     const response = await sendPromptToComfyUI(characterAttributes, [], "face");
  //     console.log('ComfyUI API Response:', response);
  //     // TODO: Handle the response, e.g., update characterImageUrl state
  //     setLoading(false);
  //   } catch (error) {
  //     console.error('Error generating character:', error);
  //     setError('Failed to generate character.');
  //     setLoading(false);
  //   }
  // };

  return (
    <div className="p-4 rounded-lg space-y-6">
      <h2 className="text-2xl font-semibold text-purple-100 mb-6 text-center" style={{ textShadow: '0 0 8px rgba(220, 180, 255,0.4)' }}>Define Attributes</h2>
      <TextInput label="Name" value={name} onChange={handleNameChange} error={nameError} />
      <DropdownSelect
        label="Anatomy"
        options={["", "Male", "Female"]}
        value={anatomy}
        onChange={handleAnatomyChange}
        error={anatomyError}
      />
      <SliderInput
        label="Gender Expression"
        min={-10}
        max={10}
        step={1}
        value={genderExpression}
        onChange={handleGenderExpressionChange}
      />
      <DropdownSelect
        label="Ethnicity"
        options={["", "Caucasian", "African", "Asian", "Hispanic", "Middle Eastern", "Mixed"]}
        value={ethnicity}
        onChange={handleEthnicityChange}
        error={ethnicityError}
      />
      <DropdownSelect
        label="Hair Color"
        options={["", "Black", "Brown", "Blonde", "Red", "Gray", "White"]}
        value={hairColor}
        onChange={handleHairColorChange}
        error={hairColorError}
      />
      <DropdownSelect
        label="Eye Color"
        options={["", "Brown", "Blue", "Green", "Hazel", "Gray"]}
        value={eyeColor}
        onChange={handleEyeColorChange}
        error={eyeColorError}
      />
      <DropdownSelect
        label="Body Type"
        options={["", "Average", "Slim", "Athletic", "Curvy", "Plus Size"]}
        value={bodyType}
        onChange={handleBodyTypeChange}
        error={bodyTypeError}
      />

      {/* The "Generate Face" button is now part of the parent page.tsx for layout reasons */}
      {/* <button
        onClick={handleGenerate}
        disabled={!isFormValid}
        className={`w-full mt-6 font-semibold py-3 px-6 rounded-lg shadow-md hover:shadow-lg transition-all duration-300 ease-in-out transform hover:scale-105 focus:outline-none focus:ring-2 focus:ring-opacity-75 ${
          isFormValid
          ? 'bg-gradient-to-r from-pink-500 to-purple-600 hover:from-pink-600 hover:to-purple-700 text-white focus:ring-pink-400'
          : 'bg-gray-700 text-gray-400 cursor-not-allowed opacity-70'
        }`}
      >
        Generate Face (from Form)
      </button> */}
    </div>
  );
};

export default AttributeInputForm;