import { requestJson } from './request-json.js';

/**
 * Updates an item's title (track) or text body (lyrics/note) via PATCH.
 * @param {string} presentationId 
 * @param {string} itemId 
 * @param {{ title?: string, body?: string }} payload 
 * @returns {Promise<any>} The updated ShowItemResponse
 */
export async function updateItem(presentationId, itemId, payload) {
  return requestJson(
    `/api/presentations/${encodeURIComponent(presentationId)}/items/${encodeURIComponent(itemId)}`,
    {
      method: 'PATCH',
      body: payload,
    }
  );
}
