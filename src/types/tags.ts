// Enhanced Tag System Type Definitions
// This file contains all tag-related types for the Metamorphosis project

import { CharacterAttributes } from './character';

// Tag Categories - Hierarchical structure for better organization
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

// Tag Rarity Levels
export type TagRarity = "common" | "uncommon" | "rare" | "legendary";

// Tag Source Types
export type TagSource = "core" | "dlc" | "ugc"; // User-generated content

// Tag Effect System
export type TagEffectType = "immediate" | "gradual" | "conditional";
export type TagEffectTarget = "attribute" | "visual" | "gameplay";
export type TagEffectOperation = "add" | "multiply" | "set" | "toggle";

export interface TagEffect {
  type: TagEffectType;
  target: TagEffectTarget;
  operation: TagEffectOperation;
  magnitude: number;
  duration?: number; // For gradual effects (in seconds)
  condition?: string; // JSON condition for conditional effects
  targetAttribute?: keyof CharacterAttributes; // For attribute effects
  description?: string; // User-friendly description
}

// Tag Attribute Modifiers
export interface AttributeModifier {
  attribute: keyof CharacterAttributes;
  operation: "add" | "multiply" | "set";
  value: number | string;
  temporary?: boolean;
  duration?: number; // For temporary modifiers
}

// Tag Unlock Conditions
export type UnlockConditionType = "attribute" | "tag" | "choice" | "time" | "custom";

export interface UnlockCondition {
  type: UnlockConditionType;
  condition: string; // JSON condition or expression
  description: string; // User-friendly description
  hidden?: boolean; // Hide from UI until discovered
}

// Tag Relationship Types
export interface TagRelationship {
  type: "conflicts" | "requires" | "implies" | "synergizes";
  tagIds: string[];
  description?: string;
  strength?: number; // 0-100 for synergies
}

// Main Tag Interface
export interface Tag {
  // Core identification
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
  rarity: TagRarity;
  
  // Relationships
  relationships: TagRelationship[];
  
  // Effects and modifiers
  effects: TagEffect[];
  modifiers: AttributeModifier[];
  
  // Content management
  unlockConditions: UnlockCondition[];
  source: TagSource;
  contentPack?: string;
  
  // Versioning
  version: string;
  deprecated?: boolean;
  
  // UI metadata
  icon?: string;
  color?: string;
  hidden?: boolean; // For system tags
  
  // Statistics
  usageCount?: number;
  discoveredAt?: Date;
  lastUsed?: Date;
}

// Tag Collection Interface
export interface TagCollection {
  id: string;
  name: string;
  description: string;
  tags: string[]; // Tag IDs
  category: string;
  author?: string;
  version: string;
  public?: boolean;
}

// Tag Search and Filtering
export interface TagFilter {
  categories?: TagCategory[];
  rarity?: TagRarity[];
  source?: TagSource[];
  unlocked?: boolean;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface TagSearchResult {
  tags: Tag[];
  total: number;
  hasMore: boolean;
}

// Tag Validation
export interface TagValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
  suggestions: string[];
}

// Tag Acquisition
export interface TagAcquisition {
  tagId: string;
  source: "creation" | "choice" | "discovery" | "reward" | "trade";
  context?: string; // Scene ID or action context
  timestamp: Date;
  automatic?: boolean; // Was it automatically added?
}

// Tag History
export interface TagHistory {
  characterId: string;
  acquisitions: TagAcquisition[];
  removals: { tagId: string; timestamp: Date; reason: string }[];
}

// Tag Prompt Integration
export interface TagPromptSettings {
  enabled: boolean;
  priority: number;
  weight: number; // 0-2.0 for prompt weighting
  negative?: boolean; // Add to negative prompt
  contextual?: boolean; // Only apply in certain contexts
}

// Tag Effect Context
export interface TagEffectContext {
  character: CharacterAttributes;
  activeTags: string[];
  scene?: string;
  timestamp: Date;
  metadata?: Record<string, any>;
}

// Tag Combination
export interface TagCombination {
  id: string;
  name: string;
  description: string;
  tagIds: string[];
  effects: TagEffect[];
  rarity: TagRarity;
  unlockConditions: UnlockCondition[];
  discoverable?: boolean;
}

// Tag Oracle Integration
export interface TagOracleQuery {
  search?: string;
  category?: number;
  limit?: number;
  offset?: number;
  minPostCount?: number;
}

export interface TagOracleResult {
  name: string;
  category_id: number;
  post_count: number;
  aliases: string[];
}

// Legacy Types for Backward Compatibility
export interface LegacyTag {
  id: string | number;
  name: string;
  description: string;
  category?: TagCategory;
}

// Type guards for runtime checking
export function isTag(obj: any): obj is Tag {
  return (
    typeof obj === 'object' &&
    typeof obj.id === 'string' &&
    typeof obj.name === 'string' &&
    typeof obj.description === 'string' &&
    typeof obj.category === 'string' &&
    Array.isArray(obj.subcategories) &&
    Array.isArray(obj.aliases) &&
    typeof obj.priority === 'number' &&
    Array.isArray(obj.relationships) &&
    Array.isArray(obj.effects) &&
    Array.isArray(obj.modifiers) &&
    Array.isArray(obj.unlockConditions) &&
    typeof obj.version === 'string'
  );
}

export function isLegacyTag(obj: any): obj is LegacyTag {
  return (
    typeof obj === 'object' &&
    (typeof obj.id === 'string' || typeof obj.id === 'number') &&
    typeof obj.name === 'string' &&
    typeof obj.description === 'string'
  );
}

// Utility types
export type TagId = string;
export type TagMap = Record<TagId, Tag>;
export type TagsByCategory = Record<TagCategory, Tag[]>;

// Export all types for easy importing
export type {
  TagCategory,
  TagRarity,
  TagSource,
  TagEffectType,
  TagEffectTarget,
  TagEffectOperation,
  TagEffect,
  AttributeModifier,
  UnlockConditionType,
  UnlockCondition,
  TagRelationship,
  Tag,
  TagCollection,
  TagFilter,
  TagSearchResult,
  TagValidationResult,
  TagAcquisition,
  TagHistory,
  TagPromptSettings,
  TagEffectContext,
  TagCombination,
  TagOracleQuery,
  TagOracleResult,
  LegacyTag,
  TagId,
  TagMap,
  TagsByCategory
};