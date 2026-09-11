import { requestJson } from './request-json.js';

/**
 * Renames an existing presentation.
 * @param {string} id 
 * @param {{ title: string }} payload 
 * @returns {Promise<{ presentation: { id: string, title: string, created_at: string, updated_at: string } }>}
 */
export async function renamePresentation(id, payload) {
  return requestJson(`/api/presentations/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: payload,
  });
}
