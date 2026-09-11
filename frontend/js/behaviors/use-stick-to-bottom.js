/**
 * Headless auto-scroll behavior.
 * Keeps a scrollable container pinned to the bottom when new items are added,
 * unless the user has actively scrolled up to inspect previous content.
 * Respects prefers-reduced-motion.
 *
 * @param {HTMLElement} container - Scrollable DOM container.
 * @param {{ threshold?: number }} [options={}]
 * @returns {{
 *   scrollToBottom: (force?: boolean) => void,
 *   isNearBottom: () => boolean,
 *   destroy: () => void,
 * }}
 */
export function useStickToBottom(container, options = {}) {
  const { threshold = 100 } = options;
  let wasNearBottom = true;

  function isNearBottom() {
    if (!container) return true;
    const distance = container.scrollHeight - container.scrollTop - container.clientHeight;
    return distance <= threshold;
  }

  function handleScroll() {
    wasNearBottom = isNearBottom();
  }

  if (container && typeof container.addEventListener === 'function') {
    container.addEventListener('scroll', handleScroll, { passive: true });
  }

  function scrollToBottom(force = false) {
    if (!container) return;
    if (!force && !wasNearBottom) {
      return;
    }

    const prefersReducedMotion =
      typeof window !== 'undefined' &&
      window.matchMedia &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    const behavior = prefersReducedMotion ? 'auto' : 'smooth';

    try {
      if (typeof container.scrollTo === 'function') {
        container.scrollTo({
          top: container.scrollHeight,
          behavior,
        });
      } else {
        container.scrollTop = container.scrollHeight;
      }
    } catch {
      container.scrollTop = container.scrollHeight;
    }

    wasNearBottom = true;
  }

  function destroy() {
    if (container && typeof container.removeEventListener === 'function') {
      container.removeEventListener('scroll', handleScroll);
    }
  }

  return {
    scrollToBottom,
    isNearBottom,
    destroy,
  };
}
