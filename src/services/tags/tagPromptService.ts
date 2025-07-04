// TagPromptService - Convert tags to optimized prompts for AI generation
import { Tag, TagEffectContext } from '../../types/tags';
import { CharacterAttributes } from '../../types/character';
import tagService from './tagService';
import tagEffectService from './tagEffectService';

export interface PromptGenerationOptions {
  includeCharacterAttributes?: boolean;
  priorityThreshold?: number; // Only include tags above this priority
  maxTags?: number; // Limit number of tags
  context?: string; // Scene context for contextual tags
  negativePrompt?: boolean; // Generate negative prompt tags
}

export interface GeneratedPrompt {
  positive: string[];
  negative: string[];
  priority: number;
  totalTags: number;
}

class TagPromptService {
  /**
   * Generate optimized prompt from character and tags
   */
  generatePrompt(
    character: CharacterAttributes,
    tagIds: string[],
    options: PromptGenerationOptions = {}
  ): GeneratedPrompt {
    const {
      includeCharacterAttributes = true,
      priorityThreshold = 0,
      maxTags = 50,
      context = 'character_creation',
      negativePrompt = false
    } = options;

    const positiveParts: string[] = [];
    const negativeParts: string[] = [];
    let totalPriority = 0;

    // Add character attributes if requested
    if (includeCharacterAttributes) {
      const characterParts = this.buildCharacterPrompt(character);
      positiveParts.push(...characterParts);
    }

    // Get and process tags
    const tags = tagService.getTags(tagIds);
    const validTags = tags.filter(tag => 
      tag.priority >= priorityThreshold && 
      !tag.hidden
    );

    // Sort by priority
    const sortedTags = validTags.sort((a, b) => b.priority - a.priority);

    // Limit tags if needed
    const limitedTags = maxTags > 0 ? sortedTags.slice(0, maxTags) : sortedTags;

    // Process tags into prompt parts
    limitedTags.forEach(tag => {
      const promptTag = tag.danbooru_tag || tag.name;
      
      // Check if this is a negative tag or should go to negative prompt
      if (this.isNegativeTag(tag) || (negativePrompt && this.shouldBeNegative(tag))) {
        negativeParts.push(promptTag);
      } else {
        positiveParts.push(promptTag);
        totalPriority += tag.priority;
      }
    });

    // Apply contextual modifications
    if (context) {
      const contextualTags = this.getContextualTags(context, character);
      positiveParts.push(...contextualTags);
    }

    // Remove duplicates while preserving order
    const uniquePositive = [...new Set(positiveParts)];
    const uniqueNegative = [...new Set(negativeParts)];

    return {
      positive: uniquePositive,
      negative: uniqueNegative,
      priority: totalPriority,
      totalTags: limitedTags.length
    };
  }

  /**
   * Build prompt parts from character attributes
   */
  private buildCharacterPrompt(character: CharacterAttributes): string[] {
    const parts: string[] = [];

    // Anatomy and gender expression
    if (character.anatomy === 'Male') {
      if (character.genderExpression > 66) {
        parts.push('1boy', 'feminine');
      } else if (character.genderExpression < 33) {
        parts.push('1boy', 'masculine');
      } else {
        parts.push('1boy', 'androgynous');
      }
    } else if (character.anatomy === 'Female') {
      if (character.genderExpression > 66) {
        parts.push('1girl', 'feminine');
      } else if (character.genderExpression < 33) {
        parts.push('1girl', 'masculine');
      } else {
        parts.push('1girl', 'androgynous');
      }
    }

    // Physical attributes
    if (character.ethnicity) parts.push(character.ethnicity.toLowerCase());
    if (character.hairColor) parts.push(`${character.hairColor.toLowerCase()} hair`);
    if (character.eyeColor) parts.push(`${character.eyeColor.toLowerCase()} eyes`);
    if (character.bodyType) parts.push(`${character.bodyType.toLowerCase()}`);

    return parts;
  }

  /**
   * Check if a tag should be in negative prompt
   */
  private isNegativeTag(tag: Tag): boolean {
    // Tags that should typically go in negative prompts
    const negativeKeywords = ['ugly', 'deformed', 'bad', 'worst', 'low quality', 'blurry'];
    const tagName = tag.name.toLowerCase();
    
    return negativeKeywords.some(keyword => tagName.includes(keyword));
  }

  /**
   * Check if a tag should be moved to negative prompt based on context
   */
  private shouldBeNegative(tag: Tag): boolean {
    // For character creation, we generally don't want negative tags
    // This could be expanded for other contexts
    return false;
  }

  /**
   * Get contextual tags based on scene
   */
  private getContextualTags(context: string, character: CharacterAttributes): string[] {
    const contextualTags: string[] = [];

    switch (context) {
      case 'character_creation':
        contextualTags.push('simple gradient background', 'portrait', 'high quality');
        break;
      case 'bedroom':
        contextualTags.push('bedroom', 'indoors', 'soft lighting');
        break;
      case 'outdoors':
        contextualTags.push('outdoors', 'natural lighting');
        break;
      default:
        // No additional contextual tags
        break;
    }

    return contextualTags;
  }

  /**
   * Convert prompt parts to string
   */
  promptToString(prompt: GeneratedPrompt): { positive: string; negative: string } {
    return {
      positive: prompt.positive.join(', '),
      negative: prompt.negative.join(', ')
    };
  }

  /**
   * Validate prompt length and content
   */
  validatePrompt(prompt: GeneratedPrompt): { valid: boolean; warnings: string[] } {
    const warnings: string[] = [];
    let valid = true;

    // Check prompt length
    const positiveLength = prompt.positive.join(', ').length;
    if (positiveLength > 1000) {
      warnings.push('Positive prompt is very long and may be truncated');
    }

    // Check for conflicting tags
    const conflicts = this.findConflictingTags(prompt.positive);
    if (conflicts.length > 0) {
      warnings.push(`Conflicting tags found: ${conflicts.join(', ')}`);
    }

    // Check for essential tags
    const hasCharacterCount = prompt.positive.some(tag => 
      tag.includes('1girl') || tag.includes('1boy')
    );
    if (!hasCharacterCount) {
      warnings.push('No character count tag (1girl/1boy) found');
    }

    return { valid, warnings };
  }

  /**
   * Find conflicting tags in a prompt
   */
  private findConflictingTags(tags: string[]): string[] {
    const conflicts: string[] = [];
    const tagLower = tags.map(t => t.toLowerCase());

    // Check for obvious conflicts
    const conflictPairs = [
      ['masculine', 'feminine'],
      ['short hair', 'long hair'],
      ['young', 'old'],
      ['small', 'large']
    ];

    conflictPairs.forEach(([tag1, tag2]) => {
      if (tagLower.includes(tag1) && tagLower.includes(tag2)) {
        conflicts.push(`${tag1} vs ${tag2}`);
      }
    });

    return conflicts;
  }

  /**
   * Get prompt statistics
   */
  getPromptStats(prompt: GeneratedPrompt): {
    positiveCount: number;
    negativeCount: number;
    totalLength: number;
    averagePriority: number;
  } {
    const positiveLength = prompt.positive.join(', ').length;
    const negativeLength = prompt.negative.join(', ').length;
    
    return {
      positiveCount: prompt.positive.length,
      negativeCount: prompt.negative.length,
      totalLength: positiveLength + negativeLength,
      averagePriority: prompt.totalTags > 0 ? prompt.priority / prompt.totalTags : 0
    };
  }
}

// Export singleton instance
export const tagPromptService = new TagPromptService();
export default tagPromptService;