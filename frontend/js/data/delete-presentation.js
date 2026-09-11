import { requestJson } from './request-json.js';

/**
 * Deletes a presentation and triggers cascade cleanup of its items and audio.
 * @param {string} id 
 * @returns {Promise<{ deleted_id: string, cleanup_pending: boolean }>}
 */
export async function deletePresentation(id) {
  return requestJson(`/api/presentations/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
