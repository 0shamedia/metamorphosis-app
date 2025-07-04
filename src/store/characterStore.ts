import { create } from 'zustand';
import { CharacterAttributes, ImageOption } from '../types/character';
import { Tag, TagHistory } from '../types/tags';
import { GenerationMode } from '../types/generation';
import { useTagStore } from './tagStore';
import tagEffectService from '../services/tags/tagEffectService';

export type CharacterCreationStep = 'attributes' | 'faceSelection' | 'bodySelection' | 'finalized';

// For WebSocket progress
export interface GenerationProgress {
  promptId: string | null;
  clientId?: string | null; // Optional: if you want to track it here too
  currentNodeId: string | null;
  currentNodeTitle: string | null; // e.g., "KSampler", "CLIPTextEncode"
  step: number; // Current step for nodes like KSampler
  maxSteps: number; // Max steps for nodes like KSampler
  message: string; // General status message, e.g., "Initializing...", "Queue position: 2"
  queuePosition?: number | null; // Specific queue position
}

interface CharacterState {
  attributes: CharacterAttributes;
  tags: Tag[]; // Legacy - keeping for backward compatibility
  activeTags: string[]; // New tag system - IDs only
  tagHistory: TagHistory | null; // Tag acquisition history
  loading: boolean; // General loading, can be used or refactored
  error: string | null;
  characterImageUrl: string | null; // Potentially for the final full body image

  creationStep: CharacterCreationStep;
  faceOptions: ImageOption[];
  selectedFace: ImageOption | null;
  isGeneratingFace: boolean;
  fullBodyOptions: ImageOption[];
  selectedFullBody: ImageOption | null;
  isGeneratingFullBody: boolean;

  // New state for V2 ComfyUI integration
  clientId: string | null;
  generationProgress: GenerationProgress | null;

  // State for finalized character
  characterId: string | null;
  savedFaceImagePath: string | null;
  savedBodyImagePath: string | null;
  faceSeed: string | number | null;
  bodySeed: string | number | null;

  // Temporary state for received images from WebSocket
  latestImageBlob: Blob | null;
  latestImageUrl: string | null;

  // V3 State
  lastGenerationMode: GenerationMode | null;
  livePreviewUrl: string | null;
}

interface CharacterActions {
  setLivePreviewUrl: (url: string | null) => void;
  setCharacterAttribute: <K extends keyof CharacterAttributes>(attribute: K, value: CharacterAttributes[K]) => void;
  setTags: (tags: Tag[]) => void; // Legacy
  setActiveTags: (tagIds: string[]) => void; // New tag system
  setTagHistory: (history: TagHistory | null) => void;
  addTag: (tagId: string, source?: string) => boolean;
  removeTag: (tagId: string) => boolean;
  applyTagEffects: () => void; // Apply tag effects to attributes
  setLoading: (isLoading: boolean) => void;
  setError: (errorMessage: string | null) => void;
  setCharacterImageUrl: (url: string | null) => void;

  setCreationStep: (step: CharacterCreationStep) => void;
  setFaceOptions: (options: ImageOption[]) => void;
  setSelectedFace: (face: ImageOption | null) => void;
  setIsGeneratingFace: (isLoading: boolean) => void;
  setFullBodyOptions: (options: ImageOption[]) => void;
  setSelectedFullBody: (body: ImageOption | null) => void;
  setIsGeneratingFullBody: (isLoading: boolean) => void;
  resetCreationState: () => void;

  // New actions for V2 ComfyUI integration
  setClientId: (id: string | null) => void;
  setGenerationProgress: (progress: GenerationProgress | null) => void;

  setCharacterId: (characterId: string) => void;
  // Action for finalization
  setFinalizedCharacter: (data: {
    characterId: string;
    attributes: CharacterAttributes; // Pass all attributes again to ensure consistency
    savedFaceImagePath: string;
    savedBodyImagePath: string;
    faceSeed: string | number;
    bodySeed: string | number;
  }) => void;

  // Action for temporary image
  setLatestImage: (blob: Blob | null, url: string | null) => void;

  // V3 Actions
  setLastGenerationMode: (mode: GenerationMode) => void;
  setSavedFaceImagePath: (path: string | null) => void;
  setSavedBodyImagePath: (path: string | null) => void;
  addImageOption: (imageType: 'face' | 'fullbody', option: ImageOption) => void;
}

const initialAttributes: CharacterAttributes = {
  name: '',
  anatomy: '',
  genderExpression: 50,
  ethnicity: '',
  hairColor: '',
  eyeColor: '',
  bodyType: '',
};

const initialGenerationProgress: GenerationProgress | null = null;

const useCharacterStore = create<CharacterState & CharacterActions>((set, get) => ({
  attributes: { ...initialAttributes },
  tags: [], // Legacy
  activeTags: [], // New tag system
  tagHistory: null,
  loading: false,
  error: null,
  characterImageUrl: null,

  creationStep: 'attributes',
  faceOptions: [],
  selectedFace: null,
  isGeneratingFace: false,
  fullBodyOptions: [],
  selectedFullBody: null,
  isGeneratingFullBody: false,

  clientId: null,
  generationProgress: initialGenerationProgress,

  characterId: null,
  savedFaceImagePath: null,
  savedBodyImagePath: null,
  faceSeed: null,
  bodySeed: null,

  latestImageBlob: null,
  latestImageUrl: null,

  // V3
  lastGenerationMode: null,
  livePreviewUrl: null,
 
  setCharacterId: (characterId) => set(() => ({ characterId })),
  setLivePreviewUrl: (url) => set(() => ({ livePreviewUrl: url })),
  setCharacterAttribute: (attribute, value) => {
    set((state) => ({
      attributes: {
        ...state.attributes,
        [attribute]: value,
      },
    }));
    
    // Auto-assign appropriate tags based on attribute changes
    const { activeTags, attributes } = get();
    const newAttributes = { ...attributes, [attribute]: value };
    
    // Auto-tag anatomy moved to face confirmation step instead of immediate application
    // This prevents premature tag application when user might change their mind
    
    // Apply any pending tag effects
    setTimeout(() => get().applyTagEffects(), 0);
  },
  
  setTags: (tags) => set(() => ({ tags })), // Legacy support
  
  setActiveTags: (tagIds) => {
    set(() => ({ activeTags: tagIds }));
    // Sync with tag store
    setTimeout(() => {
      const tagStore = useTagStore.getState();
      tagStore.loadFromCharacter(tagIds, get().tagHistory || undefined);
    }, 0);
  },
  
  setTagHistory: (history) => set(() => ({ tagHistory: history })),
  
  addTag: (tagId, source = 'manual') => {
    const state = get();
    
    // Check if tag already exists
    if (state.activeTags.includes(tagId)) {
      return false;
    }
    
    try {
      // Use tag store for validation
      const tagStore = useTagStore.getState();
      const success = tagStore.addTag(tagId, source);
      
      if (success) {
        set((state) => ({
          activeTags: [...state.activeTags, tagId]
        }));
        
        // Apply tag effects
        get().applyTagEffects();
        return true;
      }
    } catch (error) {
      console.warn('Failed to add tag:', tagId, error);
    }
    
    return false;
  },
  
  removeTag: (tagId) => {
    const state = get();
    
    if (!state.activeTags.includes(tagId)) {
      return false;
    }
    
    // Use tag store for removal
    const tagStore = useTagStore.getState();
    const success = tagStore.removeTag(tagId);
    
    if (success) {
      set((state) => ({
        activeTags: state.activeTags.filter(id => id !== tagId)
      }));
      
      // Apply remaining tag effects
      get().applyTagEffects();
      return true;
    }
    
    return false;
  },
  
  applyTagEffects: () => {
    const { activeTags, attributes } = get();
    
    if (activeTags.length === 0) {
      return;
    }
    
    try {
      const context = {
        character: attributes,
        activeTags,
        scene: 'character_creation',
        timestamp: new Date(),
        metadata: {}
      };
      
      const effects = tagEffectService.calculateEffects(activeTags, context);
      
      // Apply attribute changes
      if (Object.keys(effects.attributeChanges).length > 0) {
        set((state) => ({
          attributes: {
            ...state.attributes,
            ...effects.attributeChanges
          }
        }));
      }
      
      // Update tag store with calculated effects
      const tagStore = useTagStore.getState();
      tagStore.calculateEffects(attributes);
      
    } catch (error) {
      console.error('Failed to apply tag effects:', error);
    }
  },
  setLoading: (isLoading) => set(() => ({ loading: isLoading })),
  setError: (errorMessage) => set(() => ({ error: errorMessage })),
  setCharacterImageUrl: (url) => set(() => ({ characterImageUrl: url })),

  setCreationStep: (step) => set(() => ({ creationStep: step })),
  setFaceOptions: (newlyGeneratedOptions) => // Renamed for clarity
    set((state) => ({
      faceOptions: [...state.faceOptions, ...newlyGeneratedOptions],
      // If new images were generated, select the first one.
      // If no new images, selectedFace becomes null (e.g. if generation failed to produce images).
      selectedFace: newlyGeneratedOptions.length > 0 ? newlyGeneratedOptions[0] : null,
    })),
  setSelectedFace: (face) => set(() => ({ selectedFace: face })),
  setIsGeneratingFace: (isLoading) => set(() => ({ isGeneratingFace: isLoading })),
  setFullBodyOptions: (newlyGeneratedOptions) => // Renamed for clarity
    set((state) => ({
      fullBodyOptions: [...state.fullBodyOptions, ...newlyGeneratedOptions],
      // If new images were generated, select the first one.
      // If no new images, selectedFullBody becomes null.
      selectedFullBody: newlyGeneratedOptions.length > 0 ? newlyGeneratedOptions[0] : null,
    })),
  setSelectedFullBody: (body) => set(() => ({ selectedFullBody: body })),
  setIsGeneratingFullBody: (isLoading) => set(() => ({ isGeneratingFullBody: isLoading })),
  
  setClientId: (id) => set(() => ({ clientId: id })),
  setGenerationProgress: (progress) => set(() => ({ generationProgress: progress })),

  setFinalizedCharacter: (data) => set((state) => ({
    characterId: data.characterId,
    attributes: { ...data.attributes }, // Ensure all attributes are captured
    savedFaceImagePath: data.savedFaceImagePath,
    savedBodyImagePath: data.savedBodyImagePath,
    faceSeed: data.faceSeed,
    bodySeed: data.bodySeed,
    creationStep: 'finalized', // Mark as finalized
    // Optionally clear transient generation states if desired, or keep them for review
    // faceOptions: [],
    // fullBodyOptions: [],
    // selectedFace: null, // Or keep the selected ones for display before navigation
    // selectedFullBody: null,
    // isGeneratingFace: false,
    // isGeneratingFullBody: false,
    // generationProgress: null,
  })),

  setLatestImage: (blob, url) => set(() => ({ latestImageBlob: blob, latestImageUrl: url })),

  // V3
  setLastGenerationMode: (mode) => set(() => ({ lastGenerationMode: mode })),
  setSavedFaceImagePath: (path) => set(() => ({ savedFaceImagePath: path })),
  setSavedBodyImagePath: (path) => set(() => ({ savedBodyImagePath: path })),
  addImageOption: (imageType, option) => set((state) => {
    if (imageType === 'face') {
      return { faceOptions: [...state.faceOptions, option] };
    } else if (imageType === 'fullbody') {
      return { fullBodyOptions: [...state.fullBodyOptions, option] };
    }
    return {};
  }),

  resetCreationState: () => {
    set(() => ({
      attributes: { ...initialAttributes },
      tags: [], // Legacy
      activeTags: [], // Clear new tag system
      tagHistory: null,
      creationStep: 'attributes',
      faceOptions: [],
      selectedFace: null,
      isGeneratingFace: false,
      fullBodyOptions: [],
      selectedFullBody: null,
      isGeneratingFullBody: false,
      characterImageUrl: null,
      error: null,
      clientId: null,
      generationProgress: initialGenerationProgress,
      characterId: null,
      savedFaceImagePath: null,
      savedBodyImagePath: null,
      faceSeed: null,
      bodySeed: null,
      latestImageBlob: null,
      latestImageUrl: null,
      lastGenerationMode: null,
      livePreviewUrl: null,
    }));
    
    // Clear tag store as well
    const tagStore = useTagStore.getState();
    tagStore.clearAllTags();
  },
}));

export default useCharacterStore;