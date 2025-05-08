'use client';

import TitleScreenComponent from '../../components/setup/TitleScreen'; // This will be the enhanced existing TitleScreen
import { useEffect } from 'react';
// import { useRouter } from 'next/navigation'; // Not strictly needed if TitleScreen handles its own internal navigation triggers

export default function TitlePage() {
  // const router = useRouter(); // Only if page needs to initiate navigation

  useEffect(() => {
    console.log('[TitlePage] Mounted');
  }, []);

  // TitleScreenComponent is expected to handle its own button actions,
  // which might involve navigation using useRouter internally.
  return <TitleScreenComponent />;
}