import { invoke } from '@tauri-apps/api/core'; // For calling Tauri commands
import { CharacterAttributes, Tag } from '../../types/character';

/**
 * Generates the ComfyUI prompt payload based on character data and workflow type.
 */
export async function generateComfyUIPromptPayload(
  characterData: CharacterAttributes,
  tags: Tag[],
  workflowType: "face" | "fullbody" | "fullbody_detailer",
  selectedFaceFilename?: string | null,
  selectedFaceSubfolder?: string | null
): Promise<{ workflowJson: any; outputNodeId: string }> { // Changed workflowJson type to any for now
  try {
    const templateContent = await invoke<string>('get_workflow_template', { workflowType });
    const workflowJson = JSON.parse(templateContent);

    let positivePromptNodeId: string;
    let negativePromptNodeId: string;
    let seedNodeId: string;
    let determinedOutputNodeId: string;
    let loadImageNodeId: string | null = null;
    let insightFaceLoaderNodeId: string | null = null;
    let faceDetailerPositivePromptNodeId: string | null = null;

    if (workflowType === "face") {
      positivePromptNodeId = "6";
      negativePromptNodeId = "7";
      seedNodeId = "3";
      determinedOutputNodeId = "9";
    } else if (workflowType === "fullbody_detailer") {
      positivePromptNodeId = "2";
      negativePromptNodeId = "3";
      seedNodeId = "5";
      loadImageNodeId = "12";
      insightFaceLoaderNodeId = "10";
      faceDetailerPositivePromptNodeId = "22";
      determinedOutputNodeId = "7";
    } else { // "fullbody" (original)
      positivePromptNodeId = "2";
      negativePromptNodeId = "3";
      seedNodeId = "5";
      loadImageNodeId = "12";
      insightFaceLoaderNodeId = "10";
      determinedOutputNodeId = "7";
    }

    const textInputName = "text";
    const seedInputName = "seed";
    const imageNameInput = "image";

    // --- Construct dynamic prompt parts ---
    const promptParts: string[] = [];

    if (characterData.anatomy?.toLowerCase() === "male") {
      promptParts.push("1boy");
    } else if (characterData.anatomy?.toLowerCase() === "female") {
      promptParts.push("1girl");
    }

    if (characterData.genderExpression !== undefined) {
      const expression = characterData.genderExpression;
      if (expression <= -3) promptParts.push("masculine");
      else if (expression <= 2) promptParts.push("androgynous");
      else promptParts.push("feminine");
    }
    
    if (characterData.ethnicity) promptParts.push(characterData.ethnicity);
    if (characterData.hairColor) promptParts.push(characterData.hairColor + " hair");
    if (characterData.eyeColor) promptParts.push(characterData.eyeColor + " eyes");

    if (workflowType === "fullbody" || workflowType === "fullbody_detailer") {
      if (characterData.bodyType) promptParts.push(characterData.bodyType + " body type");
    }
    
    tags.forEach(tag => {
      promptParts.push(tag.name);
    });

    const dynamicPromptContent = promptParts.filter(part => part.trim() !== "").join(", ");

    if (workflowJson[positivePromptNodeId]?.inputs && workflowJson[positivePromptNodeId].inputs[textInputName]) {
      let basePromptText = workflowJson[positivePromptNodeId].inputs[textInputName] as string;
      const placeholder = "__DYNAMIC_PROMPT__";
      if (basePromptText.includes(placeholder)) {
        basePromptText = basePromptText.replace(placeholder, dynamicPromptContent);
      } else {
        console.warn(`Placeholder '${placeholder}' not found in positive prompt node ${positivePromptNodeId}. Appending dynamic content.`);
        if (basePromptText.trim().length > 0 && !basePromptText.trim().endsWith(',')) {
            basePromptText += ", ";
        }
        basePromptText += dynamicPromptContent;
      }
      workflowJson[positivePromptNodeId].inputs[textInputName] = basePromptText;
      console.log(`[PromptBuilder] ${workflowType} Workflow - Main Positive Prompt (Node ${positivePromptNodeId}): ${basePromptText}`);
    }

    if (workflowType === "fullbody_detailer" && faceDetailerPositivePromptNodeId && workflowJson[faceDetailerPositivePromptNodeId]?.inputs) {
      let faceDetailerPrompt = workflowJson[faceDetailerPositivePromptNodeId].inputs[textInputName] as string;
      const faceDetailerAdditions: string[] = [];
      if (characterData.eyeColor) faceDetailerAdditions.push(characterData.eyeColor + " eyes");
      if (characterData.hairColor) faceDetailerAdditions.push(characterData.hairColor + " hair");
      
      if (faceDetailerAdditions.length > 0) {
        if (faceDetailerPrompt.trim().length > 0 && !faceDetailerPrompt.trim().endsWith(',')) {
            faceDetailerPrompt += ", ";
        }
        faceDetailerPrompt += faceDetailerAdditions.join(", ");
      }
      workflowJson[faceDetailerPositivePromptNodeId].inputs[textInputName] = faceDetailerPrompt;
    }

    const negativePromptText = "modern, recent, old, oldest, cartoon, anime, graphic, text, painting, crayon, graphite, abstract, glitch, deformed, mutated, ugly, disfigured, long body, lowres, bad anatomy, bad hands, missing fingers, extra digits, fewer digits, cropped, very displeasing, (worst quality, bad quality:1.2), bad anatomy, sketch, jpeg artifacts, signature, watermark, username, conjoined, bad ai-generated, shine, shiny, porcelain skin, child, loli";
    if (workflowJson[negativePromptNodeId]?.inputs) {
      workflowJson[negativePromptNodeId].inputs[textInputName] = negativePromptText;
    }

    if (workflowJson[seedNodeId]?.inputs) {
      workflowJson[seedNodeId].inputs[seedInputName] = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
    }

    if ((workflowType === "fullbody" || workflowType === "fullbody_detailer") && selectedFaceFilename && loadImageNodeId && workflowJson[loadImageNodeId]?.inputs) {
      const imagePath = selectedFaceSubfolder ? `${selectedFaceSubfolder}/${selectedFaceFilename}` : selectedFaceFilename;
      workflowJson[loadImageNodeId].inputs[imageNameInput] = imagePath;
      console.log(`[PromptBuilder] Set LoadImage node (${loadImageNodeId}) input to: ${imagePath}`);
    }

    if ((workflowType === "fullbody" || workflowType === "fullbody_detailer") && insightFaceLoaderNodeId && workflowJson[insightFaceLoaderNodeId]?.inputs) {
      workflowJson[insightFaceLoaderNodeId].inputs["model_name"] = "buffalo_l";
      console.log(`[PromptBuilder] Set IPAdapterInsightFaceLoader node (${insightFaceLoaderNodeId}) model_name to: buffalo_l`);
    }
    
    return { workflowJson, outputNodeId: determinedOutputNodeId };

  } catch (error) {
    console.error(`Error generating ComfyUI prompt payload for ${workflowType} workflow:`, error);
    throw error;
  }
}