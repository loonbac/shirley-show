/**
 * Pure utility to format ISO timestamps into warm human-readable Spanish text.
 * Unknown or invalid dates return a neutral fallback string ("Fecha no disponible").
 *
 * @param {string|number|Date|null|undefined} isoDate 
 * @param {Date} [now=new Date()] Optional reference date for testing
 * @returns {string}
 */
export function formatRelativeDate(isoDate, now = new Date()) {
  if (!isoDate) {
    return 'Fecha no disponible';
  }

  const date = isoDate instanceof Date ? isoDate : new Date(isoDate);

  if (isNaN(date.getTime())) {
    return 'Fecha no disponible';
  }

  const nowDate = now instanceof Date ? now : new Date(now);
  if (isNaN(nowDate.getTime())) {
    return 'Fecha no disponible';
  }

  // Same calendar day check
  const isSameDay =
    date.getFullYear() === nowDate.getFullYear() &&
    date.getMonth() === nowDate.getMonth() &&
    date.getDate() === nowDate.getDate();

  if (isSameDay) {
    return 'Actualizado hoy';
  }

  // Yesterday check
  const yesterday = new Date(nowDate);
  yesterday.setDate(yesterday.getDate() - 1);
  const isYesterday =
    date.getFullYear() === yesterday.getFullYear() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getDate() === yesterday.getDate();

  if (isYesterday) {
    return 'Actualizado ayer';
  }

  const diffMs = nowDate.getTime() - date.getTime();
  if (diffMs < 0) {
    // Slight future timestamp (e.g. clock drift)
    return 'Actualizado hoy';
  }

  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays <= 0) {
    return 'Actualizado hoy';
  }

  if (diffDays === 1) {
    return 'Actualizado ayer';
  }

  if (diffDays < 7) {
    return `Hace ${diffDays} días`;
  }

  if (diffDays < 14) {
    return 'Hace 1 semana';
  }

  if (diffDays < 30) {
    const weeks = Math.floor(diffDays / 7);
    return `Hace ${weeks} semanas`;
  }

  if (diffDays < 60) {
    return 'Hace 1 mes';
  }

  if (diffDays < 365) {
    const months = Math.floor(diffDays / 30);
    return `Hace ${months} meses`;
  }

  const years = Math.floor(diffDays / 365);
  return `Hace ${years} ${years === 1 ? 'año' : 'años'}`;
}
