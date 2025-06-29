import { CharacterAttributes, Tag } from '@/types/character';
import useCharacterStore from '@/store/characterStore';
import { GenerationMode } from '@/types/generation';

/**
 * Defines the optimal, static settings for the ComfyUI workflow.
 * These are high-quality defaults that should not change frequently.
 */
export interface OptimalWorkflowSettings {
  samplerName: string;
  scheduler: string;
  cfg: number;
  // Add other static settings as needed
}

/**
 * Provides the default optimal workflow settings.
 * @returns The optimal workflow settings.
 */
export function get_optimal_workflow_settings(): OptimalWorkflowSettings {
    return {
        samplerName: 'euler_ancestral',
        scheduler: 'karras',
        cfg: 5,
    };
}

/**
 * Generates an array of dynamic prompt fragments based on character attributes and tags.
 * @param attributes The character's attributes.
 * @param tags The character's tags.
 * @returns A promise that resolves to an array of strings for the prompt.
 */
export function setLastGenerationMode(mode: GenerationMode): void {
    useCharacterStore.getState().setLastGenerationMode(mode);
}

export async function get_dynamic_prompt_content(
  attributes: CharacterAttributes,
  tags: Tag[]
): Promise<string[]> {
  const fragments: string[] = [];

  // Process attributes
  if (attributes.name) fragments.push(attributes.name);
  if (attributes.anatomy) fragments.push(attributes.anatomy);
  if (attributes.ethnicity) fragments.push(attributes.ethnicity);
  if (attributes.hairColor) fragments.push(`${attributes.hairColor} hair`);
  if (attributes.eyeColor) fragments.push(`${attributes.eyeColor} eyes`);
  if (attributes.bodyType) fragments.push(attributes.bodyType);
  
  // A simple representation of gender expression
  if (attributes.genderExpression < 40) {
    fragments.push('masculine');
  } else if (attributes.genderExpression > 60) {
    fragments.push('feminine');
  } else {
    fragments.push('androgynous');
  }

  // Process tags
  tags.forEach(tag => {
    fragments.push(tag.name);
  });

  return fragments;
}