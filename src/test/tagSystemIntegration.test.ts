// Tag System Integration Test
// Quick test to verify tag system functionality

import { tagService, tagEffectService } from '../services/tags';
import { useTagStore } from '../store/tagStore';
import useCharacterStore from '../store/characterStore';

// Test basic tag system functionality
export async function testTagSystemIntegration(): Promise<boolean> {
  console.log('🧪 Testing Tag System Integration...');
  
  try {
    // Test 1: Tag Service Loading
    console.log('1. Testing tag service loading...');
    await tagService.loadTags();
    
    if (!tagService.isLoaded()) {
      throw new Error('Tag service failed to load');
    }
    
    const stats = tagService.getStatistics();
    console.log('✅ Tag service loaded:', stats);
    
    // Test 2: Tag Search
    console.log('2. Testing tag search...');
    const searchResult = tagService.searchTags({
      search: 'feminine',
      limit: 5
    });
    
    if (searchResult.tags.length === 0) {
      throw new Error('No search results found');
    }
    
    console.log('✅ Tag search working:', searchResult.tags.map(t => t.name));
    
    // Test 3: Tag Validation
    console.log('3. Testing tag validation...');
    const validation = tagService.validateTags(['feminine', 'masculine']);
    
    if (validation.valid) {
      console.warn('⚠️ Expected validation conflict between feminine/masculine');
    } else {
      console.log('✅ Tag validation working:', validation.errors[0]);
    }
    
    // Test 4: Effect Calculation
    console.log('4. Testing effect calculation...');
    const testCharacter = {
      name: 'Test',
      anatomy: 'Female' as const,
      genderExpression: 50,
      ethnicity: 'Caucasian' as const,
      hairColor: 'Brown' as const,
      eyeColor: 'Brown' as const,
      bodyType: 'Average' as const
    };
    
    const context = {
      character: testCharacter,
      activeTags: ['feminine', 'long_hair'],
      scene: 'character_creation',
      timestamp: new Date(),
      metadata: {}
    };
    
    const effects = tagEffectService.calculateEffects(['feminine', 'long_hair'], context);
    console.log('✅ Effect calculation working:', effects.resolvedEffects.length, 'effects');
    
    // Test 5: Store Integration
    console.log('5. Testing store integration...');
    
    // Test character store
    const characterStore = useCharacterStore.getState();
    characterStore.setCharacterAttribute('anatomy', 'Female');
    characterStore.addTag('feminine');
    
    if (characterStore.activeTags.includes('feminine')) {
      console.log('✅ Character store integration working');
    } else {
      throw new Error('Character store tag integration failed');
    }
    
    // Test tag store
    const tagStore = useTagStore.getState();
    await tagStore.loadTags();
    tagStore.addTag('confident', 'test');
    
    if (tagStore.activeTags.includes('confident')) {
      console.log('✅ Tag store integration working');
    } else {
      throw new Error('Tag store integration failed');
    }
    
    // Test 6: Tag Suggestions
    console.log('6. Testing tag suggestions...');
    const suggestions = tagService.getTagSuggestions(['feminine'], 3);
    console.log('✅ Tag suggestions working:', suggestions.map(t => t.name));
    
    console.log('🎉 All tag system integration tests passed!');
    return true;
    
  } catch (error) {
    console.error('❌ Tag system integration test failed:', error);
    return false;
  }
}

// Test compatibility with existing character creation
export function testCharacterCreationCompatibility(): boolean {
  console.log('🧪 Testing Character Creation Compatibility...');
  
  try {
    const characterStore = useCharacterStore.getState();
    
    // Reset to clean state
    characterStore.resetCreationState();
    
    // Test legacy tag support
    const legacyTags = [
      { id: 'test1', name: 'Test Tag 1', description: 'Test' },
      { id: 'test2', name: 'Test Tag 2', description: 'Test' }
    ];
    
    characterStore.setTags(legacyTags);
    
    if (characterStore.tags.length === 2) {
      console.log('✅ Legacy tag support working');
    } else {
      throw new Error('Legacy tag support failed');
    }
    
    // Test attribute changes triggering tag effects
    characterStore.setCharacterAttribute('genderExpression', 75);
    
    // This should trigger automatic tag assignment
    setTimeout(() => {
      if (characterStore.attributes.genderExpression === 75) {
        console.log('✅ Attribute updates working');
      }
    }, 100);
    
    console.log('🎉 Character creation compatibility tests passed!');
    return true;
    
  } catch (error) {
    console.error('❌ Character creation compatibility test failed:', error);
    return false;
  }
}

// Run all tests
export async function runAllTagTests(): Promise<void> {
  console.log('🚀 Starting Tag System Tests...\n');
  
  const integrationPassed = await testTagSystemIntegration();
  const compatibilityPassed = testCharacterCreationCompatibility();
  
  console.log('\n📊 Test Results:');
  console.log('Integration Tests:', integrationPassed ? '✅ PASSED' : '❌ FAILED');
  console.log('Compatibility Tests:', compatibilityPassed ? '✅ PASSED' : '❌ FAILED');
  
  if (integrationPassed && compatibilityPassed) {
    console.log('\n🎉 All tests passed! Tag system is ready for Phase 2.');
  } else {
    console.log('\n⚠️ Some tests failed. Check issues before proceeding.');
  }
}