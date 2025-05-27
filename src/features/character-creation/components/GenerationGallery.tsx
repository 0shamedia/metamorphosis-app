import React from 'react';
import { ImageOption } from '@/types/character';

interface GenerationGalleryProps {
  images: ImageOption[];
  isLoading: boolean;
  onSelectImage: (image: ImageOption) => void;
  galleryTitle: string;
  selectedImageId?: string | null;
  itemType?: 'face' | 'body'; // To apply different styling if needed
}

const GenerationGallery: React.FC<GenerationGalleryProps> = ({
  images,
  isLoading,
  onSelectImage,
  galleryTitle,
  selectedImageId,
  itemType = 'face',
}) => {
  const getItemClasses = () => {
    if (itemType === 'face') {
      return 'w-32 h-32 md:w-36 md:h-36 rounded-full'; // Circular for faces
    }
    // Rectangular for full body, adjust aspect ratio as needed
    return 'w-40 h-64 md:w-48 md:h-72 rounded-lg'; 
  };

  const getGridClasses = () => {
    if (itemType === 'face') {
      // For face gallery: use flex for overlapping and scrolling
      return 'flex flex-row items-center p-4 overflow-x-auto justify-start w-full gallery-scrollbar space-x-[-64px]'; // Negative margin for overlap
    }
    // Body gallery remains vertically oriented, page scroll will handle overflow
    return 'grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6 p-4 justify-center w-full';
  }

  if (isLoading && images.length === 0) {
    return (
      <div className="text-center p-10">
        <h2 className="text-2xl font-semibold text-purple-200 mb-4">{galleryTitle}</h2>
        <div className="flex justify-center items-center space-x-2">
          <div className="w-8 h-8 bg-pink-500 rounded-full animate-bounce [animation-delay:-0.3s]"></div>
          <div className="w-8 h-8 bg-purple-500 rounded-full animate-bounce [animation-delay:-0.15s]"></div>
          <div className="w-8 h-8 bg-teal-500 rounded-full animate-bounce"></div>
        </div>
        <p className="text-purple-300 mt-4">Generating your options... please wait.</p>
      </div>
    );
  }

  return (
    <div className="w-full flex flex-col items-center backdrop-blur-xl rounded-3xl p-6 md:p-10 border border-white/10" style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)' }}>
      <h2 className="text-3xl font-bold bg-gradient-to-r from-pink-400 to-purple-400 text-transparent bg-clip-text mb-6 md:mb-8"
          style={{ textShadow: '0 0 20px rgba(236, 72, 153, 0.2)' }}
      >
        {galleryTitle}
      </h2>
      {images.length === 0 && !isLoading && (
        <p className="text-purple-300">No images to display. Try generating some!</p>
      )}
      <div className={getGridClasses()}>
        {images.map((image, index) => (
          // For face items, we don't need an extra wrapper if using space-x with negative margins on the container
          <div
            key={image.id}
            className={`
              ${getItemClasses()}
              overflow-hidden
              cursor-pointer
              transition-all duration-300 ease-in-out
              border-4
              hover:border-pink-500 hover:scale-105
              focus:outline-none focus:ring-4 focus:ring-pink-500 focus:ring-opacity-75
              shadow-lg hover:shadow-xl
              ${selectedImageId === image.id ? 'border-teal-400 scale-105 ring-4 ring-teal-400 ring-opacity-75 z-10' : 'border-purple-500/30'}
              flex-shrink-0 relative
            `}
            // z-index for selected item to be on top of others during overlap
            style={{
              backgroundImage: `url(${image.url})`,
              backgroundSize: 'cover',
              backgroundPosition: 'center',
              // marginLeft: itemType === 'face' && index > 0 ? '-64px' : '0', // Apply negative margin for overlap, handled by space-x now
            }}
            onClick={() => onSelectImage(image)}
            role="button"
            aria-label={`Select ${image.alt}`}
            tabIndex={0}
            onKeyPress={(e) => e.key === 'Enter' && onSelectImage(image)}
          >
            {/* Optional: Add an overlay or icon on hover/selection */}
          </div>
        ))}
      </div>
      {isLoading && images.length > 0 && (
         <p className="text-purple-300 mt-4 text-sm">Loading more options...</p>
      )}
    </div>
  );
};

// Add a style block for the custom scrollbar
const styles = `
  .gallery-scrollbar::-webkit-scrollbar {
    height: 8px; /* For horizontal scrollbar */
  }
  .gallery-scrollbar::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
  }
  .gallery-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(236, 72, 153, 0.3); /* Pinkish color from theme */
    border-radius: 4px;
    transition: background 0.2s ease;
  }
  .gallery-scrollbar::-webkit-scrollbar-thumb:hover {
    background: rgba(236, 72, 153, 0.5);
  }
  .gallery-scrollbar {
    scrollbar-width: thin; /* For Firefox */
    scrollbar-color: rgba(236, 72, 153, 0.3) rgba(255, 255, 255, 0.05); /* For Firefox */
  }
`;

// Inject styles into the head
if (typeof window !== 'undefined') {
  const styleSheet = document.createElement("style");
  styleSheet.type = "text/css";
  styleSheet.innerText = styles;
  document.head.appendChild(styleSheet);
}

export default GenerationGallery;