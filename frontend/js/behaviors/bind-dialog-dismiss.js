/**
 * Shared state for locking background body scroll while dialogs are open.
 * Supports nested/concurrent dialogs, captures/restores exact page scroll offset,
 * pins the body to prevent scrolling on iOS Safari without visual jump,
 * and restores exact prior body styles on the final release.
 */
let activeDialogCount = 0;
let savedScrollPosition = null;
let previousBodyStyles = null;

const LOCKED_STYLE_PROPERTIES = ['position', 'top', 'left', 'width', 'overflow'];

function getScrollPosition() {
  let x = null;
  let y = null;

  if (typeof window !== 'undefined') {
    if (typeof window.scrollX === 'number' && !Number.isNaN(window.scrollX)) {
      x = window.scrollX;
    } else if (typeof window.pageXOffset === 'number' && !Number.isNaN(window.pageXOffset)) {
      x = window.pageXOffset;
    }

    if (typeof window.scrollY === 'number' && !Number.isNaN(window.scrollY)) {
      y = window.scrollY;
    } else if (typeof window.pageYOffset === 'number' && !Number.isNaN(window.pageYOffset)) {
      y = window.pageYOffset;
    }
  }

  if (typeof document !== 'undefined') {
    if (x === null) {
      if (document.documentElement && typeof document.documentElement.scrollLeft === 'number' && !Number.isNaN(document.documentElement.scrollLeft)) {
        x = document.documentElement.scrollLeft;
      } else if (document.body && typeof document.body.scrollLeft === 'number' && !Number.isNaN(document.body.scrollLeft)) {
        x = document.body.scrollLeft;
      }
    }

    if (y === null) {
      if (document.documentElement && typeof document.documentElement.scrollTop === 'number' && !Number.isNaN(document.documentElement.scrollTop)) {
        y = document.documentElement.scrollTop;
      } else if (document.body && typeof document.body.scrollTop === 'number' && !Number.isNaN(document.body.scrollTop)) {
        y = document.body.scrollTop;
      }
    }
  }

  return {
    x: Math.max(0, x ?? 0),
    y: Math.max(0, y ?? 0),
  };
}

function restoreScrollPosition(x, y) {
  if (typeof window !== 'undefined') {
    if (typeof window.scrollTo === 'function') {
      try {
        window.scrollTo(x, y);
      } catch {}
    }
    try { window.scrollX = x; } catch {}
    try { window.pageXOffset = x; } catch {}
    try { window.scrollY = y; } catch {}
    try { window.pageYOffset = y; } catch {}
  }

  if (typeof document !== 'undefined') {
    if (document.documentElement) {
      if ('scrollLeft' in document.documentElement) {
        document.documentElement.scrollLeft = x;
      }
      if ('scrollTop' in document.documentElement) {
        document.documentElement.scrollTop = y;
      }
    }
    if (document.body) {
      if ('scrollLeft' in document.body) {
        document.body.scrollLeft = x;
      }
      if ('scrollTop' in document.body) {
        document.body.scrollTop = y;
      }
    }
  }
}

function captureBodyStyles() {
  if (typeof document === 'undefined' || !document.body || !document.body.style) {
    return null;
  }
  const styles = {};
  for (const prop of LOCKED_STYLE_PROPERTIES) {
    const hasProp = prop in document.body.style && document.body.style[prop] !== undefined;
    styles[prop] = {
      hasProp,
      value: document.body.style[prop],
    };
  }
  return styles;
}

function restoreBodyStyles(styles) {
  if (typeof document === 'undefined' || !document.body || !document.body.style || !styles) {
    return;
  }
  for (const prop of LOCKED_STYLE_PROPERTIES) {
    const recorded = styles[prop];
    if (recorded && recorded.hasProp && recorded.value !== undefined) {
      document.body.style[prop] = recorded.value;
    } else {
      try {
        delete document.body.style[prop];
      } catch {
        document.body.style[prop] = '';
      }
      if (typeof document.body.style.removeProperty === 'function') {
        document.body.style.removeProperty(prop);
      }
    }
  }
}

function acquireBodyLock() {
  if (typeof document === 'undefined' || !document.body || !document.body.style) {
    return;
  }

  if (activeDialogCount === 0) {
    const { x, y } = getScrollPosition();
    savedScrollPosition = { x, y };
    previousBodyStyles = captureBodyStyles();

    document.body.style.position = 'fixed';
    document.body.style.top = y ? `-${y}px` : '0px';
    document.body.style.left = x ? `-${x}px` : '0px';
    document.body.style.width = '100%';
    document.body.style.overflow = 'hidden';

    if (document.body.classList?.add) {
      document.body.classList.add('dialog-open');
    }
  }

  activeDialogCount++;
}

function releaseBodyLock() {
  if (typeof document === 'undefined' || !document.body) {
    return;
  }

  if (activeDialogCount <= 0) {
    activeDialogCount = 0;
    return;
  }

  activeDialogCount--;

  if (activeDialogCount === 0) {
    if (previousBodyStyles && document.body.style) {
      restoreBodyStyles(previousBodyStyles);
    }

    if (document.body.classList?.remove) {
      document.body.classList.remove('dialog-open');
    }

    if (savedScrollPosition) {
      const { x, y } = savedScrollPosition;
      restoreScrollPosition(x, y);
    }

    savedScrollPosition = null;
    previousBodyStyles = null;
  }
}

/**
 * Reset body scroll lock state (for testing isolation).
 */
export function _resetBodyScrollLockForTesting() {
  if (typeof document !== 'undefined' && document.body) {
    if (previousBodyStyles && document.body.style) {
      restoreBodyStyles(previousBodyStyles);
    } else if (document.body.style) {
      for (const prop of LOCKED_STYLE_PROPERTIES) {
        try {
          delete document.body.style[prop];
        } catch {
          document.body.style[prop] = '';
        }
        if (typeof document.body.style.removeProperty === 'function') {
          document.body.style.removeProperty(prop);
        }
      }
    }
    if (document.body.classList?.remove) {
      document.body.classList.remove('dialog-open');
    }
  }
  activeDialogCount = 0;
  savedScrollPosition = null;
  previousBodyStyles = null;
}

/**
 * Get active dialog count (for testing/inspection).
 */
export function _getActiveDialogCount() {
  return activeDialogCount;
}

/**
 * Get saved scroll position (for testing/inspection).
 */
export function _getSavedScrollPosition() {
  return savedScrollPosition ? { ...savedScrollPosition } : null;
}

/**
 * Binds dismiss behaviors (Escape key and clicking backdrop) to a dialog overlay
 * and manages the shared body scroll lock while open.
 * @param {HTMLElement} overlayEl 
 * @param {() => void} onDismiss 
 * @returns {() => void} cleanup function (idempotent)
 */
export function bindDialogDismiss(overlayEl, onDismiss) {
  if (!overlayEl || typeof onDismiss !== 'function') {
    return () => {};
  }

  acquireBodyLock();

  const handleKeyDown = (event) => {
    if (event.key === 'Escape') {
      if (typeof event.preventDefault === 'function') {
        event.preventDefault();
      }
      onDismiss();
    }
  };

  const handleClick = (event) => {
    if (event.target === overlayEl) {
      if (typeof event.preventDefault === 'function') {
        event.preventDefault();
      }
      onDismiss();
    }
  };

  document.addEventListener('keydown', handleKeyDown);
  overlayEl.addEventListener('click', handleClick);

  let isCleanedUp = false;

  return () => {
    if (isCleanedUp) {
      return;
    }
    isCleanedUp = true;

    document.removeEventListener('keydown', handleKeyDown);
    overlayEl.removeEventListener('click', handleClick);
    releaseBodyLock();
  };
}
