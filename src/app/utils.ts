export function generateComfyUIPromptData(promptData: {
  workflow_type: string;
  checkpoint_model: string;
  vae_model: string;
  positive_prompt: string;
  negative_prompt: string;
  latent_width: number;
  latent_height: number;
  seed: number;
  image: unknown; // Replaced any with unknown
  faceid_params: unknown; // Replaced any with unknown
  additional_params: unknown; // Replaced any with unknown
}): unknown { // Replaced any with unknown
  const {
  checkpoint_model,
  vae_model,
  positive_prompt,
  negative_prompt,
  latent_width,
  latent_height,
  seed,
  } = promptData;

  const prompt = {
  "4": {
  "inputs": {
  "ckpt_name": checkpoint_model
  },
  "class_type": "CheckpointLoaderSimple",
  },
  "5": {
  "inputs": {
  "width": latent_width,
  "height": latent_height,
  "batch_size": 1
  },
  "class_type": "EmptyLatentImage",
  },
  "6": {
  "inputs": {
  "text": positive_prompt,
  "clip": ["4", 1]
  },
  "class_type": "CLIPTextEncode",
  },
  "7": {
  "inputs": {
  "text": negative_prompt,
  "clip": ["4", 1]
  },
  "class_type": "CLIPTextEncode",
  },
  "37": {
  "inputs": {
  "vae_name": vae_model
  },
  "class_type": "VAELoader",
  },
  "75": {
  "inputs": {
  "add_noise": "enable",
  "noise_seed": seed,
  "steps": 20,
  "cfg": 3,
  "sampler_name": "euler_ancestral",
  "scheduler": "normal",
  "start_at_step": 0,
  "end_at_step": 10000,
  "return_with_leftover_noise": "disable",
  "preview_method": "auto",
  "vae_decode": "true",
  "model": ["4", 0],
  "positive": ["6", 0],
  "negative": ["7", 0],
  "latent_image": ["5", 0],
  "optional_vae": ["37", 0]
  },
  "class_type": "KSampler Adv. (Efficient)",
  },
  "125": {
  "inputs": {
  "samples": ["75", 3],
  "vae": ["75", 4]
  },
  "class_type": "VAEDecode",
  },
  "9": {
  "inputs": {
  "filename_prefix": "ComfyUI",
  "images": ["125", 0]
  },
  "class_type": "SaveImage",
  },
  "client_id": "metamorphosis-app"
  };

  return {
  prompt: prompt,
  };
}

// Temporary function to verify communication with ComfyUI sidecar
export async function verifyComfyUICommunication() {
  console.log("Attempting to verify ComfyUI communication...");
  try {
    // Assuming ComfyUI is running on the default port 8188
    const response = await fetch('http://127.0.0.1:8188/queue');
    if (response.ok) {
      const data = await response.json();
      console.log("ComfyUI communication successful. /queue response:", data);
      return true;
    } else {
      console.error(`ComfyUI communication failed: HTTP status ${response.status}`);
      return false;
    }
  } catch (error) {
    console.error("ComfyUI communication failed:", error);
    return false;
  }
}