import React, { useState, useEffect } from 'react';
import { Tag, TagAcquisition as TagAcquisitionType } from '../../../types/tags';
import { useTagStore } from '../../../store/tagStore';
import tagService from '../../../services/tags/tagService';

interface TagAcquisitionProps {
  isOpen: boolean;
  onClose: () => void;
  onAccept?: (tagId: string) => void;
  onReject?: (tagId: string) => void;
  className?: string;
  inline?: boolean;
}

interface NotificationItem {
  tag: Tag;
  source: TagAcquisitionType['source'];
  context?: string;
  id: string;
}

const TagAcquisition: React.FC<TagAcquisitionProps> = ({
  isOpen,
  onClose,
  onAccept,
  className = '',
  inline = false
}) => {
  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [currentNotification, setCurrentNotification] = useState<NotificationItem | null>(null);
  const [isVisible, setIsVisible] = useState(false);
  const [isAnimatingOut, setIsAnimatingOut] = useState(false);

  const { lastAcquisition } = useTagStore();

  // Monitor for new acquisitions
  useEffect(() => {
    if (lastAcquisition && isOpen) {
      const tag = tagService.getTag(lastAcquisition.tagId);
      if (tag) {
        const newNotification: NotificationItem = {
          tag,
          source: lastAcquisition.source,
          context: lastAcquisition.context,
          id: `${Date.now()}-${Math.random()}`
        };
        
        setNotifications(prev => [...prev, newNotification]);
      }
    }
  }, [lastAcquisition, isOpen]);

  // Process notification queue
  useEffect(() => {
    if (notifications.length > 0 && !currentNotification) {
      const nextNotification = notifications[0];
      setCurrentNotification(nextNotification);
      setNotifications(prev => prev.slice(1));
      
      // Auto-accept the tag
      if (onAccept) {
        onAccept(nextNotification.tag.id);
      }
      
      // Show the notification
      setIsVisible(true);
      
      // Auto-dismiss after 4 seconds
      setTimeout(() => {
        dismissNotification();
      }, 4000);
    }
  }, [notifications, currentNotification, onAccept]);

  const dismissNotification = () => {
    setIsAnimatingOut(true);
    
    setTimeout(() => {
      setIsVisible(false);
      setIsAnimatingOut(false);
      setCurrentNotification(null);
      
      // Close if no more notifications
      if (notifications.length === 0) {
        onClose();
      }
    }, 500);
  };

  const getSourceIcon = (source: TagAcquisitionType['source']) => {
    switch (source) {
      case 'creation': return '🎨';
      case 'choice': return '🤔';
      case 'discovery': return '🔍';
      case 'reward': return '🏆';
      case 'trade': return '🤝';
      case 'automatic': return '⚙️';
      default: return '✨';
    }
  };

  const getSourceLabel = (source: TagAcquisitionType['source']) => {
    switch (source) {
      case 'creation': return 'Character Creation';
      case 'choice': return 'Story Choice';
      case 'discovery': return 'Discovery';
      case 'reward': return 'Achievement Reward';
      case 'trade': return 'Trade';
      case 'automatic': return 'Automatic';
      default: return 'Manual';
    }
  };

  const getRarityColor = (rarity: string) => {
    switch (rarity) {
      case 'common': return 'from-gray-400 to-gray-600';
      case 'uncommon': return 'from-green-400 to-green-600';
      case 'rare': return 'from-blue-400 to-blue-600';
      case 'legendary': return 'from-purple-400 to-purple-600';
      default: return 'from-gray-400 to-gray-600';
    }
  };

  const getRarityGlow = (rarity: string) => {
    switch (rarity) {
      case 'common': return 'shadow-gray-400/50';
      case 'uncommon': return 'shadow-green-400/50';
      case 'rare': return 'shadow-blue-400/50';
      case 'legendary': return 'shadow-purple-400/50';
      default: return 'shadow-gray-400/50';
    }
  };

  if (!isOpen || !isVisible || !currentNotification) {
    return null;
  }

  if (inline) {
    // Inline mode for sidebar - simplified notification
    return (
      <div className={`
        transform transition-all duration-500 ease-out
        ${isAnimatingOut ? 'translate-x-full opacity-0' : 'translate-x-0 opacity-100'}
        ${className}
      `}>
        <div className={`
          bg-gray-900/95 backdrop-blur-md rounded-xl border border-white/20 
          shadow-2xl overflow-hidden ${getRarityGlow(currentNotification.tag.rarity)}
        `}>
          {/* Rarity top bar */}
          <div className={`h-1 bg-gradient-to-r ${getRarityColor(currentNotification.tag.rarity)}`} />
          
          {/* Content */}
          <div className="p-4">
            <div className="flex items-center gap-3">
              {/* Icon */}
              <div className="text-2xl flex-shrink-0">
                {getSourceIcon(currentNotification.source)}
              </div>
              
              {/* Text content */}
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-purple-300 mb-1">
                  Tag Acquired!
                </div>
                <div className="text-lg font-bold text-white truncate">
                  {currentNotification.tag.name}
                </div>
                <div className="text-xs text-white/60">
                  {getSourceLabel(currentNotification.source)}
                </div>
              </div>
              
              {/* Close button */}
              <button
                onClick={dismissNotification}
                className="text-white/40 hover:text-white/80 transition-colors flex-shrink-0"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Full screen notification mode - WoW achievement style
  return (
    <div className={`fixed inset-0 z-50 pointer-events-none ${className}`}>
      {/* Center notification */}
      <div className="flex items-center justify-center h-full">
        <div className={`
          transform transition-all duration-500 ease-out pointer-events-auto
          ${isAnimatingOut ? 'scale-75 opacity-0 translate-y-8' : 'scale-100 opacity-100 translate-y-0'}
        `}>
          <div className={`
            bg-gray-900/95 backdrop-blur-xl rounded-2xl border-2 border-white/20 
            shadow-2xl overflow-hidden max-w-md mx-4 ${getRarityGlow(currentNotification.tag.rarity)}
          `}>
            {/* Animated rarity border */}
            <div className={`h-2 bg-gradient-to-r ${getRarityColor(currentNotification.tag.rarity)} animate-pulse`} />
            
            {/* Content */}
            <div className="p-6">
              {/* Header */}
              <div className="text-center mb-4">
                <div className="text-4xl mb-2 animate-bounce">
                  {getSourceIcon(currentNotification.source)}
                </div>
                <h2 className="text-xl font-bold text-white mb-1">
                  Tag Acquired!
                </h2>
                <p className="text-sm text-purple-300">
                  {getSourceLabel(currentNotification.source)}
                </p>
              </div>

              {/* Tag showcase */}
              <div className={`
                relative bg-gradient-to-br ${getRarityColor(currentNotification.tag.rarity)}
                rounded-xl p-1 mb-4 shadow-lg
              `}>
                <div className="bg-gray-800/90 rounded-lg p-4">
                  <div className="text-center">
                    {/* Tag icon */}
                    {currentNotification.tag.icon && (
                      <div className="text-3xl mb-2">{currentNotification.tag.icon}</div>
                    )}
                    
                    {/* Tag name */}
                    <h3 className="text-2xl font-bold text-white mb-2">
                      {currentNotification.tag.name}
                    </h3>
                    
                    {/* Rarity badge */}
                    <div className={`
                      inline-block px-3 py-1 rounded-full text-xs font-medium mb-3
                      bg-white/20 text-white
                    `}>
                      {currentNotification.tag.rarity.charAt(0).toUpperCase() + currentNotification.tag.rarity.slice(1)}
                    </div>
                    
                    {/* Description */}
                    <p className="text-white/80 text-sm leading-relaxed">
                      {currentNotification.tag.description}
                    </p>
                  </div>
                </div>
              </div>

              {/* Auto-dismiss indicator */}
              <div className="text-center">
                <button
                  onClick={dismissNotification}
                  className="text-white/60 hover:text-white/90 transition-colors text-sm"
                >
                  Click to dismiss
                </button>
                <div className="mt-2 w-full bg-white/20 rounded-full h-1 overflow-hidden">
                  <div 
                    className="h-full bg-purple-400 rounded-full animate-[width-shrink_4s_linear]"
                    style={{
                      animation: 'width-shrink 4s linear forwards'
                    }}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <style jsx>{`
        @keyframes width-shrink {
          from { width: 100%; }
          to { width: 0%; }
        }
      `}</style>
    </div>
  );
};

export default TagAcquisition;