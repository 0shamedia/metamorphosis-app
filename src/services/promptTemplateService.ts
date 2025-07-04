import { GenerationMode } from '@/types/generation';
import { CharacterAttributes } from '@/types/character';

// Hardcoded templates for now. These can be moved to a config file later.
const templates: Record<GenerationMode, string> = {
  [GenerationMode.FaceFromPrompt]: "photograph of a woman's face, BREAK, detailed skin, BREAK, cinematic lighting",
  [GenerationMode.BodyFromPrompt]: "full body photograph of a woman, BREAK, standing in a futuristic city, BREAK, wearing a cyberpunk jacket",
  [GenerationMode.ClothingFromPrompt]: "a woman wearing BREAK, detailed clothing, BREAK, studio lighting",
  [GenerationMode.RegenerateFace]: "photograph of a woman's face, BREAK, detailed skin, BREAK, cinematic lighting", // Placeholder
  [GenerationMode.RegenerateBody]: "full body photograph of a woman, BREAK, standing in a futuristic city, BREAK, wearing a cyberpunk jacket", // Placeholder
};

/**
 * Retrieves the base positive prompt template for a given generation mode.
 * @param mode The generation mode.
 * @returns A promise that resolves to the prompt template string.
 */
export async function get_template(mode: GenerationMode): Promise<string> {
  return templates[mode];
}

/**
 * Builds a final prompt by inserting dynamic content into a template.
 * It splits the template by the "BREAK" keyword and inserts dynamic content fragments.
 * @param template The prompt template with "BREAK" placeholders.
 * @param dynamic_content An array of strings to insert into the template.
 * @returns The final, constructed prompt string.
 */
export function build_prompt(template: string, dynamic_content: string[]): string {
  const parts = template.split('BREAK');
  let result = parts[0];

  for (let i = 0; i < dynamic_content.length; i++) {
    result += `, ${dynamic_content[i]}` + (parts[i + 1] || '');
  }

  // If there are more parts than dynamic content, append the rest of the parts.
  if (parts.length > dynamic_content.length + 1) {
      result += parts.slice(dynamic_content.length + 1).join('');
  }

    // Clean up extra commas and whitespace
    return result.replace(/, ,/g, ',').replace(/, /g, ' ').trim();
  }
  
  /**
   * Builds a dynamic prompt string from character attributes.
   * @param attributes The character creation form state.
   * @returns A comma-separated string of prompt tags.
   */
  export function buildCharacterPrompt(attributes: CharacterAttributes): string {
    const tags: string[] = ['clothed'];
  
    // Enhanced gender expression logic (0=masculine, 100=feminine)
    if (attributes.genderExpression !== undefined) {
      const expression = attributes.genderExpression;
      console.log(`[PromptTemplateService] DEBUG: genderExpression=${expression}, anatomy=${attributes.anatomy}`);
      
      if (attributes.anatomy === 'Male') {
        if (expression > 66) {
          console.log(`[PromptTemplateService] Adding: 1boy, feminine (expression=${expression})`);
          tags.push('1boy', 'feminine');
        } else if (expression < 33) {
          console.log(`[PromptTemplateService] Adding: 1boy, masculine (expression=${expression})`);
          tags.push('1boy', 'masculine');
        } else {
          console.log(`[PromptTemplateService] Adding: 1boy, androgynous (expression=${expression})`);
          tags.push('1boy', 'androgynous');
        }
      } else if (attributes.anatomy === 'Female') {
        if (expression > 66) {
          console.log(`[PromptTemplateService] Adding: 1girl, feminine (expression=${expression})`);
          tags.push('1girl', 'feminine');
        } else if (expression < 33) {
          console.log(`[PromptTemplateService] Adding: 1girl, masculine (expression=${expression})`);
          tags.push('1girl', 'masculine');
        } else {
          console.log(`[PromptTemplateService] Adding: 1girl, androgynous (expression=${expression})`);
          tags.push('1girl', 'androgynous');
        }
      }
    }
  
    // Other Attributes
    if (attributes.hairColor) {
      tags.push(`${attributes.hairColor.toLowerCase()} hair`);
    }
    if (attributes.eyeColor) {
      tags.push(`${attributes.eyeColor.toLowerCase()} eyes`);
    }
    if (attributes.bodyType) {
      tags.push(attributes.bodyType.toLowerCase());
    }
    if (attributes.ethnicity) {
      tags.push(attributes.ethnicity.toLowerCase());
    }
  
  
    return tags.join(', ');
  }