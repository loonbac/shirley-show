import { requestJson } from './request-json.js';

/**
 * Creates a new presentation in the catalog.
 * @param {{ title: string }} payload 
 * @returns {Promise<{ presentation: { id: string, title: string, created_at: string, updated_at: string } }>}
 */
export async function createPresentation(payload) {
  return requestJson('/api/presentations', {
    method: 'POST',
    body: payload,
  });
}
