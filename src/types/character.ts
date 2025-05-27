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
  id: string;
  url: string;
  alt: string;
}