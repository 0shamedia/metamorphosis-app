import Image from 'next/image';
import React from 'react';

interface CharacterSidebarPortraitProps {
  imageUrl?: string;
  isLoading?: boolean;
  isError?: boolean;
}

const CharacterSidebarPortrait: React.FC<CharacterSidebarPortraitProps> = ({
  imageUrl,
  isLoading = false,
  isError = false,
}) => {
  // Basic styling for the sidebar portrait - adjust as needed
  const containerStyle: React.CSSProperties = {
    width: '100%', // Or a fixed width suitable for the sidebar
    height: 'auto', // Maintain aspect ratio
    display: 'flex',
    justifyContent: 'center',
    alignItems: 'center',
    backgroundColor: '#f0f0f0', // Placeholder background
    overflow: 'hidden', // Ensure image doesn't overflow
  };


  if (isLoading) {
    return (
      <div style={{ ...containerStyle, height: '300px' /* Placeholder height */ }}>
        <p>Loading portrait...</p>
      </div>
    );
  }

  if (isError) {
    return (
      <div style={{ ...containerStyle, height: '300px' /* Placeholder height */ }}>
        <p>Error loading portrait.</p>
      </div>
    );
  }

  if (!imageUrl) {
    return (
      <div style={{ ...containerStyle, height: '300px' /* Placeholder height */ }}>
        <p>No portrait available.</p>
      </div>
    );
  }

  // next/image requires width and height, but we want it to be responsive.
  // A common pattern is to use a parent container for size and set layout="intrinsic" or "responsive"
  // For responsive behavior within a container, layout="responsive" is often used,
  // but requires width and height props that define the aspect ratio.
  // Let's assume a common aspect ratio or make it flexible within the container.
  // Using fill and a parent container with relative positioning is another approach for flexible sizing.
  // For simplicity and fitting within a sidebar, let's use fill and a parent container.

  return (
    <div style={{ ...containerStyle, position: 'relative', height: '300px' /* Example fixed height for container */ }}>
      <Image
        src={imageUrl}
        alt="Character Portrait"
        layout="fill" // Fills the parent container
        objectFit="contain" // Or 'cover'
        priority={true} // Prioritize loading for above-the-fold content
      />
    </div>
  );
};

export default CharacterSidebarPortrait;