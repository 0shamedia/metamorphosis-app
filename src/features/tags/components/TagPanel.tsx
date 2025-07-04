import React, { useState, useEffect } from 'react';
import { CharacterAttributes } from '../../../types/character';
import { Tag } from '../../../types/tags';
import { useTagStore } from '../../../store/tagStore';
import useCharacterStore from '../../../store/characterStore';
import TagBrowser from './TagBrowser';
import TagManager from './TagManager';
import TagAcquisition from './TagAcquisition';
import TagEffectVisualizer from './TagEffectVisualizer';

interface TagPanelProps {
  character: CharacterAttributes;
  onCharacterUpdate?: (attributes: Partial<CharacterAttributes>) => void;
  className?: string;
  layout?: 'sidebar' | 'modal' | 'inline';
  showEffectVisualizer?: boolean;
  showAcquisitionFlow?: boolean;
}

const TagPanel: React.FC<TagPanelProps> = ({
  character,
  onCharacterUpdate,
  className = '',
  layout = 'sidebar',
  showEffectVisualizer = true,
  showAcquisitionFlow = true
}) => {
  const [activeTab, setActiveTab] = useState<'browse' | 'manage' | 'effects'>('manage');
  const [acquisitionOpen, setAcquisitionOpen] = useState(false);
  
  const { 
    loadTags, 
    addTag, 
    lastAcquisition,
    activeTags,
    currentEffects
  } = useTagStore();

  const { 
    addTag: addCharacterTag,
    removeTag: removeCharacterTag,
    applyTagEffects 
  } = useCharacterStore();

  // Load tags on mount
  useEffect(() => {
    loadTags();
  }, [loadTags]);

  // Open acquisition flow when new tags are acquired
  useEffect(() => {
    if (showAcquisitionFlow && lastAcquisition && !acquisitionOpen) {
      setAcquisitionOpen(true);
    }
  }, [lastAcquisition, showAcquisitionFlow, acquisitionOpen]);

  // Apply effects to character when they change
  useEffect(() => {
    if (currentEffects && Object.keys(currentEffects.attributeChanges).length > 0 && onCharacterUpdate) {
      onCharacterUpdate(currentEffects.attributeChanges);
    }
  }, [currentEffects, onCharacterUpdate]);

  const handleTagAdd = (tagId: string) => {
    const success = addTag(tagId, 'manual');
    if (success) {
      // Also update character store
      addCharacterTag(tagId, 'manual');
    }
  };

  const handleTagRemove = (tagId: string) => {
    removeCharacterTag(tagId);
  };

  const handleAcquisitionAccept = (tagId: string) => {
    // Ensure tag is properly added if not already present
    if (!activeTags.includes(tagId)) {
      addCharacterTag(tagId, 'acquisition');
    }
    console.log('Tag acquisition accepted:', tagId);
  };

  const handleAcquisitionReject = (tagId: string) => {
    // Remove the tag if it exists
    if (activeTags.includes(tagId)) {
      removeCharacterTag(tagId);
    }
    console.log('Tag acquisition rejected:', tagId);
  };

  const tabs = [
    { id: 'manage' as const, label: 'Active Tags', icon: '🏷️', count: activeTags.length },
    { id: 'browse' as const, label: 'Browse Tags', icon: '🔍', count: undefined },
    ...(showEffectVisualizer ? [{ id: 'effects' as const, label: 'Effects', icon: '⚡', count: currentEffects?.resolvedEffects.length }] : [])
  ];

  const getLayoutClasses = () => {
    switch (layout) {
      case 'modal':
        return 'bg-gray-900/95 backdrop-blur-md rounded-lg shadow-xl max-w-4xl w-full max-h-screen overflow-hidden border border-white/10';
      case 'inline':
        return 'bg-transparent rounded-lg border-0';
      case 'sidebar':
      default:
        return 'bg-transparent h-full flex flex-col';
    }
  };

  return (
    <>
      <div className={`${getLayoutClasses()} ${className}`}>
        {/* Header with tabs */}
        <div className="border-b border-white/10 bg-white/5">
          <div className="p-4">
            <h2 className="text-lg font-semibold text-white/90 mb-3">Tag System</h2>
            
            {/* Tab navigation */}
            <div className="flex space-x-1">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`
                    flex items-center px-3 py-2 rounded-md text-sm font-medium transition-colors
                    ${activeTab === tab.id
                      ? 'bg-purple-500/20 text-purple-300'
                      : 'text-white/60 hover:text-white/90 hover:bg-white/10'
                    }
                  `}
                >
                  <span className="mr-2">{tab.icon}</span>
                  {tab.label}
                  {tab.count !== undefined && (
                    <span className={`
                      ml-2 px-2 py-1 rounded-full text-xs
                      ${activeTab === tab.id
                        ? 'bg-purple-400/30 text-purple-200'
                        : 'bg-white/20 text-white/70'
                      }
                    `}>
                      {tab.count}
                    </span>
                  )}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Tab content */}
        <div className="flex-1 overflow-hidden">
          {activeTab === 'manage' && (
            <div className="h-full flex flex-col">
              <TagManager
                className="flex-1"
                showEffects={false} // We'll show effects in separate tab
                allowReordering={true}
                maxHeight="none"
              />
            </div>
          )}

          {activeTab === 'browse' && (
            <div className="h-full">
              <TagBrowser
                onTagAdd={handleTagAdd}
                selectedTags={activeTags}
                className="h-full"
                maxHeight="none"
              />
            </div>
          )}

          {activeTab === 'effects' && showEffectVisualizer && (
            <div className="h-full">
              <TagEffectVisualizer
                character={character}
                className="h-full"
                showAttributeChanges={true}
                showVisualModifiers={true}
                showGameplayFlags={false}
                showCombinations={true}
                realTimePreview={true}
              />
            </div>
          )}
        </div>

        {/* Footer with quick stats */}
        <div className="border-t border-white/10 bg-white/5 p-3">
          <div className="flex items-center justify-between text-xs text-white/60">
            <div>
              {activeTags.length} active tag{activeTags.length !== 1 ? 's' : ''}
            </div>
            {currentEffects && (
              <div>
                {currentEffects.resolvedEffects.length} effect{currentEffects.resolvedEffects.length !== 1 ? 's' : ''} active
              </div>
            )}
            {currentEffects && currentEffects.combinationBonuses.length > 0 && (
              <div className="text-purple-300">
                🎉 {currentEffects.combinationBonuses.length} combo{currentEffects.combinationBonuses.length !== 1 ? 's' : ''}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Tag Acquisition Modal/Inline */}
      {showAcquisitionFlow && (
        <TagAcquisition
          isOpen={acquisitionOpen}
          onClose={() => setAcquisitionOpen(false)}
          onAccept={handleAcquisitionAccept}
          onReject={handleAcquisitionReject}
          inline={layout === 'inline' || layout === 'sidebar'}
          className={layout === 'inline' || layout === 'sidebar' ? 'mt-4' : ''}
        />
      )}
    </>
  );
};

export default TagPanel;