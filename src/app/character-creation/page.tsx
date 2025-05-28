'use client';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation'; // For navigation
import AttributeInputForm from '@/features/character-creation/components/AttributeInputForm';
import GenerationGallery from '@/features/character-creation/components/GenerationGallery';
import AstralMirrorDisplay from '@/features/character-creation/components/AstralMirrorDisplay'; // Import AstralMirrorDisplay
import StyledToggle from '@/components/ui/StyledToggle';
import useCharacterStore from '@/store/characterStore';
import { ImageOption, CharacterAttributes } from '@/types/character'; // Added CharacterAttributes for clarity if needed, though store provides it
import { initiateImageGeneration, uploadImageToComfyUI } from '@/services/comfyuiService';
import { saveCharacterImage, generateUUID } from '@/services/imageSaverService'; // Added for finalization

// Placeholder for particle generation logic (can be moved to a separate component)
const Particles: React.FC = () => {
  const [particles, setParticles] = useState<Array<{ id: number; style: React.CSSProperties }>>([]);

  useEffect(() => {
    const generateParticles = () => {
      const newParticles = [];
      const particleCount = 15; // From mockup
      for (let i = 0; i < particleCount; i++) {
        const duration = 15 + Math.random() * 10;
        const delay = Math.random() * 15;
        const size = Math.random() > 0.66 ? 5 : (Math.random() > 0.33 ? 3 : 4);
        const color = Math.random() > 0.66 
          ? 'rgba(59, 130, 246, 0.4)' // blue-500
          : (Math.random() > 0.33 
            ? 'rgba(168, 85, 247, 0.4)' // purple-500
            : 'rgba(236, 72, 153, 0.4)'); // pink-500

        newParticles.push({
          id: i,
          style: {
            position: 'absolute',
            width: `${size}px`,
            height: `${size}px`,
            background: color,
            borderRadius: '50%',
            filter: 'blur(0.5px)',
            pointerEvents: 'none',
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 100}%`,
            animation: `float-particle ${duration}s infinite ease-in-out`,
            animationDelay: `${delay}s`,
          } as React.CSSProperties,
        });
      }
      setParticles(newParticles);
    };
    generateParticles();
  }, []);

  return (
    <>
      {particles.map(p => <div key={p.id} style={p.style} />)}
    </>
  );
};


const CharacterCreationPage: React.FC = () => {
  const router = useRouter();
  const {
    attributes,
    creationStep,
    setCreationStep,
    faceOptions,
    setFaceOptions,
    selectedFace,
    setSelectedFace,
    isGeneratingFace,
    setIsGeneratingFace,
    fullBodyOptions,
    setFullBodyOptions,
    selectedFullBody,
    setSelectedFullBody,
    isGeneratingFullBody,
    setIsGeneratingFullBody,
    resetCreationState,
    characterImageUrl,
    setCharacterImageUrl,
    // V2 state
    generationProgress,
    error: characterError, // Get error state from store
    setError: setCharacterError, // Action to clear error if needed
    setFinalizedCharacter // Action for finalization
  } = useCharacterStore();

  const [activePreview, setActivePreview] = useState<'face' | 'body'>('face');
  const [currentWebsocket, setCurrentWebsocket] = useState<{ close: () => void } | null>(null);
  const [uploadedFaceDetails, setUploadedFaceDetails] = useState<{ filename: string; subfolder?: string } | null>(null);
  const [isFinalizing, setIsFinalizing] = useState(false); // State for finalization loading


  useEffect(() => {
    // Cleanup WebSocket connection if the component unmounts or step changes significantly
    // This effect now only runs on mount and unmount
    return () => {
      if (currentWebsocket) {
        console.log("[CharacterCreationPage] Closing WebSocket on component unmount.");
        currentWebsocket.close();
        setCurrentWebsocket(null); // Clear the state
      }
    };
  }, []); // Empty dependency array: runs only on mount and unmount

  // Diagnostic useEffect to log state changes
  useEffect(() => {
    console.log('[CharacterCreationPage] isGeneratingFace changed:', isGeneratingFace);
    console.log('[CharacterCreationPage] faceOptions:', faceOptions);
    console.log('[CharacterCreationPage] selectedFace:', selectedFace);
    if (!isGeneratingFace && faceOptions.length > 0 && !selectedFace) {
      // Automatically select the first face if options are available and none is selected
      // This might help display something immediately in the mirror.
      // setSelectedFace(faceOptions[0]); // User might want to explicitly select
      console.log('[CharacterCreationPage] Face generation finished, options available.');
    }
  }, [isGeneratingFace, faceOptions, selectedFace]);

  const handleAttributeSubmit = async () => {
    if (currentWebsocket) currentWebsocket.close(); // Close previous socket if any
    // setCreationStep('faceSelection'); // Keep on 'attributes' step
    setSelectedFace(null); // Reset selected face if re-generating
    // setFaceOptions([]); // Removed to allow accumulation of face options
    const result = await initiateImageGeneration(attributes, [], 'face');
    if (result) {
      setCurrentWebsocket({ close: result.closeSocket });
    } else {
      console.error("Face generation initiation failed.");
      // Error is set in store by initiateImageGeneration
    }
  };

  const handleFaceSelect = (image: ImageOption) => {
    setSelectedFace(image);
    // setActivePreview('body'); // Optionally switch preview
  };
  
  const confirmFaceSelectionAndGenerateBody = async () => {
    if (!selectedFace || !selectedFace.url) return;
    
    if (currentWebsocket) currentWebsocket.close(); // Close previous socket

    setIsGeneratingFullBody(true); // Set loading state for the whole process

    // Step 1: Convert selectedFace.url to a File object
    let faceFile: File | null = null;
    try {
      const response = await fetch(selectedFace.url); // Fetch the image data from its URL
      if (!response.ok) throw new Error(`Failed to fetch image: ${response.statusText}`);
      const blob = await response.blob();
      faceFile = new File([blob], "selected_face.png", { type: blob.type || 'image/png' });
    } catch (error) {
      console.error("Error fetching selected face image to create a file:", error);
      setCharacterError(`Error processing face image: ${error instanceof Error ? error.message : String(error)}`);
      setIsGeneratingFullBody(false); // Reset loading state
      return;
    }

    // Step 2: Upload the File to ComfyUI
    // Directly use the result of uploadImageToComfyUI for the initial generation
    // and also set it to state for regeneration.
    if (faceFile) {
      const uploadResult = await uploadImageToComfyUI(faceFile, true, "character_faces_input"); // Use a distinct subfolder, overwrite true
      if (uploadResult && uploadResult.filename) {
        setUploadedFaceDetails({ filename: uploadResult.filename, subfolder: uploadResult.subfolder }); // Store details in state for regeneration

        setCreationStep('bodySelection');
        const result = await initiateImageGeneration(
          attributes,
          [],
          'fullbody',
          uploadResult.filename, // Use directly from uploadResult here
          uploadResult.subfolder
        );
        if (result) {
          setCurrentWebsocket({ close: result.closeSocket });
        } else {
          console.error("Full body generation initiation failed.");
          // Error and isGeneratingFullBody=false are handled by initiateImageGeneration
        }
      } else {
        console.error("Failed to upload face image to ComfyUI.");
        // Error is already set in the store by uploadImageToComfyUI
        setIsGeneratingFullBody(false); // Reset loading state
        return;
      }
    } else {
      // This case should ideally not be reached if the fetch was successful
      setCharacterError("Could not create face image file for upload.");
      setIsGeneratingFullBody(false); // Reset loading state
      return;
    }
    // Moved initiateImageGeneration call inside the successful upload block
    // to ensure uploadResult is in scope.
  };

  const handleRegenerateFullBody = async () => {
    if (!uploadedFaceDetails?.filename) {
      setCharacterError("No face image details available to regenerate full body. Please select a face first.");
      console.error("Attempted to regenerate full body without uploadedFaceDetails.");
      return;
    }
    if (currentWebsocket) currentWebsocket.close(); // Close previous socket

    // No need to re-upload, use stored details
    // The check at the beginning of the function ensures uploadedFaceDetails is not null here.
    const result = await initiateImageGeneration(
      attributes,
      [],
      'fullbody',
      uploadedFaceDetails!.filename, // Non-null assertion as it's checked above
      uploadedFaceDetails!.subfolder // Non-null assertion
    );
    if (result) {
      setCurrentWebsocket({ close: result.closeSocket });
    } else {
      console.error("Full body regeneration initiation failed.");
      // Error and isGeneratingFullBody=false are handled by initiateImageGeneration
    }
  };

  const handleBodySelect = (image: ImageOption) => {
    setSelectedFullBody(image);
    setCharacterImageUrl(image.url); // Set final character image for preview
    setActivePreview('body');
  };

  const handleFinalizeCharacter = async () => {
    if (!selectedFace || !selectedFace.url || typeof selectedFace.seed === 'undefined' ||
        !selectedFullBody || !selectedFullBody.url || typeof selectedFullBody.seed === 'undefined') {
      setCharacterError("Both face and full body images (with their seeds) must be selected to finalize.");
      return;
    }

    setIsFinalizing(true);
    setCharacterError(null); // Clear previous errors

    try {
      const newCharacterId = generateUUID();

      // Save face image
      const savedFace = await saveCharacterImage({
        characterId: newCharacterId,
        imageOption: selectedFace,
        imageType: 'face',
      });

      // Save full body image
      const savedBody = await saveCharacterImage({
        characterId: newCharacterId,
        imageOption: selectedFullBody,
        imageType: 'body',
      });

      // Update character store
      setFinalizedCharacter({
        characterId: newCharacterId,
        attributes: attributes, // Current attributes from store
        savedFaceImagePath: savedFace.relative,
        savedFullBodyImagePath: savedBody.relative,
        faceSeed: selectedFace.seed,
        bodySeed: selectedFullBody.seed,
      });
      
      console.log("Character finalized and saved:", {
        characterId: newCharacterId,
        attributes,
        facePath: savedFace.relative,
        bodyPath: savedBody.relative
      });

      // router.push('/game'); // Navigate to the next scene - Temporarily commented out for testing
      console.log("[CharacterCreationPage] Navigation to /game commented out for testing finalization state.");

    } catch (error) {
      console.error("Error finalizing character:", error);
      setCharacterError(`Finalization failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsFinalizing(false);
    }
  };

  const handleBack = () => {
    if (creationStep === 'bodySelection') {
      setCreationStep('attributes'); // Go back to attributes from body selection
      setSelectedFullBody(null);
      setFullBodyOptions([]);
      setActivePreview('face');
    } else if (creationStep === 'attributes') {
      // If on attributes step, reset face generation state or go to title
      if (faceOptions.length > 0 || selectedFace) {
        setFaceOptions([]);
        setSelectedFace(null);
        setIsGeneratingFace(false);
        if (currentWebsocket) {
          currentWebsocket.close();
          setCurrentWebsocket(null);
        }
      } else {
        router.push('/title');
      }
    } else if (creationStep === 'finalized') {
      setCreationStep('bodySelection'); // Go back to body selection from finalized
    } else { // Default for safety, or if other steps are added
      router.push('/title');
    }
  };
  
  const getPageTitle = () => {
    switch (creationStep) {
      case 'attributes': return "Create Your Character";
      // case 'faceSelection': return "Select Your Face"; // Removed
      case 'bodySelection': return "Select Your Form";
      case 'finalized': return "Character Complete!";
      default: return "Character Creation";
    }
  };

  const getPageSubtitle = () => {
    switch (creationStep) {
      case 'attributes': return "Define your appearance and generate your face"; // Updated subtitle
      // case 'faceSelection': return "Choose the face that best represents you"; // Removed
      case 'bodySelection': return "Select your full body form"; // Updated subtitle
      case 'finalized': return "Your adventure awaits!";
      default: return "";
    }
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
      
      <aside
        className="sidebar w-[320px] backdrop-blur-md border-r border-white/10 p-6 flex flex-col gap-6 z-10 overflow-y-auto main-content-scrollbar"
        style={{ backgroundColor: 'rgba(255, 255, 255, 0.04)' }}
      >
        <div className="character-display-wrapper flex flex-col justify-start">
          <div className="character-display text-center">
            <div
              className={`preview-container w-full flex items-center justify-center mb-3 relative transition-all duration-300 ease-in-out`}
              style={{ height: activePreview === 'face' ? '225px' : '350px' }}
            >
            <div className={`character-portrait w-[225px] h-[225px] bg-black/30 rounded-full flex items-center justify-center border-2 border-pink-500/30 text-white/30 text-sm shadow-inner-pink transition-all duration-300 ease-in-out ${activePreview === 'face' ? 'opacity-100 scale-100' : 'opacity-0 scale-90 absolute'}`}>
              {selectedFace ? <img src={selectedFace.url} alt="Selected Face" className="w-full h-full object-cover rounded-full"/> : 'Face Preview'}
            </div>
            <div className={`character-fullbody w-[200px] h-[350px] bg-black/30 rounded-xl flex items-center justify-center border-2 border-pink-500/30 text-white/30 text-sm shadow-inner-pink transition-all duration-300 ease-in-out ${activePreview === 'body' ? 'opacity-100 scale-100 relative' : 'opacity-0 scale-90 absolute'}`}>
              {selectedFullBody ? <img src={selectedFullBody.url} alt="Selected Full Body" className="w-full h-full object-cover rounded-xl"/> : (characterImageUrl ? <img src={characterImageUrl} alt="Character Preview" className="w-full h-full object-cover rounded-xl"/> : 'Full Body')}
            </div>
          </div>
          
          <StyledToggle
            isActive={activePreview === 'face'}
            onToggle={(isNowActive) => {
              setActivePreview(isNowActive ? 'face' : 'body');
            }}
          />
          
          <h2 className="character-name text-2xl font-semibold mt-1 mb-2 text-purple-200">{attributes.name || "Your Character"}</h2>
          <p className="character-subtitle text-sm text-white/70">Level 1 Adventurer</p>
        </div>
        </div>

        <div
          className="stats-section border border-white/10 rounded-xl p-5 flex-shrink-0"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="stats-title text-lg font-semibold mb-4 text-white/90">Stats</h3>
          {/* Stats items can be dynamic later */}
        </div>

        <div
          className="tags-section border border-white/10 rounded-xl p-5 flex-shrink-0"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="stats-title text-lg font-semibold mb-4 text-white/90">Tags</h3>
          {/* Tags can be dynamic later */}
        </div>
      </aside>

      <main className="main-content flex-1 p-10 overflow-y-auto overflow-x-hidden flex flex-col items-center relative z-10 main-content-scrollbar">
        <Particles />
        <div className="creation-container w-full max-w-4xl">
          <div className="creation-header text-center mb-12">
            <h1 className="creation-title text-5xl font-bold bg-gradient-to-r from-pink-500 to-purple-500 text-transparent bg-clip-text mb-3"
                style={{ textShadow: '0 0 40px rgba(236, 72, 153, 0.3)', filter: 'drop-shadow(0 0 20px rgba(236, 72, 153, 0.2))'}}
            >
              {getPageTitle()}
            </h1>
            <p className="creation-subtitle text-lg text-white/60">
              {getPageSubtitle()}
            </p>
            <div className="progress-indicator flex justify-center items-center gap-3 mt-6">
              {/* Progress indicator updated for 3 steps: Attributes (incl. face), Body, Finalized */}
              <div className={`progress-step h-2 rounded-full transition-all duration-300 ease-in-out ${creationStep === 'attributes' || creationStep === 'bodySelection' || creationStep === 'finalized' ? 'w-8 bg-gradient-to-r from-pink-500 to-purple-500' : 'w-2 bg-white/20'}`}></div>
              <div className={`progress-step h-2 rounded-full transition-all duration-300 ease-in-out ${creationStep === 'bodySelection' || creationStep === 'finalized' ? 'w-8 bg-gradient-to-r from-pink-500 to-purple-500' : 'w-2 bg-white/20'}`}></div>
              <div className={`progress-step h-2 rounded-full transition-all duration-300 ease-in-out ${creationStep === 'finalized' ? 'w-8 bg-gradient-to-r from-pink-500 to-purple-500' : 'w-2 bg-white/20'}`}></div>
            </div>
          </div>

          {creationStep === 'attributes' && (
            <div
              className="creation-content grid md:grid-cols-2 gap-12 backdrop-blur-xl rounded-3xl p-12 border border-white/10"
              style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)' }}
            >
              <div className="attribute-form flex flex-col gap-6">
                <AttributeInputForm />
              </div>
              <div className="mirror-container w-full max-w-md aspect-square flex flex-col items-center justify-center gap-6"> {/* Added sizing and aspect ratio */}
                 <AstralMirrorDisplay
                   isGenerating={isGeneratingFace}
                   progress={generationProgress}
                   previewImageUrl={selectedFace?.url || null}
                   currentStepName="Attributes" // This will show "Generating Face..." internally if isGeneratingFace is true
                 />
                 <button
                     onClick={handleAttributeSubmit}
                     disabled={isGeneratingFace}
                     className={`mt-6 nav-button primary py-3 px-8 rounded-lg text-white text-base font-semibold transition-all duration-200 ease-in-out hover:translate-y-[-2px]
                                 ${isGeneratingFace
                                   ? 'bg-gray-600/30 border border-gray-500/50 text-gray-400/70 cursor-not-allowed opacity-50'
                                   : 'bg-gradient-to-r from-pink-500 to-purple-600 hover:from-pink-600 hover:to-purple-700 hover:shadow-pink-glow border-none'}`}
                 >
                     {isGeneratingFace ? "Generating Face..." : "Generate Face"}
                 </button>

                 {(isGeneratingFace || faceOptions.length > 0) && (
                   <GenerationGallery
                     images={faceOptions}
                     isLoading={isGeneratingFace}
                     onSelectImage={handleFaceSelect}
                     galleryTitle="Select Your Face"
                     selectedImageId={selectedFace?.id}
                     itemType="face"
                   />
                 )}
                 
                 {selectedFace && !isGeneratingFace && (
                    <button
                       onClick={confirmFaceSelectionAndGenerateBody}
                       disabled={isGeneratingFullBody}
                       className={`mt-8 nav-button primary py-3 px-8 rounded-lg text-white text-base font-semibold transition-all duration-200 ease-in-out hover:translate-y-[-2px]
                                   ${isGeneratingFullBody
                                     ? 'bg-gray-600/30 border border-gray-500/50 text-gray-400/70 cursor-not-allowed opacity-50'
                                     : 'bg-gradient-to-r from-teal-500 to-green-500 hover:shadow-green-glow border-none'}`}
                     >
                       {isGeneratingFullBody ? "Preparing Body..." : "Confirm Face & Generate Body"}
                     </button>
                 )}
             </div>
           </div>
          )}

          {/* Removed faceSelection step block */}

          {creationStep === 'bodySelection' && (
            <div className="flex flex-col items-center gap-4 w-full"> {/* Removed max-h and overflow from this container */}
              <div className="w-full max-w-md aspect-[776/1344] flex justify-center"> {/* Removed max-h-[65vh] */}
                <AstralMirrorDisplay
                  isGenerating={isGeneratingFullBody}
                progress={generationProgress}
                previewImageUrl={selectedFullBody?.url || (fullBodyOptions.length > 0 ? fullBodyOptions[0].url : null)}
                currentStepName="Full Body"
                />
              </div>
              <GenerationGallery
                images={fullBodyOptions}
                isLoading={isGeneratingFullBody}
                onSelectImage={handleBodySelect}
                galleryTitle="Select Your Full Body Form"
                selectedImageId={selectedFullBody?.id}
                itemType="body"
              />
              {fullBodyOptions.length > 0 && !isGeneratingFullBody && (
                <button
                  onClick={handleRegenerateFullBody}
                  disabled={isGeneratingFullBody}
                  className={`mt-4 nav-button secondary py-3 px-8 rounded-lg text-white text-base font-semibold transition-all duration-200 ease-in-out hover:translate-y-[-2px]
                              ${isGeneratingFullBody
                                ? 'bg-gray-600/30 border border-gray-500/50 text-gray-400/70 cursor-not-allowed opacity-50'
                                : 'bg-gradient-to-r from-blue-500 to-indigo-600 hover:from-blue-600 hover:to-indigo-700 hover:shadow-indigo-glow border-none'}`}
                >
                  Generate Another Full Body
                </button>
              )}
            </div>
          )}
          
          {creationStep === 'finalized' && (
            <div className="text-center p-8 bg-gray-800 bg-opacity-70 rounded-lg shadow-xl">
              <h2 className="text-3xl font-bold text-pink-400 mb-4">Character Creation Complete!</h2>
              <p className="text-lg text-gray-300 mb-6">Your unique adventurer is ready.</p>
              {selectedFullBody && (
                <img src={selectedFullBody.url} alt="Final Character" className="w-64 h-auto mx-auto rounded-lg shadow-lg mb-6" />
              )}
              <button
                onClick={() => router.push('/game')} // Navigate to game start
                className="px-8 py-3 bg-green-500 hover:bg-green-600 text-white font-semibold rounded-lg shadow-md transition-transform duration-150 ease-in-out hover:scale-105"
              >
                Begin Your Adventure!
              </button>
            </div>
          )}


          {characterError && (
            <div className="mt-4 p-4 bg-red-500/20 border border-red-700 text-red-200 rounded-lg text-center">
              <p><strong>Error:</strong> {characterError}</p>
              <button onClick={() => setCharacterError(null)} className="mt-2 text-sm underline hover:text-red-100">Dismiss</button>
            </div>
          )}

          <nav className="navigation flex justify-between mt-12 pt-6 border-t border-white/10 w-full">
            <button
              onClick={handleBack}
              disabled={isGeneratingFace || isGeneratingFullBody} // Disable back while generating
              className={`nav-button py-3 px-8 bg-white/5 border border-white/20 rounded-lg text-white/90 text-base transition-all duration-200 ease-in-out hover:bg-white/10 hover:border-white/30 hover:translate-y-[-1px] ${(isGeneratingFace || isGeneratingFullBody || creationStep === 'attributes') ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              {creationStep === 'attributes' ? "Back to Title / Clear Face" : "Back to Attributes"}
            </button>
            
            {/* Removed buttons for 'attributes' and 'faceSelection' steps from here, they are handled within the 'attributes' block now */}

            {creationStep === 'bodySelection' && (
              <button
                onClick={handleFinalizeCharacter}
                disabled={!selectedFace || !selectedFullBody || isGeneratingFace || isGeneratingFullBody || isFinalizing}
                className={`nav-button primary py-3 px-8 rounded-lg text-white text-base font-semibold transition-all duration-200 ease-in-out hover:translate-y-[-2px]
                            ${(!selectedFace || !selectedFullBody || isGeneratingFace || isGeneratingFullBody || isFinalizing)
                              ? 'bg-gray-600/30 border border-gray-500/50 text-gray-400/70 cursor-not-allowed opacity-50'
                              : 'bg-gradient-to-r from-teal-500 to-green-500 hover:shadow-green-glow border-none'}`}
              >
                {isFinalizing ? "Finalizing..." : "Finish Character"}
              </button>
            )}
          </nav>
        </div>
      </main>

      <aside
        className="sidebar w-[320px] backdrop-blur-md border-l border-white/10 p-6 flex flex-col gap-6 z-10"
        style={{ backgroundColor: 'rgba(255, 255, 255, 0.04)' }}
      >
        <div className="settings-bar flex gap-2 justify-end mb-4">
          {/* Icons */}
        </div>
        
        <div
          className="context-panel border border-white/10 rounded-xl p-5"
          style={{ backgroundColor: 'rgba(255, 255, 255, 0.03)' }}
        >
          <h3 className="context-title text-lg font-semibold mb-4 text-white/90">Character Creation Tips</h3>
          <p className="help-text text-sm leading-relaxed text-white/60 mb-4">
            {creationStep === 'attributes' && "Define your core traits. These will influence the initial face generations. Select a face, then confirm to proceed to body generation."}
            {/* {creationStep === 'faceSelection' && "Choose a face that resonates with you. This will be the foundation for your full character."} // Removed */}
            {creationStep === 'bodySelection' && "Select the full body form that best matches your chosen face and desired physique."}
            {creationStep === 'finalized' && "Your character is ready! Prepare for an exciting adventure."}
          </p>
          <div className="tip p-3 bg-pink-500/10 border-l-2 border-pink-500 rounded-md text-sm text-white/80">
            <strong>Tip:</strong> You can regenerate images multiple times if you're not satisfied with the initial options.
          </div>
        </div>
      </aside>

      <style jsx global>{`
        @keyframes gradient-shift { /* ... */ }
        @keyframes float-particle { /* ... */ }
        .animate-gradient-shift { animation: gradient-shift 20s ease-in-out infinite; }
        .shadow-inner-pink { box-shadow: inset 0 0 20px rgba(236, 72, 153, 0.1), 0 0 15px rgba(236, 72, 153, 0.1); }
        .shadow-inner-pink-lg { box-shadow: inset 0 0 30px rgba(236, 72, 153, 0.1), 0 0 20px rgba(236, 72, 153, 0.1); }
        .hover\\:shadow-pink-glow:hover { box-shadow: 0 8px 24px rgba(236, 72, 153, 0.3); }
        .hover\\:shadow-green-glow:hover { box-shadow: 0 8px 24px rgba(16, 185, 129, 0.3); }
        .hover\\:shadow-indigo-glow:hover { box-shadow: 0 8px 24px rgba(99, 102, 241, 0.3); } // Added for new button
        .hover\\:shadow-pink-glow-sm:hover { box-shadow: 0 4px 12px rgba(236, 72, 153, 0.2); }
        .main-content-scrollbar::-webkit-scrollbar { width: 8px; }
        .main-content-scrollbar::-webkit-scrollbar-track { background: rgba(255, 255, 255, 0.05); border-radius: 4px; }
        .main-content-scrollbar::-webkit-scrollbar-thumb { background: rgba(236, 72, 153, 0.3); border-radius: 4px; transition: background 0.2s ease; }
        .main-content-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(236, 72, 153, 0.5); }
        .main-content-scrollbar { scrollbar-width: thin; scrollbar-color: rgba(236, 72, 153, 0.3) rgba(255, 255, 255, 0.05); }
      `}</style>

    </div>
  );
};

export default CharacterCreationPage;
