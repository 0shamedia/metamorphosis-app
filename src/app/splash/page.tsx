'use client';

import { useRouter } from 'next/navigation';
import SplashScreenComponent from '../../components/setup/SplashScreen';
import { useEffect } from 'react'; // Removed useState as it's not used directly here

export default function SplashPage() {
  const router = useRouter();

  const handleSplashComplete = (destination: 'setup' | 'title') => {
    console.log(`[SplashPage] Splash complete, navigating to ${destination}`);
    if (destination === 'setup') {
      router.push('/setup');
    } else if (destination === 'title') {
      router.push('/title');
    }
  };

  // useEffect can be used here if SplashPage itself needs to react to
  // global state changes or perform actions, but for now,
  // SplashScreenComponent handles its own lifecycle and calls onComplete.
  useEffect(() => {
    // Placeholder for any page-specific logic if needed in the future
    console.log('[SplashPage] Mounted');
  }, []);

  return <SplashScreenComponent onComplete={handleSplashComplete} />;
}