// TagStore - Zustand store for tag state management
import { create } from 'zustand';
import { devtools, persist } from 'zustand/middleware';
import { 
  Tag, 
  TagFilter, 
  TagSearchResult, 
  TagValidationResult,
  TagAcquisition,
  TagHistory,
  TagCombination,
  TagEffectContext
} from '../types/tags';
import { CharacterAttributes } from '../types/character';
import tagService from '../services/tags/tagService';
import tagEffectService, { EffectCalculationResult } from '../services/tags/tagEffectService';

export interface TagState {
  // Active tags for current character
  activeTags: string[];
  
  // Available tags (loaded from service)
  availableTags: Tag[];
  loadingTags: boolean;
  
  // Search and filtering
  searchResults: TagSearchResult | null;
  searchLoading: boolean;
  currentFilter: TagFilter;
  
  // Tag validation
  validationResult: TagValidationResult | null;
  
  // Tag acquisition and history
  tagHistory: TagHistory | null;
  lastAcquisition: TagAcquisition | null;
  
  // Effect calculations
  currentEffects: EffectCalculationResult | null;
  effectsLoading: boolean;
  
  // Tag combinations
  discoveredCombinations: TagCombination[];
  availableCombinations: TagCombination[];
  
  // UI state
  selectedCategory: string | null;
  tagBrowserOpen: boolean;
  tagManagerOpen: boolean;
  
  // Actions
  loadTags: () => Promise<void>;
  searchTags: (filter: TagFilter) => Promise<void>;
  
  // Tag management
  addTag: (tagId: string, source?: string, context?: string) => boolean;
  removeTag: (tagId: string, reason?: string) => boolean;
  setActiveTags: (tagIds: string[]) => void;
  clearAllTags: () => void;
  
  // Validation and effects
  validateCurrentTags: () => void;
  calculateEffects: (character: CharacterAttributes, scene?: string) => void;
  
  // Tag acquisition
  acquireTag: (tagId: string, source: TagAcquisition['source'], context?: string) => boolean;
  getTagSuggestions: (limit?: number) => Tag[];
  
  // Combinations
  checkCombinations: () => void;
  discoverCombination: (combination: TagCombination) => void;
  
  // UI actions
  setSelectedCategory: (category: string | null) => void;
  toggleTagBrowser: () => void;
  toggleTagManager: () => void;
  
  // Persistence
  saveToCharacter: () => void;
  loadFromCharacter: (tags: string[], history?: TagHistory) => void;
}

export const useTagStore = create<TagState>()(
  devtools(
    persist(
      (set, get) => ({
        // Initial state
        activeTags: [],
        availableTags: [],
        loadingTags: false,
        searchResults: null,
        searchLoading: false,
        currentFilter: {},
        validationResult: null,
        tagHistory: null,
        lastAcquisition: null,
        currentEffects: null,
        effectsLoading: false,
        discoveredCombinations: [],
        availableCombinations: [],
        selectedCategory: null,
        tagBrowserOpen: false,
        tagManagerOpen: false,

        // Load tags from service
        loadTags: async () => {
          set({ loadingTags: true });
          try {
            await tagService.loadTags();
            const availableTags = tagService.getAllTags();
            const availableCombinations = tagService.getTagCombinations();
            
            set({ 
              availableTags,
              availableCombinations,
              loadingTags: false 
            });
            
            // Validate current tags after loading
            get().validateCurrentTags();
          } catch (error) {
            console.error('Failed to load tags:', error);
            set({ loadingTags: false });
          }
        },

        // Search tags with filters
        searchTags: async (filter: TagFilter) => {
          set({ searchLoading: true, currentFilter: filter });
          try {
            const searchResults = tagService.searchTags(filter);
            set({ searchResults, searchLoading: false });
          } catch (error) {
            console.error('Tag search failed:', error);
            set({ searchLoading: false });
          }
        },

        // Add a tag to active tags
        addTag: (tagId: string, source = 'manual', context = '') => {
          const state = get();
          
          // Check if tag already exists
          if (state.activeTags.includes(tagId)) {
            return false;
          }
          
          // Get tag info
          const tag = tagService.getTag(tagId);
          if (!tag) {
            console.warn('Tag not found:', tagId);
            return false;
          }
          
          // Check if adding this tag would create conflicts
          const testTags = [...state.activeTags, tagId];
          const validation = tagService.validateTags(testTags);
          
          if (!validation.valid) {
            console.warn('Tag conflicts:', validation.errors);
            // Optionally set validation result for UI
            set({ validationResult: validation });
            return false;
          }
          
          // Add the tag
          const newActiveTags = [...state.activeTags, tagId];
          
          // Create acquisition record
          const acquisition: TagAcquisition = {
            tagId,
            source: source as TagAcquisition['source'],
            context,
            timestamp: new Date(),
            automatic: source !== 'manual'
          };
          
          // Update history
          const currentHistory = state.tagHistory || {
            characterId: 'current', // This should come from character store
            acquisitions: [],
            removals: []
          };
          
          const newHistory: TagHistory = {
            ...currentHistory,
            acquisitions: [...currentHistory.acquisitions, acquisition]
          };
          
          set({
            activeTags: newActiveTags,
            lastAcquisition: acquisition,
            tagHistory: newHistory,
            validationResult: { valid: true, errors: [], warnings: [], suggestions: [] }
          });
          
          // Trigger side effects
          get().validateCurrentTags();
          get().checkCombinations();
          
          return true;
        },

        // Remove a tag from active tags
        removeTag: (tagId: string, reason = 'manual removal') => {
          const state = get();
          
          if (!state.activeTags.includes(tagId)) {
            return false;
          }
          
          const newActiveTags = state.activeTags.filter(id => id !== tagId);
          
          // Update history
          const currentHistory = state.tagHistory || {
            characterId: 'current',
            acquisitions: [],
            removals: []
          };
          
          const newHistory: TagHistory = {
            ...currentHistory,
            removals: [...currentHistory.removals, {
              tagId,
              timestamp: new Date(),
              reason
            }]
          };
          
          set({
            activeTags: newActiveTags,
            tagHistory: newHistory
          });
          
          // Trigger side effects
          get().validateCurrentTags();
          get().checkCombinations();
          
          return true;
        },

        // Set active tags (replacing all)
        setActiveTags: (tagIds: string[]) => {
          set({ activeTags: tagIds });
          get().validateCurrentTags();
          get().checkCombinations();
        },

        // Clear all tags
        clearAllTags: () => {
          set({ 
            activeTags: [],
            validationResult: null,
            currentEffects: null,
            discoveredCombinations: []
          });
        },

        // Validate current tags
        validateCurrentTags: () => {
          const { activeTags } = get();
          const validationResult = tagService.validateTags(activeTags);
          set({ validationResult });
        },

        // Calculate effects for current tags
        calculateEffects: (character: CharacterAttributes, scene?: string) => {
          const { activeTags } = get();
          set({ effectsLoading: true });
          
          try {
            const context: TagEffectContext = {
              character,
              activeTags,
              scene,
              timestamp: new Date(),
              metadata: {}
            };
            
            const currentEffects = tagEffectService.calculateEffects(activeTags, context);
            set({ currentEffects, effectsLoading: false });
          } catch (error) {
            console.error('Effect calculation failed:', error);
            set({ effectsLoading: false });
          }
        },

        // Acquire a tag (wrapper around addTag with acquisition logic)
        acquireTag: (tagId: string, source: TagAcquisition['source'], context?: string) => {
          return get().addTag(tagId, source, context);
        },

        // Get tag suggestions based on current tags
        getTagSuggestions: (limit = 10) => {
          const { activeTags } = get();
          return tagService.getTagSuggestions(activeTags, limit);
        },

        // Check for discovered combinations
        checkCombinations: () => {
          const { activeTags, discoveredCombinations, availableCombinations } = get();
          
          const newCombinations = availableCombinations.filter(combo => {
            // Check if combination is already discovered
            if (discoveredCombinations.some(disc => disc.id === combo.id)) {
              return false;
            }
            
            // Check if all required tags are present
            return combo.tagIds.every(tagId => activeTags.includes(tagId));
          });
          
          if (newCombinations.length > 0) {
            set({
              discoveredCombinations: [...discoveredCombinations, ...newCombinations]
            });
            
            // Could trigger UI notifications here
            newCombinations.forEach(combo => {
              console.log('Discovered combination:', combo.name);
            });
          }
        },

        // Manually discover a combination
        discoverCombination: (combination: TagCombination) => {
          const { discoveredCombinations } = get();
          
          if (!discoveredCombinations.some(combo => combo.id === combination.id)) {
            set({
              discoveredCombinations: [...discoveredCombinations, combination]
            });
          }
        },

        // UI state actions
        setSelectedCategory: (category: string | null) => {
          set({ selectedCategory: category });
        },

        toggleTagBrowser: () => {
          set(state => ({ tagBrowserOpen: !state.tagBrowserOpen }));
        },

        toggleTagManager: () => {
          set(state => ({ tagManagerOpen: !state.tagManagerOpen }));
        },

        // Persistence actions
        saveToCharacter: () => {
          // This would integrate with the character store
          // For now, just log
          const { activeTags, tagHistory } = get();
          console.log('Saving tags to character:', { activeTags, tagHistory });
        },

        loadFromCharacter: (tags: string[], history?: TagHistory) => {
          set({
            activeTags: tags,
            tagHistory: history || null
          });
          get().validateCurrentTags();
          get().checkCombinations();
        }
      }),
      {
        name: 'metamorphosis-tag-store',
        // Only persist essential data
        partialize: (state) => ({
          activeTags: state.activeTags,
          tagHistory: state.tagHistory,
          discoveredCombinations: state.discoveredCombinations,
          selectedCategory: state.selectedCategory
        })
      }
    ),
    { name: 'TagStore' }
  )
);

// Convenience hooks for specific parts of the store
export const useActiveTags = () => useTagStore(state => state.activeTags);
export const useTagValidation = () => useTagStore(state => state.validationResult);
export const useTagEffects = () => useTagStore(state => state.currentEffects);
export const useTagCombinations = () => useTagStore(state => state.discoveredCombinations);
export const useTagSuggestions = () => {
  const getSuggestions = useTagStore(state => state.getTagSuggestions);
  return getSuggestions;
};

export default useTagStore;