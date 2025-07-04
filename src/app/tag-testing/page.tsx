'use client';

import React, { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import TagPanel from '@/features/tags/components/TagPanel';
import { TagBrowser, TagManager, TagEffectVisualizer } from '@/features/tags/components';
import { useTagStore } from '@/store/tagStore';
import useCharacterStore from '@/store/characterStore';
import { CharacterAttributes } from '@/types/character';

const TagTestingPage: React.FC = () => {
  const router = useRouter();
  const [testMode, setTestMode] = useState<'full' | 'browser' | 'manager' | 'effects'>('full');
  
  const { 
    loadTags, 
    activeTags, 
    addTag, 
    clearAllTags,
    currentEffects,
    discoveredCombinations 
  } = useTagStore();
  
  const { 
    attributes, 
    setCharacterAttribute,
    addTag: addCharacterTag,
    applyTagEffects 
  } = useCharacterStore();

  // Initialize on mount
  useEffect(() => {
    loadTags();
    
    // Set up a test character if none exists
    if (!attributes.name) {
      setCharacterAttribute('name', 'Test Character');
      setCharacterAttribute('anatomy', 'Female');
      setCharacterAttribute('genderExpression', 75);
      setCharacterAttribute('ethnicity', 'Caucasian');
      setCharacterAttribute('hairColor', 'Brown');
      setCharacterAttribute('eyeColor', 'Blue');
      setCharacterAttribute('bodyType', 'Athletic');
    }
  }, [loadTags, attributes.name, setCharacterAttribute]);

  const handleAddTestTags = () => {
    const testTags = ['feminine', 'athletic', 'confident', 'long_hair'];
    testTags.forEach(tagId => {
      addTag(tagId, 'test');
      addCharacterTag(tagId, 'test');
    });
  };

  const handleClearTags = () => {
    clearAllTags();
  };

  const handleRandomAttribute = () => {
    const randomExpression = Math.floor(Math.random() * 101);
    setCharacterAttribute('genderExpression', randomExpression);
    
    const ethnicities = ['Caucasian', 'African', 'Asian', 'Hispanic', 'Middle Eastern'];
    const randomEthnicity = ethnicities[Math.floor(Math.random() * ethnicities.length)];
    setCharacterAttribute('ethnicity', randomEthnicity);
    
    // Apply effects after changing attributes
    setTimeout(() => applyTagEffects(), 100);
  };

  return (
    <div
      className="app-container flex h-screen relative overflow-hidden font-quicksand"
      style={{ background: 'linear-gradient(135deg, #2d1b4e 0%, #4a1843 50%, #1e3a5f 100%)' }}
    >
      <div
        className="absolute inset-0 opacity-50 animate-gradient-shift pointer-events-none"
        style={{ backgroundImage: 'radial-gradient(circle at 30% 50%, rgba(236, 72, 153, 0.05) 0%, transparent 50%)' }}
      />
      
      {/* Left Sidebar - Character Info */}
      <aside
        className="sidebar w-[320px] backdrop-blur-md border-r border-white/10 p-6 flex flex-col gap-6 z-10 overflow-y-auto main-content-scrollbar"
        style={{ backgroundColor: 'rgba(255, 255, 255, 0.04)' }}
      >
        {/* Character Display */}
        <div className="character-display-wrapper flex flex-col justify-start">
          <div className="character-display text-center">
            <div className="preview-container w-full flex items-center justify-center mb-3 relative" style={{ height: '225px' }}>
              <div className="character-portrait w-[225px] h-[225px] bg-black/30 rounded-full flex items-center justify-center border-2 border-pink-500/30 text-white/30 text-sm shadow-inner-pink">
                Test Character
              </div>
            </div>
            
            <h2 className="character-name text-2xl font-semibold mt-1 mb-2 text-purple-200">{attributes.name || "Test Character"}</h2>
            <p className="character-subtitle text-sm text-white/70">Tag Testing Environment</p>
          </div>
        </div>

        {/* Character Stats */}
        <div
          className="stats-section border border-white/10 rounded-xl p-5 flex-shrink-0"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="stats-title text-lg font-semibold mb-4 text-white/90">Character Info</h3>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-white/70">Anatomy:</span>
              <span className="text-white">{attributes.anatomy || 'None'}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-white/70">Expression:</span>
              <span className="text-white">{attributes.genderExpression}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-white/70">Ethnicity:</span>
              <span className="text-white">{attributes.ethnicity || 'None'}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-white/70">Body Type:</span>
              <span className="text-white">{attributes.bodyType || 'None'}</span>
            </div>
          </div>
        </div>

        {/* Tag Stats */}
        <div
          className="stats-section border border-white/10 rounded-xl p-5 flex-shrink-0"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="stats-title text-lg font-semibold mb-4 text-white/90">Tag Stats</h3>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div className="text-center">
              <div className="text-2xl font-bold text-pink-300">{activeTags.length}</div>
              <div className="text-white/70">Active</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-purple-300">{currentEffects?.resolvedEffects.length || 0}</div>
              <div className="text-white/70">Effects</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-blue-300">{discoveredCombinations.length}</div>
              <div className="text-white/70">Combos</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-green-300">{Object.keys(currentEffects?.attributeChanges || {}).length}</div>
              <div className="text-white/70">Mods</div>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main className="main-content flex-1 p-10 overflow-y-auto overflow-x-hidden flex flex-col items-center relative z-10 main-content-scrollbar">
        <div className="creation-container w-full max-w-6xl">
          {/* Header */}
          <div className="creation-header text-center mb-12">
            <h1 className="creation-title text-5xl font-bold bg-gradient-to-r from-pink-500 to-purple-500 text-transparent bg-clip-text mb-3"
                style={{ textShadow: '0 0 40px rgba(236, 72, 153, 0.3)', filter: 'drop-shadow(0 0 20px rgba(236, 72, 153, 0.2))'}}>
              Tag System Testing
            </h1>
            <p className="creation-subtitle text-lg text-white/60">
              Experiment with tags, effects, and combinations
            </p>
          </div>

          {/* Control Panel */}
          <div className="bg-black/20 backdrop-blur-xl rounded-3xl p-8 border border-white/10 mb-8">
            <h2 className="text-xl font-semibold text-white mb-6">Test Controls</h2>
            
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
              <button
                onClick={() => setTestMode('full')}
                className={`px-4 py-3 rounded-lg transition-all duration-200 font-medium ${
                  testMode === 'full' 
                    ? 'bg-gradient-to-r from-pink-500 to-purple-600 text-white shadow-pink-glow' 
                    : 'bg-white/10 text-white/80 hover:bg-white/20 border border-white/20'
                }`}
              >
                Full Tag Panel
              </button>
              <button
                onClick={() => setTestMode('browser')}
                className={`px-4 py-3 rounded-lg transition-all duration-200 font-medium ${
                  testMode === 'browser' 
                    ? 'bg-gradient-to-r from-pink-500 to-purple-600 text-white shadow-pink-glow' 
                    : 'bg-white/10 text-white/80 hover:bg-white/20 border border-white/20'
                }`}
              >
                Tag Browser
              </button>
              <button
                onClick={() => setTestMode('manager')}
                className={`px-4 py-3 rounded-lg transition-all duration-200 font-medium ${
                  testMode === 'manager' 
                    ? 'bg-gradient-to-r from-pink-500 to-purple-600 text-white shadow-pink-glow' 
                    : 'bg-white/10 text-white/80 hover:bg-white/20 border border-white/20'
                }`}
              >
                Tag Manager
              </button>
              <button
                onClick={() => setTestMode('effects')}
                className={`px-4 py-3 rounded-lg transition-all duration-200 font-medium ${
                  testMode === 'effects' 
                    ? 'bg-gradient-to-r from-pink-500 to-purple-600 text-white shadow-pink-glow' 
                    : 'bg-white/10 text-white/80 hover:bg-white/20 border border-white/20'
                }`}
              >
                Effect Visualizer
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <button
                onClick={handleAddTestTags}
                className="px-4 py-3 bg-gradient-to-r from-green-500 to-emerald-600 text-white rounded-lg hover:shadow-green-glow transition-all duration-200 font-medium"
              >
                Add Test Tags
              </button>
              <button
                onClick={handleClearTags}
                className="px-4 py-3 bg-gradient-to-r from-red-500 to-rose-600 text-white rounded-lg hover:shadow-red-glow transition-all duration-200 font-medium"
              >
                Clear All Tags
              </button>
              <button
                onClick={handleRandomAttribute}
                className="px-4 py-3 bg-gradient-to-r from-blue-500 to-indigo-600 text-white rounded-lg hover:shadow-blue-glow transition-all duration-200 font-medium"
              >
                Randomize Attributes
              </button>
            </div>
          </div>

          {/* Tag Interface */}
          <div className="bg-black/20 backdrop-blur-xl rounded-3xl border border-white/10 overflow-hidden min-h-[600px]">
            {testMode === 'full' && (
              <TagPanel
                character={attributes}
                className="h-[600px] bg-transparent border-0 shadow-none"
                layout="inline"
                showEffectVisualizer={true}
                showAcquisitionFlow={true}
              />
            )}

            {testMode === 'browser' && (
              <div className="h-[600px] p-6">
                <TagBrowser
                  onTagAdd={(tagId) => {
                    addTag(tagId, 'test');
                    addCharacterTag(tagId, 'test');
                  }}
                  selectedTags={activeTags}
                  className="h-full"
                />
              </div>
            )}

            {testMode === 'manager' && (
              <div className="h-[600px] p-6">
                <TagManager
                  className="h-full"
                  showEffects={true}
                  allowReordering={true}
                />
              </div>
            )}

            {testMode === 'effects' && (
              <div className="h-[600px] p-6">
                <TagEffectVisualizer
                  character={attributes}
                  className="h-full"
                  showAttributeChanges={true}
                  showVisualModifiers={true}
                  showGameplayFlags={true}
                  showCombinations={true}
                  realTimePreview={true}
                />
              </div>
            )}
          </div>
        </div>
      </main>

      {/* Right Sidebar - Navigation & Tips */}
      <aside
        className="sidebar w-[320px] backdrop-blur-md border-l border-white/10 p-6 flex flex-col gap-6 z-10"
        style={{ backgroundColor: 'rgba(255, 255, 255, 0.04)' }}
      >
        {/* Navigation */}
        <div
          className="nav-section border border-white/10 rounded-xl p-5"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="stats-title text-lg font-semibold mb-4 text-white/90">Navigation</h3>
          <div className="flex flex-col gap-3">
            <button
              onClick={() => router.push('/character-creation')}
              className="px-4 py-2 bg-white/10 text-white rounded-lg hover:bg-white/20 transition-colors text-left"
            >
              ← Character Creation
            </button>
            <button
              onClick={() => router.push('/title')}
              className="px-4 py-2 bg-white/10 text-white rounded-lg hover:bg-white/20 transition-colors text-left"
            >
              ← Title Screen
            </button>
          </div>
        </div>

        {/* Tips */}
        <div
          className="context-panel border border-white/10 rounded-xl p-5"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="context-title text-lg font-semibold mb-4 text-white/90">Testing Tips</h3>
          <p className="help-text text-sm leading-relaxed text-white/60 mb-4">
            {testMode === 'full' && "Test the complete tag panel experience with all features enabled."}
            {testMode === 'browser' && "Browse and add tags to see how they affect your character."}
            {testMode === 'manager' && "Manage active tags, view conflicts, and reorder priorities."}
            {testMode === 'effects' && "Visualize how tag effects modify character attributes in real-time."}
          </p>
          <div className="tip p-3 bg-pink-500/10 border-l-2 border-pink-500 rounded-md text-sm text-white/80">
            <strong>Tip:</strong> Use the test controls to quickly add/remove tags and see immediate effects.
          </div>
        </div>
      </aside>

      {/* Add global styles */}
      <style jsx global>{`
        .shadow-pink-glow { box-shadow: 0 8px 24px rgba(236, 72, 153, 0.3); }
        .shadow-green-glow { box-shadow: 0 8px 24px rgba(16, 185, 129, 0.3); }
        .shadow-red-glow { box-shadow: 0 8px 24px rgba(239, 68, 68, 0.3); }
        .shadow-blue-glow { box-shadow: 0 8px 24px rgba(59, 130, 246, 0.3); }
        .shadow-inner-pink { box-shadow: inset 0 0 20px rgba(236, 72, 153, 0.1), 0 0 15px rgba(236, 72, 153, 0.1); }
        .main-content-scrollbar::-webkit-scrollbar { width: 8px; }
        .main-content-scrollbar::-webkit-scrollbar-track { background: rgba(255, 255, 255, 0.05); border-radius: 4px; }
        .main-content-scrollbar::-webkit-scrollbar-thumb { background: rgba(236, 72, 153, 0.3); border-radius: 4px; transition: background 0.2s ease; }
        .main-content-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(236, 72, 153, 0.5); }
        .main-content-scrollbar { scrollbar-width: thin; scrollbar-color: rgba(236, 72, 153, 0.3) rgba(255, 255, 255, 0.05); }
      `}</style>
    </div>
  );
};

export default TagTestingPage;