"use client";

import React, { createContext, useState, useContext } from 'react';

interface Tag {
  name: string;
  category: 'clothing' | 'transformation' | 'gender';
  description?: string;
}

interface TagContextProps {
  tags: Tag[];
  addTag: (tag: Tag) => void;
}

const TagContext = createContext<TagContextProps | undefined>(undefined);

export const TagProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [tags, setTags] = useState<Tag[]>([
    { name: "Cute", category: "clothing" },
    { name: "Large Breasts", category: "transformation" },
    { name: "Female", category: "gender" },
  ]);

  const addTag = (tag: Tag) => {
    setTags([...tags, tag]);
  };

  const value: TagContextProps = {
    tags,
    addTag,
  };

  console.log("Rendering TagProvider, providing context:", value); // DEBUG LOG
  return (
    <TagContext.Provider value={value}>
      {children}
    </TagContext.Provider>
  );
};

export const useTagContext = () => {
  const context = useContext(TagContext);
  if (!context) {
    throw new Error('useTagContext must be used within a TagProvider');
  }
  return context;
};