"use client";

 import { Geist, Geist_Mono } from "next/font/google";
 import "./globals.css";
import TagContextWrapper from "@/components/TagContextWrapper";
import { useState, useEffect } from "react";
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'; // Import the fetch function directly
import { generateComfyUIPromptData } from "./utils";
import CharacterSidebarPortrait from "@/features/character-display/components/CharacterSidebarPortrait"; // Import the component
import useCharacterStore from "@/store/characterStore"; // Import the store

import ComfyUIStatusIndicator from "@/components/comfyui-status/ComfyUIStatusIndicator";
import InstallationProgress from "@/components/comfyui-status/InstallationProgress";
import ErrorMessageDisplay from "@/components/comfyui-status/ErrorMessageDisplay";

// Remove destructuring and log

 const geist = Geist({ subsets: ['latin'] });
 const geist_mono = Geist_Mono({ subsets: ['latin'] });

 export default function RootLayout({
  children,
 }: {
  children: React.ReactNode;
 }) {
   const [comfyUIStatus, setComfyUIStatus] = useState<string>('Idle');
   const [installationProgress, setInstallationProgress] = useState<number>(0);
   const [installationStep, setInstallationStep] = useState<string>('');
   const [comfyUIErrorMessage, setComfyUIErrorMessage] = useState<string | null>(null);

   useCharacterStore(); // Access state via getState() where needed

   useEffect(() => {
     const setupListeners = async () => {
       try {
         // Dynamically import the listen function
         const { listen } = await import('@tauri-apps/api/event');

         // Listen for ComfyUI status updates
         const unlistenStatus = await listen('comfyui-status-update', (event) => {
           setComfyUIStatus(event.payload as string);
           // Clear error message when status changes from Error
           if (event.payload !== 'Error') {
             setComfyUIErrorMessage(null);
           }
         });

         // Listen for installation progress updates
         const unlistenProgress = await listen('comfyui-install-progress', (event) => {
           const payload = event.payload as { step: string; progress: number };
           setInstallationStep(payload.step);
           setInstallationProgress(payload.progress);
         });

         // Listen for ComfyUI error messages
         const unlistenError = await listen('comfyui-error', (event) => {
           setComfyUIErrorMessage(event.payload as string);
           setComfyUIStatus('Error'); // Set status to Error on receiving an error
         });

         // Clean up listeners on component unmount
         return () => {
           unlistenStatus();
           unlistenProgress();
           unlistenError();
         };
       } catch (error) {
         console.error("Failed to set up Tauri event listeners:", error);
         // Handle error if dynamic import or listening fails
       }
     };

     if (typeof window !== 'undefined') {
       setupListeners();
     }
   }, []); // Empty dependency array means this effect runs once on mount

   // Existing useEffect for sending prompt (keeping for now)
   useEffect(() => {
     const sendPromptToComfyUI = async () => {
       try {
         const promptData = {
           workflow_type: "face",
           checkpoint_model: "zonkeyRealism_v42.safetensors",
           vae_model: "sdxl_vae.safetensors",
           positive_prompt: `Character attributes: { name: "Test Character", age: 25 }, tags: tag1, tag2`,
           negative_prompt: "ugly, disfigured, low quality, blurry",
           latent_width: 1024,
           latent_height: 1024,
           seed: 12345,
           image: null,
           faceid_params: null,
           additional_params: null
         };

         const promptDataForComfyUI = generateComfyUIPromptData(promptData);

         console.log("DEBUG: Sending to ComfyUI:", JSON.stringify(promptDataForComfyUI, null, 2)); // Log the request payload
         // Use Tauri HTTP plugin fetch
         const response = await tauriFetch("http://127.0.0.1:8188/prompt", {
           method: "POST",
           headers: {
             "Content-Type": "application/json",
           },
           body: JSON.stringify(promptDataForComfyUI), // Pass stringified JSON directly
           // responseType: httpPlugin.ResponseType.JSON // Remove invalid option
         });

         // Check status code for success
         if (response.status < 200 || response.status >= 300) {
           // Log the raw response data which might contain error details
           console.error("ComfyUI error response:", response);
           // Try to parse data as text or JSON depending on content type if available, otherwise use status
           const errorDetails = `Status code ${response.status}`; // Simplified error message
           throw new Error(`HTTP error! ${errorDetails}`);
         }

         // Assume response body is directly the parsed JSON data based on common plugin patterns
         // If this fails later, we might need response.json() or similar if available
         // Parse the JSON body from the response
         const data = await response.json(); // Use standard .json() method
         console.log("ComfyUI response:", data);
       } catch (error: unknown) {
         if (error instanceof Error) {
           console.error("Error sending prompt to ComfyUI:", error);
           setComfyUIErrorMessage((error as Error).message || "An error occurred");
           setComfyUIStatus('Error'); // Set status to Error on HTTP error
         } else {
           console.error("An unknown error occurred:", error);
           setComfyUIErrorMessage("An unknown error occurred.");
           setComfyUIStatus('Error'); // Set status to Error on unknown error
         }
       }
     };

     // sendPromptToComfyUI(); // Commenting out for now to avoid sending on every load
   }, []);


   return (
     <html lang="en">
       <body
         className={`${geist.className} ${geist_mono.className} antialiased`}
       >
         <TagContextWrapper>
           {/* ComfyUI Status Area */}
           <div className="p-4 bg-gray-200">
             <ComfyUIStatusIndicator status={comfyUIStatus as any} errorMessage={comfyUIErrorMessage} />
             {(comfyUIStatus === 'Setting Up' || comfyUIStatus === 'Installing Dependencies') && (
               <InstallationProgress currentStep={installationStep} progress={installationProgress} />
             )}
             {comfyUIErrorMessage && (
               <ErrorMessageDisplay message={comfyUIErrorMessage} action={{ text: 'View Logs', onClick: () => console.log('View Logs clicked') }} />
             )}
           </div>

           <div className="flex h-screen bg-gray-100">
             <div className="w-64 bg-gray-200 p-4">
               {/* Left Sidebar */}
               {/* {tags.map((tag) => (
                 <TagBadge key={tag.name} name={tag.name} category={tag.category} />
               ))} */}
               {/* Integrate CharacterSidebarPortrait */}
               <CharacterSidebarPortrait
                 imageUrl={useCharacterStore.getState().characterImageUrl ?? undefined}
                 isLoading={useCharacterStore.getState().loading}
                 isError={!!useCharacterStore.getState().error}
               />
             </div>
             <div className="flex-1 p-4">
               {children}
             </div>
           </div>
         </TagContextWrapper>
       </body>
     </html>
   );
 }
