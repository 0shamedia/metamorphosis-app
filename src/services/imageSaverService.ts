import { writeFile, mkdir, exists, BaseDirectory } from '@tauri-apps/plugin-fs';
import { appLocalDataDir, join as pathJoin, sep } from '@tauri-apps/api/path'; // Import sep
import { ImageOption } from '../types/character';

// Helper to generate UUID
const generateUUID = (): string => {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
    const r = (Math.random() * 16) | 0,
      v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
};

// Helper to ensure directory exists
const ensureDirExists = async (relativePathFromAppLocal: string): Promise<string> => {
  try {
    const dirExists = await exists(relativePathFromAppLocal, { baseDir: BaseDirectory.AppLocalData });
    if (!dirExists) {
      await mkdir(relativePathFromAppLocal, { baseDir: BaseDirectory.AppLocalData, recursive: true });
      console.log(`Created directory: ${relativePathFromAppLocal} relative to AppLocalData`);
    }
  } catch (e) {
    console.error(`Error ensuring directory ${relativePathFromAppLocal} exists relative to AppLocalData:`, e);
    throw e; 
  }
  // Return the full absolute path
  const appLocalData = await appLocalDataDir();
  return await pathJoin(appLocalData, relativePathFromAppLocal);
};

// Helper to convert image data (base64 or blob URL) to Uint8Array
const getImageDataAsUint8Array = async (imageUrl: string): Promise<Uint8Array> => {
  if (imageUrl.startsWith('data:image')) {
    const base64Data = imageUrl.split(',')[1];
    if (!base64Data) {
      throw new Error('Invalid base64 image data');
    }
    const binaryString = window.atob(base64Data);
    const len = binaryString.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  } else if (imageUrl.startsWith('blob:')) {
    const response = await fetch(imageUrl);
    const blob = await response.blob();
    return new Uint8Array(await blob.arrayBuffer());
  } else {
    console.warn('Image URL is not a data URL or blob URL, attempting to fetch:', imageUrl);
    const response = await fetch(imageUrl);
    if (!response.ok) {
      throw new Error(`Failed to fetch image from URL: ${response.statusText}`);
    }
    const blob = await response.blob();
    return new Uint8Array(await blob.arrayBuffer());
  }
};

interface SaveImageParams {
  characterId: string;
  imageOption: ImageOption;
  imageType: 'face' | 'body';
  iterationIndex?: number; 
}

interface SavedImagePaths {
  relative: string; // This will be relative to appLocalDataDir/character_images
  absolute: string;
}

export const saveCharacterImage = async ({
  characterId,
  imageOption,
  imageType,
  iterationIndex = 0,
}: SaveImageParams): Promise<SavedImagePaths> => {
  if (!imageOption.url || typeof imageOption.seed === 'undefined') {
    throw new Error('ImageOption is missing url or seed.');
  }
  
  // Construct the path relative to AppLocalDataDir
  const pathSegments = ['character_images', 'characters', characterId, imageType === 'face' ? 'faces' : 'bodies'];
  const relativeBaseDirFromAppLocal = pathSegments.join(await sep()); // Use Tauri's path separator

  // ensureDirExists will create these segments under AppLocalDataDir and return the full absolute path
  const absoluteBaseDir = await ensureDirExists(relativeBaseDirFromAppLocal);

  const timestamp = Date.now();
  const imageName = `${characterId}_${imageType}_${imageOption.seed}_${timestamp}_${iterationIndex}.png`;
  const absoluteImagePath = await pathJoin(absoluteBaseDir, imageName); 
  
  // This relative path is for storing in the character state, relative to the root of "character_images"
  const relativeImagePathForStore = ['characters', characterId, imageType === 'face' ? 'faces' : 'bodies', imageName].join('/');


  try {
    const imageData = await getImageDataAsUint8Array(imageOption.url);
    await writeFile(absoluteImagePath, imageData); // writeFile uses absolute path
    console.log(`Image saved to: ${absoluteImagePath}`);
    return { relative: relativeImagePathForStore, absolute: absoluteImagePath };
  } catch (error) {
    console.error('Error saving image:', error);
    throw new Error(`Failed to save ${imageType} image: ${error instanceof Error ? error.message : String(error)}`);
  }
};

export { generateUUID };