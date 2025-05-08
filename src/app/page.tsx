'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function Home() {
  const router = useRouter();

  useEffect(() => {
    router.replace('/splash');
  }, [router]);

  // Render null or a loading indicator while redirecting
  return null;
}