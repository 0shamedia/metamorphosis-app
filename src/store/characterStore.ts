import { create } from 'zustand';
import { CharacterAttributes, Tag } from '../types/character';

interface CharacterState {
  attributes: CharacterAttributes;
  tags: Tag[];
  loading: boolean;
  error: string | null;
  characterImageUrl: string | null;
}

interface CharacterActions {
  setCharacterAttribute: <K extends keyof CharacterAttributes>(attribute: K, value: CharacterAttributes[K]) => void;
  setTags: (tags: Tag[]) => void;
  setLoading: (isLoading: boolean) => void;
  setError: (errorMessage: string | null) => void;
  setCharacterImageUrl: (url: string | null) => void;
}

const useCharacterStore = create<CharacterState & CharacterActions>((set) => ({
  attributes: { // Initial empty attributes conforming to CharacterAttributes
    name: '',
    anatomy: '',
    genderExpression: 0,
    ethnicity: '',
    hairColor: '',
    eyeColor: '',
    bodyType: '',
  },
  tags: [], // Initial empty tags array
  loading: false,
  error: null,
  characterImageUrl: null,

  setCharacterAttribute: (attribute, value) =>
    set((state) => ({
      attributes: {
        ...state.attributes,
        [attribute]: value,
      },
    })),
  setTags: (tags) => set(() => ({ tags })),
  setLoading: (isLoading) => set(() => ({ loading: isLoading })),
  setError: (errorMessage) => set(() => ({ error: errorMessage })),
  setCharacterImageUrl: (url) => set(() => ({ characterImageUrl: url })),
}));

export default useCharacterStore;