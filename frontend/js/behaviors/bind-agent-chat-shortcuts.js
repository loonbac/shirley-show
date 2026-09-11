/**
 * Global keyboard shortcuts handler for Shirley Show AI Agent chat.
 * Binds Ctrl+K / Cmd+K to toggle the chat panel, suppressing activation during Stage Mode.
 *
 * @param {{
 *   onToggleChat: () => void,
 *   isStageModeActive: () => boolean,
 * }} options
 * @returns {() => void} Cleanup function to remove event listeners.
 */
export function bindAgentChatShortcuts(options) {
  const { onToggleChat, isStageModeActive } = options;

  function handleKeyDown(e) {
    // Check for Ctrl+K or Cmd+K
    const isK = (e.key === 'k' || e.key === 'K') || e.code === 'KeyK';
    const isModifier = e.ctrlKey || e.metaKey;

    if (isModifier && isK) {
      if (typeof isStageModeActive === 'function' && isStageModeActive()) {
        // Suppress during Stage Mode to preserve distraction-free performance
        return;
      }

      e.preventDefault();
      if (typeof onToggleChat === 'function') {
        onToggleChat();
      }
    }
  }

  window.addEventListener('keydown', handleKeyDown);

  return function unbind() {
    window.removeEventListener('keydown', handleKeyDown);
  };
}
