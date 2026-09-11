import { requestJson } from './request-json.js';

/**
 * Fetches all presentations from the catalog with enriched summaries.
 * @returns {Promise<{
 *   presentations: Array<{
 *     id: string,
 *     title: string,
 *     created_at: string,
 *     updated_at: string,
 *     tracks_count: number,
 *     lyrics_count: number,
 *     notes_count: number,
 *     total_items: number,
 *     preview_items: Array<{
 *       type: 'track' | 'lyrics' | 'note',
 *       id: string,
 *       position: number,
 *       title?: string,
 *       snippet?: string,
 *     }>,
 *   }>
 * }>}
 */
export async function listPresentations() {
  return requestJson('/api/presentations', {
    method: 'GET',
  });
}
