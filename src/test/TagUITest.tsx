import React, { useEffect, useState } from 'react';
import { TagBrowser, TagManager, TagAcquisition, TagEffectVisualizer } from '../features/tags/components';
import TagPanel from '../features/tags/components/TagPanel';
import { useTagStore } from '../store/tagStore';
import useCharacterStore from '../store/characterStore';
import { CharacterAttributes } from '../types/character';

const TagUITest: React.FC = () => {
  const [currentTest, setCurrentTest] = useState<'browser' | 'manager' | 'acquisition' | 'effects' | 'panel'>('panel');
  const [acquisitionOpen, setAcquisitionOpen] = useState(false);
  
  const { loadTags, activeTags, addTag } = useTagStore();
  const { attributes, setCharacterAttribute } = useCharacterStore();

  // Initialize test data
  useEffect(() => {
    loadTags();
    
    // Set up test character
    setCharacterAttribute('name', 'Test Character');
    setCharacterAttribute('anatomy', 'Female');
    setCharacterAttribute('genderExpression', 75);
    setCharacterAttribute('ethnicity', 'Caucasian');
    setCharacterAttribute('hairColor', 'Brown');
    setCharacterAttribute('eyeColor', 'Blue');
    setCharacterAttribute('bodyType', 'Athletic');
  }, [loadTags, setCharacterAttribute]);

  const addTestTags = () => {
    addTag('feminine', 'test');
    addTag('athletic', 'test');
    addTag('confident', 'test');
  };

  const clearTags = () => {
    const tagStore = useTagStore.getState();
    tagStore.clearAllTags();
  };

  const testCharacter: CharacterAttributes = {
    name: 'Test Character',
    anatomy: 'Female',
    genderExpression: 75,
    ethnicity: 'Caucasian',
    hairColor: 'Brown',
    eyeColor: 'Blue',
    bodyType: 'Athletic'
  };

  return (
    <div className="min-h-screen bg-gray-100 p-8">
      <div className="max-w-6xl mx-auto">
        <h1 className="text-3xl font-bold text-gray-900 mb-6">Tag System UI Test</h1>
        
        {/* Test Controls */}
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <div className="flex flex-wrap gap-4 mb-4">
            <button
              onClick={() => setCurrentTest('panel')}
              className={`px-4 py-2 rounded ${currentTest === 'panel' ? 'bg-blue-500 text-white' : 'bg-gray-200'}`}
            >
              Tag Panel
            </button>
            <button
              onClick={() => setCurrentTest('browser')}
              className={`px-4 py-2 rounded ${currentTest === 'browser' ? 'bg-blue-500 text-white' : 'bg-gray-200'}`}
            >
              Tag Browser
            </button>
            <button
              onClick={() => setCurrentTest('manager')}
              className={`px-4 py-2 rounded ${currentTest === 'manager' ? 'bg-blue-500 text-white' : 'bg-gray-200'}`}
            >
              Tag Manager
            </button>
            <button
              onClick={() => setCurrentTest('effects')}
              className={`px-4 py-2 rounded ${currentTest === 'effects' ? 'bg-blue-500 text-white' : 'bg-gray-200'}`}
            >
              Effect Visualizer
            </button>
            <button
              onClick={() => setAcquisitionOpen(true)}
              className="px-4 py-2 rounded bg-purple-500 text-white"
            >
              Test Acquisition
            </button>
          </div>
          
          <div className="flex gap-4">
            <button
              onClick={addTestTags}
              className="px-4 py-2 bg-green-500 text-white rounded"
            >
              Add Test Tags
            </button>
            <button
              onClick={clearTags}
              className="px-4 py-2 bg-red-500 text-white rounded"
            >
              Clear All Tags
            </button>
          </div>
          
          <div className="mt-4 text-sm text-gray-600">
            Active Tags: {activeTags.length}
          </div>
        </div>

        {/* Test Component Display */}
        <div className="bg-white rounded-lg shadow">
          {currentTest === 'panel' && (
            <div className="h-[600px]">
              <TagPanel
                character={testCharacter}
                className="h-full"
                layout="modal"
                showEffectVisualizer={true}
                showAcquisitionFlow={true}
              />
            </div>
          )}

          {currentTest === 'browser' && (
            <div className="h-[600px] p-6">
              <TagBrowser
                onTagAdd={(tagId) => addTag(tagId, 'test')}
                selectedTags={activeTags}
                className="h-full"
              />
            </div>
          )}

          {currentTest === 'manager' && (
            <div className="h-[600px] p-6">
              <TagManager
                className="h-full"
                showEffects={true}
                allowReordering={true}
              />
            </div>
          )}

          {currentTest === 'effects' && (
            <div className="h-[600px] p-6">
              <TagEffectVisualizer
                character={testCharacter}
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

        {/* Tag Acquisition Modal */}
        <TagAcquisition
          isOpen={acquisitionOpen}
          onClose={() => setAcquisitionOpen(false)}
          onAccept={(tagId) => {
            console.log('Accepted tag:', tagId);
            setAcquisitionOpen(false);
          }}
          onReject={(tagId) => {
            console.log('Rejected tag:', tagId);
            setAcquisitionOpen(false);
          }}
        />
      </div>
    </div>
  );
};

export default TagUITest;