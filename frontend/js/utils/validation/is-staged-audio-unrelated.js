import { containsYouTubeUrl } from './contains-youtube-url.js';

const STAGED_AUDIO_KEYWORDS = [
  'audio',
  'pista',
  'agrégalo',
  'agregalo',
  'ponlo',
  'añádelo',
  'anadelo',
  'mételo',
  'metelo',
  'guárdalo',
  'guardalo',
];

/**
 * Pure utility determining whether a user input represents an explicit unrelated command
 * that should invalidate the ephemeral stagedAudioId before dispatching the turn.
 * Conservative: preserves the capability on ambiguous messages.
 *
 * @param {string} text
 * @param {any} [attachment]
 * @returns {boolean}
 */
export function isStagedAudioUnrelated(text, attachment) {
  // 1. Any new attachment invalidates previous staged audio
  if (attachment) {
    return true;
  }

  if (typeof text !== 'string') {
    return false;
  }

  const trimmed = text.trim();
  if (!trimmed) {
    return false;
  }

  // 2. A new YouTube URL starts a new download intent
  if (containsYouTubeUrl(trimmed)) {
    return true;
  }

  const lower = trimmed.toLowerCase();

  // If the message contains references to audio / pista / follow-up, it is NOT unrelated
  if (STAGED_AUDIO_KEYWORDS.some((kw) => lower.includes(kw))) {
    return false;
  }

  // 3. Explicit unrelated actions:
  // a) Create presentation
  if (
    lower.startsWith('crea ') ||
    lower.startsWith('crear ') ||
    lower.startsWith('créame ') ||
    lower.startsWith('creame ') ||
    lower.includes('nueva presentación') ||
    lower.includes('nueva presentacion') ||
    lower.includes('nuevo show')
  ) {
    return true;
  }

  // b) List presentations
  if (
    lower.startsWith('lista ') ||
    lower.startsWith('listar ') ||
    lower.startsWith('muestra ') ||
    lower.startsWith('mostrar ') ||
    lower.includes('mis presentaciones') ||
    lower.includes('mis shows')
  ) {
    return true;
  }

  // c) Add notes or lyrics
  if (
    lower.includes('agrega nota') ||
    lower.includes('agregar nota') ||
    lower.includes('agrega la nota') ||
    lower.includes('añade nota') ||
    lower.includes('anade nota') ||
    lower.includes('agrega letra') ||
    lower.includes('agregar letra') ||
    lower.includes('agrega la letra') ||
    lower.includes('añade letra') ||
    lower.includes('anade letra')
  ) {
    return true;
  }

  return false;
}
