import { requestJson } from './request-json.js';

/**
 * Appends a lyrics or note item to a presentation.
 * @param {string} presentationId 
 * @param {{ type: 'lyrics' | 'note', body: string }} payload 
 * @returns {Promise<{ item: any }>}
 */
export async function addTextItem(presentationId, payload) {
  return requestJson(`/api/presentations/${encodeURIComponent(presentationId)}/items`, {
    method: 'POST',
    body: payload,
  });
}
