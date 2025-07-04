import { invoke } from '@tauri-apps/api/core'; // For calling Tauri commands
import { CharacterAttributes, LegacyTag } from '../../types/character';
import { Tag } from '../../types/tags';
import { tagService } from '../tags/tagService';

/**
 * Process tags for prompt generation with priority sorting and deduplication
 */
async function processTagsForPrompt(tags: Tag[] | LegacyTag[] | string[]): Promise<string[]> {
  if (!tags || tags.length === 0) {
    return [];
  }

  let processedTags: string[] = [];

  // Handle different tag formats
  if (typeof tags[0] === 'string') {
    // Array of tag IDs
    const tagIds = tags as string[];
    const tagObjects = tagService.getTags(tagIds);
    
    // Sort by priority (higher first) and use danbooru_tag or name
    const sortedTags = tagObjects.sort((a, b) => b.priority - a.priority);
    processedTags = sortedTags.map(tag => tag.danbooru_tag || tag.name);
    
  } else {
    // Array of tag objects (legacy or new)
    const tagObjects = tags as (Tag | LegacyTag)[];
    
    if ('priority' in tagObjects[0]) {
      // New tag system
      const newTags = tagObjects as Tag[];
      const sortedTags = newTags.sort((a, b) => b.priority - a.priority);
      processedTags = sortedTags.map(tag => tag.danbooru_tag || tag.name);
    } else {
      // Legacy tag system
      const legacyTags = tagObjects as LegacyTag[];
      processedTags = legacyTags.map(tag => tag.name);
    }
  }

  // Remove duplicates and filter out system tags
  const uniqueTags = [...new Set(processedTags)]
    .filter(tag => tag && tag.trim() !== '')
    .filter(tag => !tag.startsWith('_')); // Filter out system tags

  return uniqueTags;
}

/**
 * Generates the ComfyUI prompt payload based on character data and workflow type.
 */
export async function generateComfyUIPromptPayload(
  characterData: CharacterAttributes,
  tags: Tag[] | LegacyTag[] | string[], // Support multiple tag formats
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

    // Use enhanced gender expression logic (0=masculine, 100=feminine)
    if (characterData.genderExpression !== undefined) {
      const expression = characterData.genderExpression;
      console.log(`[PromptBuilder] DEBUG: genderExpression=${expression}, anatomy=${characterData.anatomy}`);
      
      if (characterData.anatomy === 'Male') {
        if (expression > 66) {
          console.log(`[PromptBuilder] Adding: 1boy, feminine (expression=${expression})`);
          promptParts.push("1boy", "feminine");
        } else if (expression < 33) {
          console.log(`[PromptBuilder] Adding: 1boy, masculine (expression=${expression})`);
          promptParts.push("1boy", "masculine");
        } else {
          console.log(`[PromptBuilder] Adding: 1boy, androgynous (expression=${expression})`);
          promptParts.push("1boy", "androgynous");
        }
      } else if (characterData.anatomy === 'Female') {
        if (expression > 66) {
          console.log(`[PromptBuilder] Adding: 1girl, feminine (expression=${expression})`);
          promptParts.push("1girl", "feminine");
        } else if (expression < 33) {
          console.log(`[PromptBuilder] Adding: 1girl, masculine (expression=${expression})`);
          promptParts.push("1girl", "masculine");
        } else {
          console.log(`[PromptBuilder] Adding: 1girl, androgynous (expression=${expression})`);
          promptParts.push("1girl", "androgynous");
        }
      }
    }
    
    if (characterData.ethnicity) promptParts.push(characterData.ethnicity);
    if (characterData.hairColor) promptParts.push(characterData.hairColor + " hair");
    if (characterData.eyeColor) promptParts.push(characterData.eyeColor + " eyes");

    if (workflowType === "fullbody" || workflowType === "fullbody_detailer") {
      if (characterData.bodyType) promptParts.push(characterData.bodyType + " body type");
    }
    
    // Enhanced tag processing with priority support
    const processedTags = await processTagsForPrompt(tags);
    promptParts.push(...processedTags);

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