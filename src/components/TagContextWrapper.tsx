"use client";

import React from 'react';
import { TagProvider } from './TagContext';

interface TagContextWrapperProps {
  children: React.ReactNode;
}

const TagContextWrapper: React.FC<TagContextWrapperProps> = ({ children }) => {
  console.log("Rendering TagContextWrapper"); // DEBUG LOG
  return (
    <TagProvider>
      {children}
    </TagProvider>
  );
};

export default TagContextWrapper;