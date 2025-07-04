import React, { useState, useEffect } from 'react';
import { CharacterAttributes } from '../../../types/character';
import { Tag, TagEffectContext } from '../../../types/tags';
import { useTagStore, useActiveTags, useTagEffects } from '../../../store/tagStore';
import tagEffectService, { EffectCalculationResult, ResolvedEffect } from '../../../services/tags/tagEffectService';
import tagService from '../../../services/tags/tagService';

interface TagEffectVisualizerProps {
  character: CharacterAttributes;
  className?: string;
  showAttributeChanges?: boolean;
  showVisualModifiers?: boolean;
  showGameplayFlags?: boolean;
  showCombinations?: boolean;
  realTimePreview?: boolean;
}

const TagEffectVisualizer: React.FC<TagEffectVisualizerProps> = ({
  character,
  className = '',
  showAttributeChanges = true,
  showVisualModifiers = true,
  showGameplayFlags = false,
  showCombinations = true,
  realTimePreview = true
}) => {
  const activeTags = useActiveTags();
  const currentEffects = useTagEffects();
  const [previewEffects, setPreviewEffects] = useState<EffectCalculationResult | null>(null);
  const [previewTags, setPreviewTags] = useState<string[]>([]);
  const [expandedSections, setExpandedSections] = useState({
    attributes: true,
    visual: false,
    gameplay: false,
    combinations: true
  });

  // Calculate effects when tags or character change
  useEffect(() => {
    if (realTimePreview && activeTags.length > 0) {
      const context: TagEffectContext = {
        character,
        activeTags,
        scene: 'character_creation',
        timestamp: new Date(),
        metadata: {}
      };

      const effects = tagEffectService.calculateEffects(activeTags, context);
      setPreviewEffects(effects);
    }
  }, [activeTags, character, realTimePreview]);

  // Preview effects for a specific set of tags (for preview mode)
  const previewTagEffects = (tagIds: string[]) => {
    if (!realTimePreview) return;
    
    const context: TagEffectContext = {
      character,
      activeTags: tagIds,
      scene: 'character_creation',
      timestamp: new Date(),
      metadata: {}
    };

    const effects = tagEffectService.calculateEffects(tagIds, context);
    setPreviewEffects(effects);
    setPreviewTags(tagIds);
  };

  const clearPreview = () => {
    setPreviewTags([]);
    if (activeTags.length > 0) {
      const context: TagEffectContext = {
        character,
        activeTags,
        scene: 'character_creation',
        timestamp: new Date(),
        metadata: {}
      };
      const effects = tagEffectService.calculateEffects(activeTags, context);
      setPreviewEffects(effects);
    } else {
      setPreviewEffects(null);
    }
  };

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections(prev => ({
      ...prev,
      [section]: !prev[section]
    }));
  };

  const getAttributeChangeDisplay = (attr: keyof CharacterAttributes, value: any, originalValue: any) => {
    if (typeof value === 'number' && typeof originalValue === 'number') {
      const diff = value - originalValue;
      const sign = diff > 0 ? '+' : '';
      return `${originalValue} → ${value} (${sign}${diff})`;
    }
    
    if (value !== originalValue) {
      return `${originalValue} → ${value}`;
    }
    
    return `${value} (no change)`;
  };

  const getEffectsByTag = (effects: ResolvedEffect[]) => {
    const byTag = new Map<string, ResolvedEffect[]>();
    
    effects.forEach(effect => {
      const tagId = effect.tag.id;
      if (!byTag.has(tagId)) {
        byTag.set(tagId, []);
      }
      byTag.get(tagId)!.push(effect);
    });
    
    return byTag;
  };

  const effects = previewEffects || currentEffects;
  const displayTags = previewTags.length > 0 ? previewTags : activeTags;

  if (!effects || displayTags.length === 0) {
    return (
      <div className={`bg-white rounded-lg border p-6 text-center ${className}`}>
        <div className="text-gray-400 mb-2">
          <svg className="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
        </div>
        <p className="text-gray-600">No tag effects to display</p>
        <p className="text-sm text-gray-500 mt-1">Add tags to see their effects on your character</p>
      </div>
    );
  }

  const effectsByTag = getEffectsByTag(effects.resolvedEffects);

  return (
    <div className={`bg-white rounded-lg border shadow-lg ${className}`}>
      {/* Header */}
      <div className="p-4 border-b">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-gray-900">Effect Visualizer</h3>
          {previewTags.length > 0 && (
            <button
              onClick={clearPreview}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              Clear Preview
            </button>
          )}
        </div>
        
        {previewTags.length > 0 && (
          <p className="text-sm text-blue-600 mt-1">
            Previewing effects for {previewTags.length} tag{previewTags.length !== 1 ? 's' : ''}
          </p>
        )}
        
        <p className="text-xs text-gray-500 mt-1">
          {effects.resolvedEffects.length} effect{effects.resolvedEffects.length !== 1 ? 's' : ''} calculated
        </p>
      </div>

      <div className="max-h-96 overflow-y-auto">
        {/* Attribute Changes */}
        {showAttributeChanges && Object.keys(effects.attributeChanges).length > 0 && (
          <div className="border-b">
            <button
              onClick={() => toggleSection('attributes')}
              className="w-full p-4 text-left hover:bg-gray-50 flex items-center justify-between"
            >
              <div className="flex items-center">
                <span className="text-lg mr-2">📊</span>
                <span className="font-medium text-gray-900">Attribute Changes</span>
                <span className="ml-2 text-sm text-gray-500">
                  ({Object.keys(effects.attributeChanges).length})
                </span>
              </div>
              <svg 
                className={`w-4 h-4 transform transition-transform ${expandedSections.attributes ? 'rotate-180' : ''}`}
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            
            {expandedSections.attributes && (
              <div className="px-4 pb-4">
                {Object.entries(effects.attributeChanges).map(([attr, value]) => {
                  const originalValue = character[attr as keyof CharacterAttributes];
                  return (
                    <div key={attr} className="flex items-center justify-between py-2 border-b border-gray-100 last:border-b-0">
                      <span className="text-sm font-medium text-gray-700 capitalize">
                        {attr.replace(/([A-Z])/g, ' $1').trim()}
                      </span>
                      <span className="text-sm text-gray-900 font-mono">
                        {getAttributeChangeDisplay(attr as keyof CharacterAttributes, value, originalValue)}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {/* Visual Modifiers */}
        {showVisualModifiers && Object.keys(effects.visualModifiers).length > 0 && (
          <div className="border-b">
            <button
              onClick={() => toggleSection('visual')}
              className="w-full p-4 text-left hover:bg-gray-50 flex items-center justify-between"
            >
              <div className="flex items-center">
                <span className="text-lg mr-2">🎨</span>
                <span className="font-medium text-gray-900">Visual Modifiers</span>
                <span className="ml-2 text-sm text-gray-500">
                  ({Object.keys(effects.visualModifiers).length})
                </span>
              </div>
              <svg 
                className={`w-4 h-4 transform transition-transform ${expandedSections.visual ? 'rotate-180' : ''}`}
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            
            {expandedSections.visual && (
              <div className="px-4 pb-4">
                {Object.entries(effects.visualModifiers).map(([tagId, modifier]) => {
                  const tag = tagService.getTag(tagId);
                  return (
                    <div key={tagId} className="flex items-center justify-between py-2 border-b border-gray-100 last:border-b-0">
                      <span className="text-sm text-gray-700">
                        {tag?.name || tagId}
                      </span>
                      <span className={`text-sm font-mono ${modifier > 0 ? 'text-green-600' : modifier < 0 ? 'text-red-600' : 'text-gray-600'}`}>
                        {modifier > 0 ? '+' : ''}{modifier}
                      </span>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {/* Gameplay Flags */}
        {showGameplayFlags && Object.keys(effects.gameplayFlags).length > 0 && (
          <div className="border-b">
            <button
              onClick={() => toggleSection('gameplay')}
              className="w-full p-4 text-left hover:bg-gray-50 flex items-center justify-between"
            >
              <div className="flex items-center">
                <span className="text-lg mr-2">🎮</span>
                <span className="font-medium text-gray-900">Gameplay Flags</span>
                <span className="ml-2 text-sm text-gray-500">
                  ({Object.keys(effects.gameplayFlags).length})
                </span>
              </div>
              <svg 
                className={`w-4 h-4 transform transition-transform ${expandedSections.gameplay ? 'rotate-180' : ''}`}
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            
            {expandedSections.gameplay && (
              <div className="px-4 pb-4">
                {Object.entries(effects.gameplayFlags).map(([flag, enabled]) => (
                  <div key={flag} className="flex items-center justify-between py-2 border-b border-gray-100 last:border-b-0">
                    <span className="text-sm text-gray-700 capitalize">
                      {flag.replace(/_/g, ' ')}
                    </span>
                    <span className={`text-sm font-medium ${enabled ? 'text-green-600' : 'text-gray-400'}`}>
                      {enabled ? '✓ Enabled' : '✗ Disabled'}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Tag Combinations */}
        {showCombinations && effects.combinationBonuses.length > 0 && (
          <div className="border-b">
            <button
              onClick={() => toggleSection('combinations')}
              className="w-full p-4 text-left hover:bg-gray-50 flex items-center justify-between"
            >
              <div className="flex items-center">
                <span className="text-lg mr-2">🎉</span>
                <span className="font-medium text-gray-900">Active Combinations</span>
                <span className="ml-2 text-sm text-gray-500">
                  ({effects.combinationBonuses.length})
                </span>
              </div>
              <svg 
                className={`w-4 h-4 transform transition-transform ${expandedSections.combinations ? 'rotate-180' : ''}`}
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            
            {expandedSections.combinations && (
              <div className="px-4 pb-4">
                {effects.combinationBonuses.map((combo, index) => (
                  <div key={index} className="bg-purple-50 border border-purple-200 rounded-lg p-3 mb-2 last:mb-0">
                    <div className="flex items-center justify-between mb-1">
                      <h4 className="font-medium text-purple-900">{combo.name}</h4>
                      <span className="text-xs bg-purple-200 text-purple-800 px-2 py-1 rounded">
                        {combo.rarity}
                      </span>
                    </div>
                    <p className="text-sm text-purple-700 mb-2">{combo.description}</p>
                    <div className="text-xs text-purple-600">
                      Required: {combo.tagIds.map(id => tagService.getTag(id)?.name || id).join(', ')}
                    </div>
                    {combo.effects.length > 0 && (
                      <div className="mt-2">
                        <div className="text-xs font-medium text-purple-800 mb-1">Bonus Effects:</div>
                        {combo.effects.map((effect, effectIndex) => (
                          <div key={effectIndex} className="text-xs text-purple-700">
                            • {effect.target}: {effect.operation} {effect.magnitude}
                            {effect.description && ` - ${effect.description}`}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Effect Details by Tag */}
        <div className="p-4">
          <h4 className="font-medium text-gray-900 mb-3">Effects by Tag</h4>
          {effectsByTag.size === 0 ? (
            <p className="text-sm text-gray-500">No individual tag effects</p>
          ) : (
            <div className="space-y-2">
              {Array.from(effectsByTag.entries()).map(([tagId, tagEffects]) => {
                const tag = tagService.getTag(tagId);
                return (
                  <div key={tagId} className="bg-gray-50 rounded-lg p-3">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-medium text-gray-900">
                        {tag?.icon} {tag?.name || tagId}
                      </span>
                      <span className="text-xs text-gray-500">
                        {tagEffects.length} effect{tagEffects.length !== 1 ? 's' : ''}
                      </span>
                    </div>
                    {tagEffects.map((effect, index) => (
                      <div key={index} className="text-xs text-gray-700 mb-1">
                        <span className="font-medium">
                          {effect.effect.target}
                        </span>
                        {effect.effect.targetAttribute && (
                          <span className="text-gray-500 ml-1">
                            ({effect.effect.targetAttribute})
                          </span>
                        )}
                        : <span className="text-gray-900 ml-1">{effect.description}</span>
                      </div>
                    ))}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default TagEffectVisualizer;