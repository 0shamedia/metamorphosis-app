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

// Define union type for tag categories (hierarchical structure)
export type TagCategory = 
  | "appearance"
  | "appearance.body"
  | "appearance.body.anatomy"
  | "appearance.body.transformation"
  | "appearance.clothing"
  | "appearance.clothing.outfit"
  | "appearance.clothing.accessories"
  | "appearance.features"
  | "appearance.features.hair"
  | "appearance.features.eyes"
  | "appearance.features.skin"
  | "identity"
  | "identity.gender"
  | "identity.species"
  | "identity.personality"
  | "status"
  | "status.mental"
  | "status.physical"
  | "status.magical"
  | "environment"
  | "environment.location"
  | "environment.time"
  | "environment.weather"
  | string; // Allow custom categories

// Tag effect types
export type TagEffectType = "immediate" | "gradual" | "conditional";
export type TagEffectTarget = "attribute" | "visual" | "gameplay";

// Tag effect operations
export interface TagEffect {
  type: TagEffectType;
  target: TagEffectTarget;
  operation: "add" | "multiply" | "set" | "toggle";
  magnitude: number;
  duration?: number; // For gradual effects (in seconds)
  condition?: string; // JSON condition for conditional effects
  targetAttribute?: keyof CharacterAttributes; // For attribute effects
}

// Tag attribute modifiers
export interface AttributeModifier {
  attribute: keyof CharacterAttributes;
  operation: "add" | "multiply" | "set";
  value: number | string;
  temporary?: boolean;
}

// Tag unlock conditions
export interface UnlockCondition {
  type: "attribute" | "tag" | "choice" | "time" | "custom";
  condition: string; // JSON condition
  description: string; // User-friendly description
}

// Enhanced tag interface
export interface Tag {
  id: string;
  name: string;
  description: string;
  category: TagCategory;
  subcategories: string[];
  
  // Danbooru integration
  danbooru_tag?: string;
  aliases: string[];
  
  // Metadata
  priority: number; // 0-100, higher = more important in prompts
  rarity: "common" | "uncommon" | "rare" | "legendary";
  conflicts: string[]; // Tag IDs that conflict with this tag
  requires: string[]; // Tag IDs required before this tag
  implies: string[]; // Tag IDs automatically added with this tag
  
  // Effects
  effects: TagEffect[];
  modifiers: AttributeModifier[];
  
  // Content management
  unlockConditions: UnlockCondition[];
  source: "core" | "dlc" | "ugc"; // User-generated content
  contentPack?: string;
  
  // Versioning
  version: string;
  deprecated?: boolean;
  
  // UI metadata
  icon?: string;
  color?: string;
  hidden?: boolean; // For system tags
}

// Legacy tag interface for backward compatibility
export interface LegacyTag {
  id: string | number;
  name: string;
  description: string;
  category?: TagCategory;
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