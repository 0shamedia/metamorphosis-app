import React, { useState, useEffect, useMemo } from 'react';
import { Tag, TagCategory, TagFilter } from '../../../types/tags';
import { useTagStore } from '../../../store/tagStore';
import tagService from '../../../services/tags/tagService';

interface TagBrowserProps {
  onTagSelect?: (tag: Tag) => void;
  onTagAdd?: (tagId: string) => void;
  selectedTags?: string[];
  className?: string;
  maxHeight?: string;
}

const TagBrowser: React.FC<TagBrowserProps> = ({
  onTagSelect,
  onTagAdd,
  selectedTags = [],
  className = '',
  maxHeight = '400px'
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<TagCategory | 'all'>('all');
  const [showOnlyUnlocked, setShowOnlyUnlocked] = useState(false);
  const [sortBy, setSortBy] = useState<'name' | 'priority' | 'rarity'>('priority');
  
  const { 
    searchResults, 
    searchLoading, 
    searchTags,
    loadTags,
    activeTags 
  } = useTagStore();

  // Load tags on mount
  useEffect(() => {
    loadTags();
  }, [loadTags]);

  // Search when filters change
  useEffect(() => {
    const filter: TagFilter = {
      search: searchQuery || undefined,
      categories: selectedCategory !== 'all' ? [selectedCategory] : undefined,
      unlocked: showOnlyUnlocked || undefined,
      limit: 50
    };
    
    searchTags(filter);
  }, [searchQuery, selectedCategory, showOnlyUnlocked, searchTags]);

  // Get available categories
  const categories: { value: TagCategory | 'all'; label: string; icon: string }[] = [
    { value: 'all', label: 'All Categories', icon: '🏷️' },
    { value: 'appearance', label: 'Appearance', icon: '👁️' },
    { value: 'appearance.body', label: 'Body', icon: '🧍' },
    { value: 'appearance.clothing', label: 'Clothing', icon: '👗' },
    { value: 'appearance.features', label: 'Features', icon: '😊' },
    { value: 'identity', label: 'Identity', icon: '🆔' },
    { value: 'identity.gender', label: 'Gender', icon: '⚧️' },
    { value: 'identity.personality', label: 'Personality', icon: '🎭' },
    { value: 'status', label: 'Status', icon: '📊' },
    { value: 'environment', label: 'Environment', icon: '🌍' }
  ];

  // Sort and process search results
  const sortedTags = useMemo(() => {
    if (!searchResults) return [];
    
    const tags = [...searchResults.tags];
    
    switch (sortBy) {
      case 'name':
        return tags.sort((a, b) => a.name.localeCompare(b.name));
      case 'priority':
        return tags.sort((a, b) => b.priority - a.priority);
      case 'rarity':
        const rarityOrder = { 'common': 0, 'uncommon': 1, 'rare': 2, 'legendary': 3 };
        return tags.sort((a, b) => rarityOrder[b.rarity] - rarityOrder[a.rarity]);
      default:
        return tags;
    }
  }, [searchResults, sortBy]);

  const handleTagClick = (tag: Tag) => {
    if (onTagSelect) {
      onTagSelect(tag);
    }
    
    if (onTagAdd && !selectedTags.includes(tag.id)) {
      onTagAdd(tag.id);
    }
  };

  const isTagSelected = (tagId: string) => {
    return selectedTags.includes(tagId) || activeTags.includes(tagId);
  };

  const getRarityColor = (rarity: string) => {
    switch (rarity) {
      case 'common': return 'text-gray-600 border-gray-300';
      case 'uncommon': return 'text-green-600 border-green-300';
      case 'rare': return 'text-blue-600 border-blue-300';
      case 'legendary': return 'text-purple-600 border-purple-300';
      default: return 'text-gray-600 border-gray-300';
    }
  };

  const getRarityIcon = (rarity: string) => {
    switch (rarity) {
      case 'common': return '●';
      case 'uncommon': return '◆';
      case 'rare': return '★';
      case 'legendary': return '✦';
      default: return '●';
    }
  };

  return (
    <div className={`bg-white rounded-lg border shadow-lg ${className}`}>
      {/* Header */}
      <div className="p-4 border-b">
        <h3 className="text-lg font-semibold text-gray-900 mb-3">Tag Browser</h3>
        
        {/* Search Bar */}
        <div className="relative mb-3">
          <input
            type="text"
            placeholder="Search tags..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <div className="absolute inset-y-0 right-0 flex items-center pr-3">
            <svg className="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </div>
        </div>

        {/* Filters Row */}
        <div className="flex flex-wrap gap-2 mb-3">
          {/* Category Filter */}
          <select
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value as TagCategory | 'all')}
            className="px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          >
            {categories.map(cat => (
              <option key={cat.value} value={cat.value}>
                {cat.icon} {cat.label}
              </option>
            ))}
          </select>

          {/* Sort Filter */}
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as 'name' | 'priority' | 'rarity')}
            className="px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          >
            <option value="priority">By Priority</option>
            <option value="name">By Name</option>
            <option value="rarity">By Rarity</option>
          </select>

          {/* Unlocked Only Toggle */}
          <label className="flex items-center text-sm">
            <input
              type="checkbox"
              checked={showOnlyUnlocked}
              onChange={(e) => setShowOnlyUnlocked(e.target.checked)}
              className="mr-1"
            />
            Unlocked Only
          </label>
        </div>

        {/* Results Info */}
        <div className="text-sm text-gray-600">
          {searchLoading ? (
            <span>Searching...</span>
          ) : (
            <span>
              {searchResults?.total || 0} tags found
              {searchResults?.hasMore && ' (showing first 50)'}
            </span>
          )}
        </div>
      </div>

      {/* Tag List */}
      <div 
        className="p-4 overflow-y-auto"
        style={{ maxHeight }}
      >
        {searchLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
          </div>
        ) : sortedTags.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            <p>No tags found</p>
            {searchQuery && (
              <p className="text-sm mt-1">Try adjusting your search or filters</p>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-2">
            {sortedTags.map((tag) => {
              const isSelected = isTagSelected(tag.id);
              return (
                <div
                  key={tag.id}
                  onClick={() => handleTagClick(tag)}
                  className={`
                    p-3 rounded-lg border-2 cursor-pointer transition-all duration-200 hover:shadow-md
                    ${isSelected 
                      ? 'border-blue-500 bg-blue-50' 
                      : 'border-gray-200 hover:border-gray-300'
                    }
                    ${getRarityColor(tag.rarity)}
                  `}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-medium text-gray-900">{tag.name}</span>
                        <span className={`text-xs ${getRarityColor(tag.rarity)}`}>
                          {getRarityIcon(tag.rarity)}
                        </span>
                        <span className="text-xs text-gray-500">
                          P{tag.priority}
                        </span>
                        {tag.icon && <span>{tag.icon}</span>}
                      </div>
                      
                      <p className="text-sm text-gray-600 mb-1">{tag.description}</p>
                      
                      <div className="flex items-center gap-2 text-xs text-gray-500">
                        <span className="bg-gray-100 px-2 py-1 rounded">
                          {tag.category}
                        </span>
                        
                        {tag.danbooru_tag && tag.danbooru_tag !== tag.name && (
                          <span className="bg-blue-100 px-2 py-1 rounded text-blue-700">
                            DB: {tag.danbooru_tag}
                          </span>
                        )}
                        
                        {tag.aliases.length > 0 && (
                          <span className="text-gray-400">
                            Aliases: {tag.aliases.slice(0, 2).join(', ')}
                            {tag.aliases.length > 2 && '...'}
                          </span>
                        )}
                      </div>
                    </div>
                    
                    {isSelected && (
                      <div className="text-blue-500 ml-2">
                        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                          <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                        </svg>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};

export default TagBrowser;