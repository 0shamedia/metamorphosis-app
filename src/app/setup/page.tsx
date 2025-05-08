'use client';

import { useRouter } from 'next/navigation';
import SetupScreenComponent from '../../components/setup/SetupScreen'; // This will be the enhanced existing SetupScreen
import { useEffect } from 'react';

export default function SetupPage() {
  const router = useRouter();

  const handleSetupComplete = () => {
    console.log('[SetupPage] Setup complete, navigating to /title');
    router.push('/title');
  };

  useEffect(() => {
    console.log('[SetupPage] Mounted');
  }, []);

  return <SetupScreenComponent onComplete={handleSetupComplete} />;
}