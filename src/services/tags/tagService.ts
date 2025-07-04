// TagService - Core tag management operations
import { 
  Tag, 
  TagFilter, 
  TagSearchResult, 
  TagValidationResult, 
  TagCombination,
  TagCategory,
  TagMap,
  TagsByCategory,
  isTag,
  TagOracleQuery,
  TagOracleResult
} from '../../types/tags';

class TagService {
  private tagMap: TagMap = new Map();
  private tagsByCategory: TagsByCategory = {};
  private combinations: TagCombination[] = [];
  private loaded = false;
  private readonly CACHE_KEY = 'metamorphosis_tags_cache';

  constructor() {
    this.loadTags();
  }

  /**
   * Load tags from database files
   */
  async loadTags(): Promise<void> {
    try {
      // Load from cache first
      const cached = this.loadFromCache();
      if (cached) {
        this.tagMap = cached.tagMap;
        this.tagsByCategory = cached.tagsByCategory;
        this.combinations = cached.combinations;
        this.loaded = true;
        return;
      }

      // Load from files
      const [coreTagsResponse, combinationsResponse] = await Promise.all([
        fetch('/resources/tags/core_tags.json'),
        fetch('/resources/tags/tag_combinations.json')
      ]);

      if (!coreTagsResponse.ok || !combinationsResponse.ok) {
        throw new Error('Failed to load tag data files');
      }

      const coreTagsData = await coreTagsResponse.json();
      const combinationsData = await combinationsResponse.json();

      // Process core tags
      this.processCoreTags(coreTagsData.tags);
      
      // Process combinations
      this.combinations = combinationsData.combinations;

      // Cache the loaded data
      this.saveToCache();
      
      this.loaded = true;
      console.log(`Loaded ${this.tagMap.size} tags and ${this.combinations.length} combinations`);
    } catch (error) {
      console.error('Failed to load tags:', error);
      // Load minimal fallback tags
      this.loadFallbackTags();
    }
  }

  /**
   * Process core tags into maps
   */
  private processCoreTags(tags: Tag[]): void {
    this.tagMap.clear();
    this.tagsByCategory = {};

    tags.forEach(tag => {
      if (!isTag(tag)) {
        console.warn('Invalid tag format:', tag);
        return;
      }

      this.tagMap.set(tag.id, tag);
      
      // Group by category
      if (!this.tagsByCategory[tag.category]) {
        this.tagsByCategory[tag.category] = [];
      }
      this.tagsByCategory[tag.category].push(tag);
    });
  }

  /**
   * Load fallback tags for when files fail
   */
  private loadFallbackTags(): void {
    const fallbackTags: Tag[] = [
      {
        id: 'feminine',
        name: 'Feminine',
        description: 'Feminine characteristics',
        category: 'identity.gender',
        subcategories: ['expression'],
        aliases: ['fem'],
        priority: 80,
        rarity: 'common',
        relationships: [],
        effects: [],
        modifiers: [],
        unlockConditions: [],
        source: 'core',
        version: '1.0.0'
      },
      {
        id: 'masculine',
        name: 'Masculine',
        description: 'Masculine characteristics',
        category: 'identity.gender',
        subcategories: ['expression'],
        aliases: ['masc'],
        priority: 80,
        rarity: 'common',
        relationships: [],
        effects: [],
        modifiers: [],
        unlockConditions: [],
        source: 'core',
        version: '1.0.0'
      }
    ];

    this.processCoreTags(fallbackTags);
    this.loaded = true;
  }

  /**
   * Get tag by ID
   */
  getTag(id: string): Tag | undefined {
    return this.tagMap.get(id);
  }

  /**
   * Get multiple tags by IDs
   */
  getTags(ids: string[]): Tag[] {
    return ids.map(id => this.tagMap.get(id)).filter(Boolean) as Tag[];
  }

  /**
   * Get all tags
   */
  getAllTags(): Tag[] {
    return Array.from(this.tagMap.values());
  }

  /**
   * Get tags by category
   */
  getTagsByCategory(category: TagCategory): Tag[] {
    return this.tagsByCategory[category] || [];
  }

  /**
   * Search tags with filters
   */
  searchTags(filter: TagFilter): TagSearchResult {
    let results = Array.from(this.tagMap.values());

    // Apply filters
    if (filter.categories && filter.categories.length > 0) {
      results = results.filter(tag => 
        filter.categories!.some(cat => 
          tag.category === cat || tag.category.startsWith(cat + '.')
        )
      );
    }

    if (filter.rarity && filter.rarity.length > 0) {
      results = results.filter(tag => filter.rarity!.includes(tag.rarity));
    }

    if (filter.source && filter.source.length > 0) {
      results = results.filter(tag => filter.source!.includes(tag.source));
    }

    if (filter.search) {
      const searchLower = filter.search.toLowerCase();
      results = results.filter(tag => 
        tag.name.toLowerCase().includes(searchLower) ||
        tag.description.toLowerCase().includes(searchLower) ||
        tag.aliases.some(alias => alias.toLowerCase().includes(searchLower))
      );
    }

    if (filter.unlocked !== undefined) {
      // TODO: Implement unlock status checking
      // For now, show all tags
    }

    // Sort by priority (higher first) then by name
    results.sort((a, b) => {
      if (a.priority !== b.priority) {
        return b.priority - a.priority;
      }
      return a.name.localeCompare(b.name);
    });

    // Apply pagination
    const offset = filter.offset || 0;
    const limit = filter.limit || 50;
    const total = results.length;
    const paginatedResults = results.slice(offset, offset + limit);

    return {
      tags: paginatedResults,
      total,
      hasMore: offset + limit < total
    };
  }

  /**
   * Validate a set of tags for conflicts and requirements
   */
  validateTags(tagIds: string[]): TagValidationResult {
    const tags = this.getTags(tagIds);
    const errors: string[] = [];
    const warnings: string[] = [];
    const suggestions: string[] = [];

    // Check for conflicts
    tags.forEach(tag => {
      tag.relationships.forEach(rel => {
        if (rel.type === 'conflicts') {
          const conflictingTags = rel.tagIds.filter(id => tagIds.includes(id));
          if (conflictingTags.length > 0) {
            errors.push(`Tag "${tag.name}" conflicts with: ${conflictingTags.map(id => this.getTag(id)?.name).join(', ')}`);
          }
        }
      });
    });

    // Check for missing requirements
    tags.forEach(tag => {
      tag.relationships.forEach(rel => {
        if (rel.type === 'requires') {
          const missingRequired = rel.tagIds.filter(id => !tagIds.includes(id));
          if (missingRequired.length > 0) {
            errors.push(`Tag "${tag.name}" requires: ${missingRequired.map(id => this.getTag(id)?.name).join(', ')}`);
          }
        }
      });
    });

    // Check for implied tags (suggestions)
    tags.forEach(tag => {
      tag.relationships.forEach(rel => {
        if (rel.type === 'implies') {
          const missingImplied = rel.tagIds.filter(id => !tagIds.includes(id));
          if (missingImplied.length > 0) {
            suggestions.push(`Tag "${tag.name}" suggests adding: ${missingImplied.map(id => this.getTag(id)?.name).join(', ')}`);
          }
        }
      });
    });

    // Check for synergies
    tags.forEach(tag => {
      tag.relationships.forEach(rel => {
        if (rel.type === 'synergizes') {
          const synergyTags = rel.tagIds.filter(id => tagIds.includes(id));
          if (synergyTags.length > 0) {
            suggestions.push(`Great combination! "${tag.name}" synergizes well with: ${synergyTags.map(id => this.getTag(id)?.name).join(', ')}`);
          }
        }
      });
    });

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      suggestions
    };
  }

  /**
   * Get tag suggestions based on current tags
   */
  getTagSuggestions(currentTagIds: string[], limit = 10): Tag[] {
    const currentTags = this.getTags(currentTagIds);
    const suggestionMap = new Map<string, number>();

    // Find synergistic tags
    currentTags.forEach(tag => {
      tag.relationships.forEach(rel => {
        if (rel.type === 'synergizes') {
          rel.tagIds.forEach(tagId => {
            if (!currentTagIds.includes(tagId)) {
              const current = suggestionMap.get(tagId) || 0;
              suggestionMap.set(tagId, current + (rel.strength || 50));
            }
          });
        }
      });
    });

    // Find implied tags
    currentTags.forEach(tag => {
      tag.relationships.forEach(rel => {
        if (rel.type === 'implies') {
          rel.tagIds.forEach(tagId => {
            if (!currentTagIds.includes(tagId)) {
              const current = suggestionMap.get(tagId) || 0;
              suggestionMap.set(tagId, current + 30);
            }
          });
        }
      });
    });

    // Sort suggestions by score
    const sortedSuggestions = Array.from(suggestionMap.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit)
      .map(([tagId]) => this.getTag(tagId))
      .filter(Boolean) as Tag[];

    return sortedSuggestions;
  }

  /**
   * Get tag combinations
   */
  getTagCombinations(): TagCombination[] {
    return this.combinations;
  }

  /**
   * Check if a tag combination is discoverable
   */
  checkCombination(tagIds: string[]): TagCombination | null {
    return this.combinations.find(combo => 
      combo.tagIds.every(id => tagIds.includes(id)) &&
      combo.tagIds.length === tagIds.length
    ) || null;
  }

  /**
   * Integration with Tag Oracle MCP server
   */
  async searchTagOracle(query: TagOracleQuery): Promise<TagOracleResult[]> {
    try {
      // This would integrate with the MCP server when available
      // For now, return empty results
      console.log('Tag Oracle integration not yet implemented');
      return [];
    } catch (error) {
      console.error('Tag Oracle search failed:', error);
      return [];
    }
  }

  /**
   * Cache management
   */
  private loadFromCache(): { tagMap: TagMap; tagsByCategory: TagsByCategory; combinations: TagCombination[] } | null {
    try {
      const cached = localStorage.getItem(this.CACHE_KEY);
      if (cached) {
        const data = JSON.parse(cached);
        const tagMap = new Map(data.tagMap);
        return {
          tagMap,
          tagsByCategory: data.tagsByCategory,
          combinations: data.combinations
        };
      }
    } catch (error) {
      console.warn('Failed to load tag cache:', error);
    }
    return null;
  }

  private saveToCache(): void {
    try {
      const data = {
        tagMap: Array.from(this.tagMap.entries()),
        tagsByCategory: this.tagsByCategory,
        combinations: this.combinations,
        timestamp: Date.now()
      };
      localStorage.setItem(this.CACHE_KEY, JSON.stringify(data));
    } catch (error) {
      console.warn('Failed to save tag cache:', error);
    }
  }

  /**
   * Clear cache and reload
   */
  async clearCache(): Promise<void> {
    localStorage.removeItem(this.CACHE_KEY);
    this.loaded = false;
    await this.loadTags();
  }

  /**
   * Check if service is loaded
   */
  isLoaded(): boolean {
    return this.loaded;
  }

  /**
   * Get tag statistics
   */
  getStatistics() {
    return {
      totalTags: this.tagMap.size,
      categories: Object.keys(this.tagsByCategory).length,
      combinations: this.combinations.length,
      coreTagsCount: Array.from(this.tagMap.values()).filter(tag => tag.source === 'core').length,
      dlcTagsCount: Array.from(this.tagMap.values()).filter(tag => tag.source === 'dlc').length,
      ugcTagsCount: Array.from(this.tagMap.values()).filter(tag => tag.source === 'ugc').length
    };
  }
}

// Export singleton instance
export const tagService = new TagService();
export default tagService;