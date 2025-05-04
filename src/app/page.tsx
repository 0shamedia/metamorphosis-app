"use client";

import Image from "next/image";
import { useState, useEffect } from "react";
import ModelDownloader from "@/components/ModelDownloader/ModelDownloader";

export default function Home() {
  const [status, setStatus] = useState("Idle");
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const setupListener = async () => {
      try {
        // Dynamically import the listen function
        const { listen } = await import("@tauri-apps/api/event");

        const unlisten = await listen("installation-status", (event) => {
          const payload = event.payload as {
            status: string;
            progress?: number;
            error?: string;
          };
          setStatus(payload.status);
          if (payload.progress !== undefined) {
            setProgress(payload.progress);
          }
          if (payload.error !== undefined) {
            setError(payload.error);
          }
        });

        return () => {
          unlisten();
        };
      } catch (error) {
        console.error("Failed to set up Tauri event listener in page.tsx:", error);
        // Handle error if dynamic import or listening fails
      }
    };

    if (typeof window !== 'undefined') {
      setupListener();
    }
  }, []);

  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main>
        <div>Main Content Area</div>
      </main>
      <footer className="row-start-3 flex gap-[24px] flex-wrap items-center justify-center">
        <a
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://nextjs.org/learn?utm_source=create-next-app&utm_medium=appdir-template-tw&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/file.svg"
            alt="File icon"
            width={16}
            height={16}
          />
          Learn
        </a>
        <a
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://vercel.com/templates?framework=next.js&utm_source=create-next-app&utm_medium=appdir-template-tw&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/window.svg"
            alt="Window icon"
            width={16}
            height={16}
          />
          Examples
        </a>
        <a
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://nextjs.org?utm_source=create-next-app&utm_medium=appdir-template-tw&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/globe.svg"
            alt="Globe icon"
            width={16}
            height={16}
          />
          Go to nextjs.org →
        </a>
      </footer>
    </div>
  );
}
