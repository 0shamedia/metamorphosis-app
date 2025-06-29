import { GenerationMode } from "./generation";

/**
 * This interface defines the data structure sent to the Rust backend
 * to initiate a character generation task using the unified workflow.
 * It is the TypeScript representation of the `CharacterGenerationState` struct in Rust.
 */
export interface CharacterGenerationState {
  generationMode: GenerationMode;
  positivePrompt: string;
  negativePrompt: string;
  seed: number;
  steps: number;
  cfg: number;
  samplerName: string;
  scheduler: string;
  denoise: number;
  baseFaceImageFilename?: string | null;
  baseBodyImageFilename?: string | null;
}