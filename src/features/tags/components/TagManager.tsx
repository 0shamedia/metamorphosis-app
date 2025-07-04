import React, { useState, useEffect } from 'react';
import { Tag, TagValidationResult } from '../../../types/tags';
import { useTagStore, useActiveTags, useTagValidation, useTagEffects } from '../../../store/tagStore';
import tagService from '../../../services/tags/tagService';
import tagEffectService from '../../../services/tags/tagEffectService';

interface TagManagerProps {
  className?: string;
  showEffects?: boolean;
  allowReordering?: boolean;
  maxHeight?: string;
}

const TagManager: React.FC<TagManagerProps> = ({
  className = '',
  showEffects = true,
  allowReordering = true,
  maxHeight = '300px'
}) => {
  const activeTags = useActiveTags();
  const validationResult = useTagValidation();
  const currentEffects = useTagEffects();
  
  const { 
    removeTag, 
    validateCurrentTags,
    getTagSuggestions 
  } = useTagStore();

  const [draggedTag, setDraggedTag] = useState<string | null>(null);
  const [suggestions, setSuggestions] = useState<Tag[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);

  // Get tag objects from IDs
  const tagObjects = tagService.getTags(activeTags);

  // Load suggestions when active tags change
  useEffect(() => {
    if (activeTags.length > 0) {
      const newSuggestions = getTagSuggestions(5);
      setSuggestions(newSuggestions);
    } else {
      setSuggestions([]);
    }
  }, [activeTags, getTagSuggestions]);

  // Validate tags on changes
  useEffect(() => {
    if (activeTags.length > 0) {
      validateCurrentTags();
    }
  }, [activeTags, validateCurrentTags]);

  const handleRemoveTag = (tagId: string) => {
    removeTag(tagId);
  };

  const handleDragStart = (e: React.DragEvent, tagId: string) => {
    if (!allowReordering) return;
    setDraggedTag(tagId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent) => {
    if (!allowReordering) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  const handleDrop = (e: React.DragEvent, targetTagId: string) => {
    if (!allowReordering || !draggedTag) return;
    e.preventDefault();
    
    // TODO: Implement tag reordering logic
    // This would update the order in the tag store
    console.log('Reorder:', draggedTag, 'to position of', targetTagId);
    setDraggedTag(null);
  };

  const getRarityColor = (rarity: string) => {
    switch (rarity) {
      case 'common': return 'border-l-gray-400';
      case 'uncommon': return 'border-l-green-400';
      case 'rare': return 'border-l-blue-400';
      case 'legendary': return 'border-l-purple-400';
      default: return 'border-l-gray-400';
    }
  };

  const getEffectSummary = () => {
    if (!currentEffects || currentEffects.resolvedEffects.length === 0) {
      return null;
    }

    const summaries = tagEffectService.getEffectSummary(currentEffects.resolvedEffects);
    return summaries;
  };

  return (
    <div className={`bg-transparent rounded-lg border-0 ${className}`}>
      {/* Header */}
      <div className="p-4 border-b border-white/10">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-lg font-semibold text-white/90">Active Tags</h3>
          <span className="text-sm text-white/60">
            {activeTags.length} tag{activeTags.length !== 1 ? 's' : ''}
          </span>
        </div>

        {/* Validation Status */}
        {validationResult && (
          <div className="mt-2">
            {validationResult.errors.length > 0 && (
              <div className="bg-red-500/20 border border-red-500/30 rounded-md p-2 mb-2">
                <div className="flex items-center">
                  <svg className="w-4 h-4 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
                  </svg>
                  <span className="text-sm font-medium text-red-300">Conflicts Detected</span>
                </div>
                {validationResult.errors.map((error, index) => (
                  <p key={index} className="text-xs text-red-200 mt-1">{error}</p>
                ))}
              </div>
            )}

            {validationResult.warnings.length > 0 && (
              <div className="bg-yellow-500/20 border border-yellow-500/30 rounded-md p-2 mb-2">
                <div className="flex items-center">
                  <svg className="w-4 h-4 text-yellow-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                  </svg>
                  <span className="text-sm font-medium text-yellow-300">Warnings</span>
                </div>
                {validationResult.warnings.map((warning, index) => (
                  <p key={index} className="text-xs text-yellow-200 mt-1">{warning}</p>
                ))}
              </div>
            )}

            {validationResult.suggestions.length > 0 && (
              <div className="bg-blue-500/20 border border-blue-500/30 rounded-md p-2">
                <div className="flex items-center">
                  <svg className="w-4 h-4 text-blue-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
                  </svg>
                  <span className="text-sm font-medium text-blue-300">Suggestions</span>
                </div>
                {validationResult.suggestions.slice(0, 2).map((suggestion, index) => (
                  <p key={index} className="text-xs text-blue-200 mt-1">{suggestion}</p>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Tag List */}
      <div 
        className="p-4 overflow-y-auto"
        style={{ maxHeight }}
      >
        {activeTags.length === 0 ? (
          <div className="text-center py-8 text-white/50">
            <div className="mb-2">
              <svg className="w-12 h-12 mx-auto text-white/30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
              </svg>
            </div>
            <p>No active tags</p>
            <p className="text-sm mt-1">Add tags from the browser to see them here</p>
          </div>
        ) : (
          <div className="space-y-2">
            {tagObjects.map((tag) => (
              <div
                key={tag.id}
                draggable={allowReordering}
                onDragStart={(e) => handleDragStart(e, tag.id)}
                onDragOver={handleDragOver}
                onDrop={(e) => handleDrop(e, tag.id)}
                className={`
                  group relative bg-white/10 rounded-lg border-l-4 p-3 hover:bg-white/20 transition-colors
                  ${getRarityColor(tag.rarity)}
                  ${allowReordering ? 'cursor-move' : ''}
                  ${draggedTag === tag.id ? 'opacity-50' : ''}
                `}
              >
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-medium text-white/90">{tag.name}</span>
                      {tag.icon && <span>{tag.icon}</span>}
                      <span className="text-xs bg-white/20 px-2 py-1 rounded text-white/70">
                        P{tag.priority}
                      </span>
                    </div>
                    
                    <p className="text-sm text-white/70">{tag.description}</p>
                    
                    {tag.subcategories.length > 0 && (
                      <div className="flex gap-1 mt-1">
                        {tag.subcategories.slice(0, 3).map((sub, index) => (
                          <span key={index} className="text-xs bg-blue-500/30 text-blue-300 px-1 py-0.5 rounded">
                            {sub}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                  
                  {/* Remove Button */}
                  <button
                    onClick={() => handleRemoveTag(tag.id)}
                    className="opacity-0 group-hover:opacity-100 ml-2 p-1 text-red-500 hover:text-red-700 transition-all"
                    title="Remove tag"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Effects Summary */}
      {showEffects && activeTags.length > 0 && (
        <div className="border-t border-white/10 p-4">
          <h4 className="text-sm font-medium text-white/90 mb-2">Effects Summary</h4>
          {currentEffects ? (
            <div className="space-y-1">
              {getEffectSummary()?.map((effect, index) => (
                <div key={index} className="text-xs text-white/70 bg-white/10 px-2 py-1 rounded">
                  {effect}
                </div>
              )) || (
                <p className="text-xs text-white/50">No effects calculated</p>
              )}
              
              {currentEffects.combinationBonuses.length > 0 && (
                <div className="mt-2">
                  <p className="text-xs font-medium text-purple-300">🎉 Combinations:</p>
                  {currentEffects.combinationBonuses.map((combo, index) => (
                    <div key={index} className="text-xs text-purple-300 bg-purple-500/20 px-2 py-1 rounded mt-1">
                      {combo.name}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <p className="text-xs text-white/50">Calculating effects...</p>
          )}
        </div>
      )}

      {/* Suggestions */}
      {suggestions.length > 0 && (
        <div className="border-t border-white/10 p-4">
          <button
            onClick={() => setShowSuggestions(!showSuggestions)}
            className="flex items-center justify-between w-full text-sm font-medium text-white/70 hover:text-white/90"
          >
            <span>💡 Suggested Tags ({suggestions.length})</span>
            <svg 
              className={`w-4 h-4 transform transition-transform ${showSuggestions ? 'rotate-180' : ''}`}
              fill="none" 
              stroke="currentColor" 
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </button>
          
          {showSuggestions && (
            <div className="mt-2 space-y-1">
              {suggestions.map((suggestion) => (
                <div
                  key={suggestion.id}
                  className="flex items-center justify-between text-xs p-2 bg-green-500/20 border border-green-500/30 rounded"
                >
                  <div>
                    <span className="font-medium text-green-300">{suggestion.name}</span>
                    <p className="text-green-200">{suggestion.description}</p>
                  </div>
                  <button
                    onClick={() => {
                      // This would trigger tag addition
                      console.log('Add suggested tag:', suggestion.id);
                    }}
                    className="text-green-300 hover:text-green-100"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                    </svg>
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default TagManager;