import { requestJson } from './request-json.js';

/**
 * Moves an item up or down in the sequence.
 * @param {string} presentationId 
 * @param {string} itemId 
 * @param {'up' | 'down'} direction 
 * @returns {Promise<{ items: Array<any> }>}
 */
export async function moveItem(presentationId, itemId, direction) {
  return requestJson(
    `/api/presentations/${encodeURIComponent(presentationId)}/items/${encodeURIComponent(itemId)}/move`,
    {
      method: 'POST',
      body: { direction },
    }
  );
}
