import { fetch } from '@tauri-apps/plugin-http';

const comfyuiApiUrl = 'http://127.0.0.1:8188'; // Default ComfyUI API endpoint

/**
 * Sends a prompt to the ComfyUI API for image generation.
 * @param promptData - The prompt data payload based on character data and workflow.
 * @returns A promise resolving with the API response, including image data or URL.
 */
export async function sendPromptToComfyUI(characterData: CharacterAttributes, tags: Tag[], workflowType: "face" | "fullbody"): Promise<object> {
  try {
    // Generate the prompt payload using the dedicated function
    const promptPayload = await generateComfyUIPromptPayload(characterData, tags, workflowType);

    const response = await fetch(`${comfyuiApiUrl}/prompt`, {
      method: 'POST',
      body: JSON.stringify(promptPayload), // Use the generated payload
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      // Handle non-success HTTP status codes
      const errorBody = await response.text();
      console.error(`ComfyUI API Error: ${response.status} - ${response.statusText}`, errorBody);
      throw new Error(`ComfyUI API Error: ${response.status} - ${response.statusText}`);
    }

    // Assuming the API returns JSON with image data or URL
    const responseData = await response.json();
    console.log('ComfyUI API Response:', responseData);

    // TODO: Further process responseData to extract image data/URL based on actual API structure
    // This might involve handling different response types (e.g., base64 image, file path, etc.)

    return responseData;

  } catch (error) {
    // Handle network errors or other exceptions
    console.error('Error sending prompt to ComfyUI:', error);
    throw error; // Re-throw the error for the caller to handle
  }
}

import { resolveResource } from '@tauri-apps/api/path';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { CharacterAttributes, Tag } from '../types/character';

// TODO: Add more functions as needed for other ComfyUI API interactions (e.g., getting queue status, fetching history)

/**
 * Generates the ComfyUI prompt payload based on character data and workflow type.
 * @param characterData - The character attributes.
 * @param tags - An array of tags associated with the character.
 * @param workflowType - The type of workflow ("face" or "fullbody").
 * @returns A promise resolving with the generated JSON payload.
 */
export async function generateComfyUIPromptPayload(
  characterData: CharacterAttributes,
  tags: Tag[],
  workflowType: "face" | "fullbody"
): Promise<object> {
  try {
    const templateFileName = workflowType === "face"
      ? "face_workflow_template.json"
      : "fullbody_workflow_template.json";

    // Path relative to the src-tauri directory
    // Path relative to the src-tauri directory where resources are bundled
    const templateResourcePath = `../metamorphosis-app/resources/workflows/${templateFileName}`;

    const resolvedPath = await resolveResource(templateResourcePath);
    const templateContent = await readTextFile(resolvedPath);
    const workflowJson = JSON.parse(templateContent);

    // --- Dynamically modify the workflow JSON ---
    // This part requires knowledge of the specific workflow JSON structure.
    // Assuming a basic text-to-image workflow with nodes for positive/negative prompts, seed, and dimensions.
    // Node IDs and field names below are placeholders and need to match the actual JSON templates.

    // Example: Find a node assumed to be for the positive prompt (e.g., a CLIPTextEncode node)
    // and update its text field.
    // This is a simplified example. A real implementation would need to iterate
    // through the nodes and find the correct ones based on their type or other identifiers.
    // Define node IDs and input names based on workflow type
    let positivePromptNodeId: string;
    let negativePromptNodeId: string;
    let seedNodeId: string;
    let widthNodeId: string;
    let heightNodeId: string;

    if (workflowType === "face") {
      positivePromptNodeId = "6";
      negativePromptNodeId = "7";
      seedNodeId = "3";
      widthNodeId = "5";
      heightNodeId = "5";
    } else { // fullbody
      positivePromptNodeId = "2";
      negativePromptNodeId = "3";
      seedNodeId = "5";
      widthNodeId = "4";
      heightNodeId = "4";
    }

    const textInputName = "text";
    const seedInputName = "seed";
    const widthInputName = "width";
    const heightInputName = "height";

    // Construct prompt string from character data and tags
    let positivePrompt = `${characterData.anatomy} ${characterData.ethnicity} ${characterData.hairColor} hair, ${characterData.eyeColor} eyes, ${characterData.bodyType} body`;
    if (characterData.genderExpression !== undefined) {
        positivePrompt += `, gender expression: ${characterData.genderExpression}`;
    }
    tags.forEach(tag => {
      positivePrompt += `, ${tag.name}`;
    });

    // Assuming a negative prompt field exists
    const negativePrompt = "ugly, deformed, low quality"; // Basic negative prompt

    // Update the workflow JSON (placeholders)
    // Update the workflow JSON with actual values
    if (workflowJson[positivePromptNodeId] && workflowJson[positivePromptNodeId].inputs && workflowJson[positivePromptNodeId].inputs[textInputName] !== undefined) {
        workflowJson[positivePromptNodeId].inputs[textInputName] = positivePrompt;
    }
     if (workflowJson[negativePromptNodeId] && workflowJson[negativePromptNodeId].inputs && workflowJson[negativePromptNodeId].inputs[textInputName] !== undefined) {
        workflowJson[negativePromptNodeId].inputs[textInputName] = negativePrompt;
    }
    // Assuming a seed node with a 'seed' input
    if (workflowJson[seedNodeId] && workflowJson[seedNodeId].inputs && workflowJson[seedNodeId].inputs[seedInputName] !== undefined) {
        // Generate a random seed for now
        workflowJson[seedNodeId].inputs[seedInputName] = Math.floor(Math.random() * 1000000000);
    }
    // Assuming nodes for width and height
     if (workflowJson[widthNodeId] && workflowJson[widthNodeId].inputs && workflowJson[widthNodeId].inputs[widthInputName] !== undefined) {
        workflowJson[widthNodeId].inputs[widthInputName] = workflowType === "face" ? 512 : 768; // Example dimensions
    }
     if (workflowJson[heightNodeId] && workflowJson[heightNodeId].inputs && workflowJson[heightNodeId].inputs[heightInputName] !== undefined) {
        workflowJson[heightNodeId].inputs[heightInputName] = workflowType === "face" ? 768 : 1024; // Example dimensions
    }


    // Return the modified JSON payload
    return workflowJson;

  } catch (error) {
    console.error(`Error generating ComfyUI prompt payload for ${workflowType} workflow:`, error);
    throw error;
  }
}