import React from 'react';
import { Tag } from '../../../types/tags';
import tagService from '../../../services/tags/tagService';

interface TagDisplayProps {
  tagId: string;
  className?: string;
  showIcon?: boolean;
  showRarity?: boolean;
}

const TagDisplay: React.FC<TagDisplayProps> = ({
  tagId,
  className = '',
  showIcon = true,
  showRarity = false
}) => {
  const tag = tagService.getTag(tagId);

  if (!tag) {
    return (
      <div className={`text-xs bg-red-500/20 text-red-300 px-2 py-1 rounded ${className}`}>
        Unknown: {tagId}
      </div>
    );
  }

  const getRarityColor = (rarity: string) => {
    switch (rarity) {
      case 'common': return 'bg-gray-500/20 text-gray-300';
      case 'uncommon': return 'bg-green-500/20 text-green-300';
      case 'rare': return 'bg-blue-500/20 text-blue-300';
      case 'legendary': return 'bg-purple-500/20 text-purple-300';
      default: return 'bg-gray-500/20 text-gray-300';
    }
  };

  return (
    <div className={`text-xs px-2 py-1 rounded flex items-center gap-1 ${showRarity ? getRarityColor(tag.rarity) : 'bg-white/10 text-white/80'} ${className}`}>
      {showIcon && tag.icon && <span>{tag.icon}</span>}
      <span>{tag.name}</span>
      {showRarity && (
        <span className="text-xs opacity-60">
          {tag.rarity[0].toUpperCase()}
        </span>
      )}
    </div>
  );
};

export default TagDisplay;