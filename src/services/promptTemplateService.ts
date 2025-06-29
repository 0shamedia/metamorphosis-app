import { GenerationMode } from '@/types/generation';

// Hardcoded templates for now. These can be moved to a config file later.
const templates: Record<GenerationMode, string> = {
  [GenerationMode.FaceFromPrompt]: "photograph of a woman's face, BREAK, detailed skin, BREAK, cinematic lighting",
  [GenerationMode.FaceFromImage]: "photograph of a woman's face, BREAK, detailed skin, BREAK, cinematic lighting",
  [GenerationMode.FullBodyFromPrompt]: "full body photograph of a woman, BREAK, standing in a futuristic city, BREAK, wearing a cyberpunk jacket",
  [GenerationMode.ClothingFromImage]: "a woman wearing BREAK, detailed clothing, BREAK, studio lighting",
  [GenerationMode.ClothingFromPrompt]: "a woman wearing BREAK, detailed clothing, BREAK, studio lighting",
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