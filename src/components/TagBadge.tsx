import React from 'react';

interface TagBadgeProps {
  name: string;
  category: 'clothing' | 'transformation' | 'gender';
}

const TagBadge: React.FC<TagBadgeProps> = ({ name, category }) => {
  let badgeColor = '';

  switch (category) {
    case 'clothing':
      badgeColor = 'bg-blue-200 text-blue-800';
      break;
    case 'transformation':
      badgeColor = 'bg-green-200 text-green-800';
      break;
    case 'gender':
      badgeColor = 'bg-purple-200 text-purple-800';
      break;
    default:
      badgeColor = 'bg-gray-200 text-gray-800';
      break;
  }

  return (
    <span className={`inline-flex items-center rounded-full px-3 py-0.5 text-sm font-medium ${badgeColor}`}>
      {name}
    </span>
  );
};

export default TagBadge;