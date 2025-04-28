"use client";

 import { Geist, Geist_Mono } from "next/font/google";
 import "./globals.css";
 import TagContextWrapper from "@/components/TagContextWrapper";
 import { useState, useEffect } from "react";
 import * as httpPlugin from '@tauri-apps/plugin-http'; // Import the entire module
 import { generateComfyUIPromptData } from "./utils";

 // Remove destructuring and log

 const geist = Geist({ subsets: ['latin'] });
 const geist_mono = Geist_Mono({ subsets: ['latin'] });

 export default function RootLayout({
  children,
 }: {
  children: React.ReactNode;
 }) {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

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
  const response = await httpPlugin.fetch("http://127.0.0.1:8188/prompt", {
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
    setErrorMessage((error as Error).message || "An error occurred");
  } else {
    console.error("An unknown error occurred:", error);
    setErrorMessage("An unknown error occurred.");
  }
  // Proper type checking for error message
  if (error instanceof Error) {
    setErrorMessage(error.message);
  } else {
    setErrorMessage("An unknown error occurred");
  }
  }
  };

  sendPromptToComfyUI();
  }, []);

  return (
  <html lang="en">
  <body
  className={`${geist.className} ${geist_mono.className} antialiased`}
  >
  <TagContextWrapper>
  {errorMessage && (
  <div className="bg-red-500 text-white p-2">{errorMessage}</div>
  )}
  <div className="flex h-screen bg-gray-100">
  <div className="w-64 bg-gray-200 p-4">
  {/* Left Sidebar */}
  {/* {tags.map((tag) => (
  <TagBadge key={tag.name} name={tag.name} category={tag.category} />
  ))} */}
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
