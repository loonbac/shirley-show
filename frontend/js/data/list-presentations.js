import { requestJson } from './request-json.js';

/**
 * Fetches all presentations from the catalog.
 * @returns {Promise<{ presentations: Array<{ id: string, title: string, created_at: string, updated_at: string }> }>}
 */
export async function listPresentations() {
  return requestJson('/api/presentations', {
    method: 'GET',
  });
}
