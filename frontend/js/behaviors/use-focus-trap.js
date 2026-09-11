/**
 * Headless behavior for trapping keyboard focus within a modal/dialog element.
 * Restores focus to the previously focused element upon deactivation or destruction.
 *
 * @param {HTMLElement} element - Container element to trap focus inside.
 * @param {{ onEscape?: () => void }} [options={}]
 * @returns {{
 *   activate: () => void,
 *   deactivate: () => void,
 *   destroy: () => void,
 * }}
 */
export function useFocusTrap(element, options = {}) {
  const { onEscape } = options;
  let previousActiveElement = null;
  let isActive = false;

  const FOCUSABLE_SELECTORS = [
    'a[href]',
    'area[href]',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    'button:not([disabled])',
    '[tabindex]:not([tabindex="-1"])',
  ].join(', ');

  function getFocusableElements() {
    if (!element || typeof element.querySelectorAll !== 'function') return [];
    const elements = Array.from(element.querySelectorAll(FOCUSABLE_SELECTORS));
    return elements.filter((el) => {
      // Basic visibility check
      return el.style.display !== 'none' && !el.hasAttribute('disabled');
    });
  }

  function handleKeyDown(e) {
    if (!isActive) return;

    if (e.key === 'Escape' && typeof onEscape === 'function') {
      e.preventDefault();
      onEscape();
      return;
    }

    if (e.key !== 'Tab') return;

    const focusables = getFocusableElements();
    if (focusables.length === 0) {
      e.preventDefault();
      return;
    }

    const first = focusables[0];
    const last = focusables[focusables.length - 1];

    if (e.shiftKey) {
      if (document.activeElement === first || !element.contains(document.activeElement)) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last || !element.contains(document.activeElement)) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  function activate() {
    if (isActive) return;
    isActive = true;
    previousActiveElement = document.activeElement;

    window.addEventListener('keydown', handleKeyDown);

    const focusables = getFocusableElements();
    if (focusables.length > 0) {
      // Focus first focusable element (prefer textarea if present)
      const preferred = focusables.find((el) => el.tagName.toLowerCase() === 'textarea') || focusables[0];
      preferred.focus();
    }
  }

  function deactivate() {
    if (!isActive) return;
    isActive = false;
    window.removeEventListener('keydown', handleKeyDown);

    if (previousActiveElement && typeof previousActiveElement.focus === 'function') {
      try {
        previousActiveElement.focus();
      } catch {
        // ignore
      }
    }
    previousActiveElement = null;
  }

  function destroy() {
    deactivate();
  }

  return {
    activate,
    deactivate,
    destroy,
  };
}
