import { invoke } from '@tauri-apps/api/core';
import useCharacterStore from '@/store/characterStore';
import { GenerationMode } from '@/types/generation';
import { CharacterAttributes } from '@/types/character';
import { CharacterGenerationState } from '@/types/rust';
import * as characterStateService from './characterStateService';
import * as promptTemplateService from './promptTemplateService';
import { createWebSocketManager } from './webSocketManager';

interface GenerationResponse {
  prompt_id: string;
  client_id: string;
}

async function runGeneration(generationState: CharacterGenerationState) {
    const { setError, setLoading } = useCharacterStore.getState();
    setLoading(true);
    setError(null);

    try {
        // Log only the essential generation info (not the massive workflow JSON)
        console.log(`[GENERATION] Mode: ${generationState.generationMode}, Prompt: "${generationState.positivePrompt}", Seed: ${generationState.seed}`);
        const response = await invoke<GenerationResponse>('generate_character', { state: generationState });
        
        createWebSocketManager({
            clientId: response.client_id,
            promptId: response.prompt_id,
            inputSeed: generationState.seed,
        });
    } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error("Error during generation orchestration:", errorMessage);
        setError(errorMessage);
        setLoading(false);
    }
}

export async function generateFace(attributes: CharacterAttributes, negativePrompt: string): Promise<void> {
    characterStateService.setLastGenerationMode(GenerationMode.FaceFromPrompt);
    const optimalSettings = characterStateService.get_optimal_workflow_settings();
    const workflowString = await invoke<string>('get_unified_workflow');
    const { characterId } = useCharacterStore.getState();

    const generationState: CharacterGenerationState = {
        workflowJson: workflowString,
        generationMode: GenerationMode.FaceFromPrompt,
        positivePrompt: promptTemplateService.buildCharacterPrompt(attributes),
        negativePrompt: negativePrompt,
        seed: Math.floor(Math.random() * 1_000_000_000),
        steps: 30,
        cfg: optimalSettings.cfg,
        samplerName: optimalSettings.samplerName,
        scheduler: optimalSettings.scheduler,
        denoise: 1.0,
        baseFaceImageFilename: null,
        baseBodyImageFilename: null,
        characterId: characterId,
        context: "character_creation",
    };

    await runGeneration(generationState);
}

export async function generateBodyFromPrompt(baseFaceImageFilename: string, attributes: CharacterAttributes): Promise<void> {
    characterStateService.setLastGenerationMode(GenerationMode.BodyFromPrompt);
    const optimalSettings = characterStateService.get_optimal_workflow_settings();
    const workflowString = await invoke<string>('get_unified_workflow');
    const negativePrompt = "bad quality, worst quality, deformed, ugly, disfigured, missing limbs";
    const { characterId } = useCharacterStore.getState();

    const generationState: CharacterGenerationState = {
        workflowJson: workflowString,
        generationMode: GenerationMode.BodyFromPrompt,
        positivePrompt: promptTemplateService.buildCharacterPrompt(attributes),
        negativePrompt: negativePrompt,
        seed: Math.floor(Math.random() * 1_000_000_000),
        steps: 30,
        cfg: optimalSettings.cfg,
        samplerName: optimalSettings.samplerName,
        scheduler: optimalSettings.scheduler,
        denoise: 1.0,
        baseFaceImageFilename: baseFaceImageFilename,
        baseBodyImageFilename: null,
        characterId: characterId,
        context: "character_creation",
    };

    await runGeneration(generationState);
}