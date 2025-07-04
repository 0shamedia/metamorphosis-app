// Tag Services Export Index
// Central export point for all tag-related services

export { default as tagService } from './tagService';
export { default as tagEffectService } from './tagEffectService';

// Re-export types for convenience
export type {
  Tag,
  TagCategory,
  TagRarity,
  TagSource,
  TagEffect,
  TagEffectType,
  TagEffectTarget,
  TagFilter,
  TagSearchResult,
  TagValidationResult,
  TagAcquisition,
  TagHistory,
  TagCombination,
  TagEffectContext,
  AttributeModifier,
  UnlockCondition,
  TagRelationship,
  TagMap,
  TagsByCategory
} from '../../types/tags';

export type {
  ResolvedEffect,
  EffectCalculationResult
} from './tagEffectService';