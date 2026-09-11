import { requestJson } from './request-json.js';

/**
 * Gets presentation details and its ordered show items.
 * @param {string} id 
 * @returns {Promise<{ presentation: { id: string, title: string, created_at: string, updated_at: string }, items: Array<any> }>}
 */
export async function getPresentation(id) {
  return requestJson(`/api/presentations/${encodeURIComponent(id)}`, {
    method: 'GET',
  });
}
