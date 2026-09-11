/**
 * Pure function to parse a URL hash into a route representation.
 * Supported routes:
 *   #/presentations -> { view: 'presentations', params: {} }
 *   #/presentations/:id -> { view: 'presentation-detail', params: { id } }
 * @param {string} hash 
 * @returns {{ view: 'presentations' | 'presentation-detail', params: Record<string, string>, fallback?: boolean }}
 */
export function parseRoute(hash) {
  const cleanHash = (hash || '').replace(/^#/, '').trim();

  if (!cleanHash || cleanHash === '/' || cleanHash === '/presentations' || cleanHash === 'presentations') {
    return {
      view: 'presentations',
      params: {},
    };
  }

  // Check for #/presentations/:id
  const detailMatch = cleanHash.match(/^\/?presentations\/([^/?#]+)$/);
  if (detailMatch) {
    return {
      view: 'presentation-detail',
      params: {
        id: decodeURIComponent(detailMatch[1]),
      },
    };
  }

  // Any other route falls back to presentations catalog
  return {
    view: 'presentations',
    params: {},
    fallback: true,
  };
}
