import { requestJson } from './request-json.js';

/**
 * Deletes an item from a presentation sequence and compacts indices.
 * @param {string} presentationId 
 * @param {string} itemId 
 * @returns {Promise<{ deleted_id: string, items: Array<any>, cleanup_pending: boolean }>}
 */
export async function deleteItem(presentationId, itemId) {
  return requestJson(
    `/api/presentations/${encodeURIComponent(presentationId)}/items/${encodeURIComponent(itemId)}`,
    {
      method: 'DELETE',
    }
  );
}
