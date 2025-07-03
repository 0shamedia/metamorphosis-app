// Define union types for attributes with predefined options
export type Anatomy = "Male" | "Female" | "";
export type Ethnicity = "Caucasian" | "African" | "Asian" | "Hispanic" | "Middle Eastern" | "Mixed" | "";
export type HairColor = "Black" | "Brown" | "Blonde" | "Red" | "Gray" | "White" | "";
export type EyeColor = "Brown" | "Blue" | "Green" | "Hazel" | "Gray" | "";
export type BodyType = "Average" | "Slim" | "Athletic" | "Curvy" | "Plus Size" | "";

// Define the interface for character attributes
export interface CharacterAttributes {
  name: string;
  anatomy: Anatomy;
  genderExpression: number;
  ethnicity: Ethnicity;
  hairColor: HairColor;
  eyeColor: EyeColor;
  bodyType: BodyType;
}

// Define union type for tag categories based on UI plan
export type TagCategory = "clothing" | "transformation" | "gender" | string;

// Define the interface for tags
export interface Tag {
  id: string | number;
  name: string;
  description: string;
  category?: TagCategory; // Optional based on UI plan, but good to include for categorization
}

// Interface for generated image options
export interface ImageOption {
  id: string; // Could be the ComfyUI prompt ID or a client-generated one
  url: string; // This would be the blob URL or data URL from ComfyUI
  alt: string; // Alt text, e.g., "Face option 1"
  seed?: string | number; // The seed used for generation, crucial for naming and reproducibility
  filename?: string; // The filename of the image on disk
  comfyPrompt?: Record<string, any>; // Optional: store the exact prompt sent to ComfyUI
}