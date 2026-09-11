const ALLOWED_YOUTUBE_HOSTS = new Set([
  'youtube.com',
  'www.youtube.com',
  'm.youtube.com',
  'youtu.be',
]);

const ALLOWED_PROTOCOLS = new Set(['http:', 'https:']);

/**
 * Pure utility checking whether a string contains at least one valid YouTube URL.
 * Strictly checks protocol and hostname (rejects spoofed domains like notyoutube.com).
 *
 * @param {string} text
 * @returns {boolean}
 */
export function containsYouTubeUrl(text) {
  if (typeof text !== 'string' || !text.trim()) {
    return false;
  }

  // Find candidate URLs starting with http:// or https://
  const urlMatches = text.match(/https?:\/\/[^\s<>"'`]+/gi);
  if (!urlMatches) {
    return false;
  }

  for (const candidate of urlMatches) {
    try {
      const parsed = new URL(candidate);
      if (!ALLOWED_PROTOCOLS.has(parsed.protocol.toLowerCase())) {
        continue;
      }
      const host = parsed.hostname.toLowerCase();
      if (ALLOWED_YOUTUBE_HOSTS.has(host)) {
        return true;
      }
    } catch {
      // Invalid URL candidate, skip
    }
  }

  return false;
}
