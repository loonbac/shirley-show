/**
 * Formats a byte count into a human-readable string (KB / MB).
 * 
 * @param {number} bytes - Total number of bytes.
 * @returns {string} Formatted size string (e.g., '1.0 KB', '5.4 MB') or '' if invalid/empty.
 */
export function formatBytes(bytes) {
  if (!bytes || bytes <= 0 || !Number.isFinite(bytes)) return '';
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
