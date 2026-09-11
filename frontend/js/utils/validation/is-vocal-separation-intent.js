/**
 * Determines whether a user message expresses an intention to perform vocal
 * or instrumental separation on an audio track.
 * Pure utility function adhering to extreme micro-modularity standards.
 *
 * @param {string|null|undefined} message - The message text to analyze.
 * @returns {boolean} True if the message expresses vocal separation intent.
 */
export function isVocalSeparationIntent(message) {
  if (!message || typeof message !== 'string') {
    return false;
  }

  const normalized = message
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .trim();

  if (!normalized) {
    return false;
  }

  // 1. Explicit separation commands
  // e.g. "separa las voces", "separar coros", "separacion vocal", "separando la pista"
  if (/\bsepara(r|d[ao]s?|ndo|cion)?\b/.test(normalized)) {
    if (/\b(voz|voces|vocal|coro|coros|instrumental|pista|cancion|audio)\b/.test(normalized)) {
      return true;
    }
  }

  // 2. Removal / extraction of vocals
  // e.g. "quitar la voz", "extraer voz", "remover voz", "aislar coros"
  if (/\b(quitar|extraer|remover|aislar|eliminar)\b.*\b(voz|voces|vocal|coro|coros)\b/.test(normalized)) {
    return true;
  }

  // 3. Instrumental intent
  // e.g. "deja la pista instrumental", "hacer instrumental", "dejarla instrumental"
  if (/\b(dejar|deja|dejarla|hacer|hacerla|volver|crear)\b.*\binstrumental\b/.test(normalized)) {
    return true;
  }

  // 4. Karaoke intent
  // e.g. "haz una version karaoke", "hacer karaoke", "pista karaoke"
  if (/\b(version|pista|hacer|haz|crear)?\s*karaoke\b/.test(normalized)) {
    return true;
  }

  // 5. Standalone / short mode selections
  // e.g. "solo coros", "modo coros", "solo instrumental"
  if (/^(solo\s+)?(coros|instrumental)\b/.test(normalized)) {
    return true;
  }

  return false;
}
