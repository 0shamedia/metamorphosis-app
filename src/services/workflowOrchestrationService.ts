import { invoke } from '@tauri-apps/api/core';
import useCharacterStore from '@/store/characterStore';
import { GenerationMode } from '@/types/generation';
import { CharacterGenerationState } from '@/types/rust';
import * as characterStateService from './characterStateService';
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

export async function generateFace(positivePrompt: string, negativePrompt: string): Promise<void> {
    characterStateService.setLastGenerationMode(GenerationMode.FaceFromPrompt);
    const optimalSettings = characterStateService.get_optimal_workflow_settings();

    const generationState: CharacterGenerationState = {
        generationMode: GenerationMode.FaceFromPrompt,
        positivePrompt: positivePrompt,
        negativePrompt: negativePrompt,
        seed: Math.floor(Math.random() * 1_000_000_000),
        steps: 30,
        cfg: optimalSettings.cfg,
        samplerName: optimalSettings.samplerName,
        scheduler: optimalSettings.scheduler,
        denoise: 1.0,
        baseFaceImageFilename: null,
        baseBodyImageFilename: null,
    };

    await runGeneration(generationState);
}

export async function generateBodyFromPrompt(faceImagePath: string, positivePrompt: string, negativePrompt: string): Promise<void> {
    characterStateService.setLastGenerationMode(GenerationMode.BodyFromPrompt);
    const optimalSettings = characterStateService.get_optimal_workflow_settings();

    try {
        const tempFilename = await invoke<string>('prepare_image_for_edit', { permanentPath: faceImagePath });

        const generationState: CharacterGenerationState = {
            generationMode: GenerationMode.BodyFromPrompt,
            positivePrompt: positivePrompt,
            negativePrompt: negativePrompt,
            seed: Math.floor(Math.random() * 1_000_000_000),
            steps: 30,
            cfg: optimalSettings.cfg,
            samplerName: optimalSettings.samplerName,
            scheduler: optimalSettings.scheduler,
            denoise: 1.0,
            baseFaceImageFilename: tempFilename,
            baseBodyImageFilename: null,
        };

        await runGeneration(generationState);
    } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error("Error preparing image for edit:", errorMessage);
        useCharacterStore.getState().setError(errorMessage);
    }
}