// TagEffectService - Calculate and apply tag effects
import { 
  Tag, 
  TagEffect, 
  TagEffectContext, 
  AttributeModifier,
  TagCombination 
} from '../../types/tags';
import { CharacterAttributes } from '../../types/character';
import tagService from './tagService';

export interface ResolvedEffect {
  tag: Tag;
  effect: TagEffect;
  appliedValue: number;
  description: string;
}

export interface EffectCalculationResult {
  resolvedEffects: ResolvedEffect[];
  attributeChanges: Partial<CharacterAttributes>;
  visualModifiers: Record<string, number>;
  gameplayFlags: Record<string, boolean>;
  combinationBonuses: TagCombination[];
}

class TagEffectService {
  /**
   * Calculate all effects for a set of tags
   */
  calculateEffects(
    tagIds: string[], 
    context: TagEffectContext
  ): EffectCalculationResult {
    const tags = tagService.getTags(tagIds);
    const resolvedEffects: ResolvedEffect[] = [];
    const attributeChanges: Partial<CharacterAttributes> = {};
    const visualModifiers: Record<string, number> = {};
    const gameplayFlags: Record<string, boolean> = {};
    const combinationBonuses: TagCombination[] = [];

    // Process individual tag effects
    tags.forEach(tag => {
      tag.effects.forEach(effect => {
        if (this.shouldApplyEffect(effect, context)) {
          const resolved = this.resolveEffect(tag, effect, context);
          resolvedEffects.push(resolved);
          
          this.applyResolvedEffect(resolved, {
            attributeChanges,
            visualModifiers,
            gameplayFlags
          });
        }
      });

      // Process attribute modifiers
      tag.modifiers.forEach(modifier => {
        this.applyAttributeModifier(modifier, attributeChanges);
      });
    });

    // Check for tag combinations
    const combinations = tagService.getTagCombinations();
    combinations.forEach(combo => {
      if (this.hasAllTags(tagIds, combo.tagIds)) {
        combinationBonuses.push(combo);
        
        // Apply combination effects
        combo.effects.forEach(effect => {
          if (this.shouldApplyEffect(effect, context)) {
            const resolved: ResolvedEffect = {
              tag: { 
                id: combo.id, 
                name: combo.name, 
                description: combo.description 
              } as Tag,
              effect,
              appliedValue: effect.magnitude,
              description: `Combination bonus: ${combo.name}`
            };
            
            resolvedEffects.push(resolved);
            this.applyResolvedEffect(resolved, {
              attributeChanges,
              visualModifiers,
              gameplayFlags
            });
          }
        });
      }
    });

    return {
      resolvedEffects,
      attributeChanges,
      visualModifiers,
      gameplayFlags,
      combinationBonuses
    };
  }

  /**
   * Check if an effect should be applied based on conditions
   */
  private shouldApplyEffect(effect: TagEffect, context: TagEffectContext): boolean {
    if (effect.type === 'conditional' && effect.condition) {
      return this.evaluateCondition(effect.condition, context);
    }
    return true;
  }

  /**
   * Resolve a single effect into its applied value
   */
  private resolveEffect(
    tag: Tag, 
    effect: TagEffect, 
    context: TagEffectContext
  ): ResolvedEffect {
    let appliedValue = effect.magnitude;
    
    // Apply any modifiers based on context
    if (effect.target === 'attribute' && effect.targetAttribute) {
      const currentValue = context.character[effect.targetAttribute];
      
      switch (effect.operation) {
        case 'add':
          appliedValue = effect.magnitude;
          break;
        case 'multiply':
          appliedValue = (currentValue as number) * effect.magnitude;
          break;
        case 'set':
          appliedValue = effect.magnitude;
          break;
        case 'toggle':
          appliedValue = effect.magnitude;
          break;
      }
    }

    return {
      tag,
      effect,
      appliedValue,
      description: effect.description || `${tag.name}: ${effect.operation} ${effect.magnitude}`
    };
  }

  /**
   * Apply a resolved effect to the result accumulator
   */
  private applyResolvedEffect(
    resolved: ResolvedEffect,
    accumulator: {
      attributeChanges: Partial<CharacterAttributes>;
      visualModifiers: Record<string, number>;
      gameplayFlags: Record<string, boolean>;
    }
  ): void {
    const { effect, appliedValue } = resolved;
    
    switch (effect.target) {
      case 'attribute':
        if (effect.targetAttribute) {
          this.applyAttributeEffect(
            effect.targetAttribute,
            effect.operation,
            appliedValue,
            accumulator.attributeChanges
          );
        }
        break;
        
      case 'visual':
        this.applyVisualEffect(
          resolved.tag.id,
          effect.operation,
          appliedValue,
          accumulator.visualModifiers
        );
        break;
        
      case 'gameplay':
        accumulator.gameplayFlags[resolved.tag.id] = effect.operation === 'toggle' 
          ? appliedValue > 0 
          : true;
        break;
    }
  }

  /**
   * Apply an attribute modifier
   */
  private applyAttributeModifier(
    modifier: AttributeModifier,
    attributeChanges: Partial<CharacterAttributes>
  ): void {
    const current = attributeChanges[modifier.attribute] as any;
    
    switch (modifier.operation) {
      case 'add':
        attributeChanges[modifier.attribute] = (current || 0) + (modifier.value as number);
        break;
      case 'multiply':
        attributeChanges[modifier.attribute] = (current || 1) * (modifier.value as number);
        break;
      case 'set':
        attributeChanges[modifier.attribute] = modifier.value as any;
        break;
    }
  }

  /**
   * Apply an effect to a character attribute
   */
  private applyAttributeEffect(
    attribute: keyof CharacterAttributes,
    operation: string,
    value: number,
    attributeChanges: Partial<CharacterAttributes>
  ): void {
    const current = attributeChanges[attribute] as any;
    
    switch (operation) {
      case 'add':
        if (attribute === 'genderExpression') {
          // Clamp gender expression between 0-100
          const newValue = Math.max(0, Math.min(100, (current || 50) + value));
          attributeChanges[attribute] = newValue as any;
        } else {
          attributeChanges[attribute] = (current || 0) + value as any;
        }
        break;
      case 'multiply':
        attributeChanges[attribute] = (current || 1) * value as any;
        break;
      case 'set':
        attributeChanges[attribute] = value as any;
        break;
    }
  }

  /**
   * Apply a visual effect modifier
   */
  private applyVisualEffect(
    tagId: string,
    operation: string,
    value: number,
    visualModifiers: Record<string, number>
  ): void {
    const current = visualModifiers[tagId] || 0;
    
    switch (operation) {
      case 'add':
        visualModifiers[tagId] = current + value;
        break;
      case 'multiply':
        visualModifiers[tagId] = current * value;
        break;
      case 'set':
        visualModifiers[tagId] = value;
        break;
    }
  }

  /**
   * Evaluate a condition string
   */
  private evaluateCondition(condition: string, context: TagEffectContext): boolean {
    try {
      // Simple condition evaluation
      // In a production system, you'd want a proper expression evaluator
      switch (condition) {
        case 'in_transformation_scene':
          return context.scene === 'transformation' || context.scene?.includes('transformation') || false;
        case 'has_magic_potential':
          return context.activeTags.includes('magic_potential');
        default:
          // Try to evaluate as a simple expression
          return this.evaluateSimpleCondition(condition, context);
      }
    } catch (error) {
      console.warn('Failed to evaluate condition:', condition, error);
      return false;
    }
  }

  /**
   * Evaluate simple conditions like "genderExpression >= 60"
   */
  private evaluateSimpleCondition(condition: string, context: TagEffectContext): boolean {
    // Simple parser for attribute conditions
    const attributeRegex = /(\w+)\s*([><=!]+)\s*(\d+)/;
    const match = condition.match(attributeRegex);
    
    if (match) {
      const [, attribute, operator, valueStr] = match;
      const value = parseInt(valueStr);
      const charValue = context.character[attribute as keyof CharacterAttributes];
      
      if (typeof charValue === 'number') {
        switch (operator) {
          case '>=': return charValue >= value;
          case '<=': return charValue <= value;
          case '>': return charValue > value;
          case '<': return charValue < value;
          case '==': return charValue === value;
          case '!=': return charValue !== value;
        }
      } else if (typeof charValue === 'string') {
        switch (operator) {
          case '==': return charValue === valueStr;
          case '!=': return charValue !== valueStr;
        }
      }
    }
    
    return false;
  }

  /**
   * Check if all required tags are present
   */
  private hasAllTags(activeTags: string[], requiredTags: string[]): boolean {
    return requiredTags.every(tag => activeTags.includes(tag));
  }

  /**
   * Get effect preview without applying
   */
  previewEffects(
    tagIds: string[], 
    context: TagEffectContext
  ): EffectCalculationResult {
    return this.calculateEffects(tagIds, context);
  }

  /**
   * Get effect summary for UI display
   */
  getEffectSummary(effects: ResolvedEffect[]): string[] {
    const summaries: string[] = [];
    const groupedEffects = new Map<string, ResolvedEffect[]>();
    
    // Group effects by target
    effects.forEach(effect => {
      const key = `${effect.effect.target}_${effect.effect.targetAttribute || 'general'}`;
      if (!groupedEffects.has(key)) {
        groupedEffects.set(key, []);
      }
      groupedEffects.get(key)!.push(effect);
    });
    
    // Create summaries
    groupedEffects.forEach((effectGroup, key) => {
      if (effectGroup.length === 1) {
        summaries.push(effectGroup[0].description);
      } else {
        const total = effectGroup.reduce((sum, eff) => sum + eff.appliedValue, 0);
        summaries.push(`${key}: ${total > 0 ? '+' : ''}${total}`);
      }
    });
    
    return summaries;
  }
}

// Export singleton instance
export const tagEffectService = new TagEffectService();
export default tagEffectService;