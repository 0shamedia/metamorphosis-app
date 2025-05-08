'use client';

import { useRouter } from 'next/navigation';
import SplashScreenComponent from '../../components/setup/SplashScreen';
import { useEffect } from 'react'; // Removed useState as it's not used directly here

export default function SplashPage() {
  const router = useRouter();

  const handleSplashComplete = () => {
    console.log('[SplashPage] Splash complete, navigating to /setup');
    // In a real scenario, you might check if setup is needed.
    // For now, always navigate to /setup as per docs/ui/setup_flow.md
    router.push('/setup');
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