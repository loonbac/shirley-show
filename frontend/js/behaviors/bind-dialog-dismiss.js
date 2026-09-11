/**
 * Binds dismiss behaviors (Escape key and clicking backdrop) to a dialog overlay.
 * @param {HTMLElement} overlayEl 
 * @param {() => void} onDismiss 
 * @returns {() => void} cleanup function
 */
export function bindDialogDismiss(overlayEl, onDismiss) {
  if (!overlayEl || typeof onDismiss !== 'function') {
    return () => {};
  }

  const handleKeyDown = (event) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      onDismiss();
    }
  };

  const handleClick = (event) => {
    if (event.target === overlayEl) {
      event.preventDefault();
      onDismiss();
    }
  };

  document.addEventListener('keydown', handleKeyDown);
  overlayEl.addEventListener('click', handleClick);

  return () => {
    document.removeEventListener('keydown', handleKeyDown);
    overlayEl.removeEventListener('click', handleClick);
  };
}
