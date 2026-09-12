/**
 * Safely decodes a URI component string.
 * Returns null if decoding fails (e.g. malformed percent-encoding).
 * @param {string} val 
 * @returns {string | null}
 */
function safeDecodeURIComponent(val) {
  try {
    return decodeURIComponent(val);
  } catch (_err) {
    return null;
  }
}

/**
 * Pure function to parse a URL hash into a route representation.
 * Supported routes:
 *   #/presentations -> { view: 'presentations', params: {} }
 *   #/presentations/:id -> { view: 'presentation-detail', params: { id } }
 *   #/eventos -> { view: 'events', params: {} }
 *   #/eventos/:id -> { view: 'event-detail', params: { id } }
 *   #/ticket/:token -> { view: 'public-ticket', params: { token } }
 *   #/puerta/:id -> { view: 'door', params: { id } }
 *
 * @param {string} hash 
 * @returns {{
 *   view: 'presentations' | 'presentation-detail' | 'events' | 'event-detail' | 'public-ticket' | 'door',
 *   params: Record<string, string>,
 *   fallback?: boolean
 * }}
 */
export function parseRoute(hash) {
  const cleanHash = (hash || '').replace(/^#/, '').trim();

  // 1. Presentations catalog (default / empty)
  if (!cleanHash || cleanHash === '/' || cleanHash === '/presentations' || cleanHash === 'presentations' || cleanHash === '/presentations/') {
    return {
      view: 'presentations',
      params: {},
    };
  }

  // 2. Presentations detail
  const presentationDetailMatch = cleanHash.match(/^\/?presentations\/([^/?#]+)$/);
  if (presentationDetailMatch) {
    const id = safeDecodeURIComponent(presentationDetailMatch[1]);
    if (id !== null) {
      return {
        view: 'presentation-detail',
        params: { id },
      };
    }
    return {
      view: 'presentations',
      params: {},
      fallback: true,
    };
  }

  // 3. Events catalog
  if (cleanHash === '/eventos' || cleanHash === 'eventos' || cleanHash === '/eventos/') {
    return {
      view: 'events',
      params: {},
    };
  }

  // 4. Event detail
  const eventDetailMatch = cleanHash.match(/^\/?eventos\/([^/?#]+)$/);
  if (eventDetailMatch) {
    const id = safeDecodeURIComponent(eventDetailMatch[1]);
    if (id !== null) {
      return {
        view: 'event-detail',
        params: { id },
      };
    }
    return {
      view: 'presentations',
      params: {},
      fallback: true,
    };
  }

  // 5. Public ticket view
  const publicTicketMatch = cleanHash.match(/^\/?ticket\/([^/?#]+)$/);
  if (publicTicketMatch) {
    const token = safeDecodeURIComponent(publicTicketMatch[1]);
    if (token !== null) {
      return {
        view: 'public-ticket',
        params: { token },
      };
    }
    return {
      view: 'presentations',
      params: {},
      fallback: true,
    };
  }

  // 6. Door / check-in view
  const doorMatch = cleanHash.match(/^\/?puerta\/([^/?#]+)$/);
  if (doorMatch) {
    const id = safeDecodeURIComponent(doorMatch[1]);
    if (id !== null) {
      return {
        view: 'door',
        params: { id },
      };
    }
    return {
      view: 'presentations',
      params: {},
      fallback: true,
    };
  }

  // Any other route falls back to presentations catalog
  return {
    view: 'presentations',
    params: {},
    fallback: true,
  };
}
