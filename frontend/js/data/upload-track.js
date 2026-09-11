import { requestJson } from './request-json.js';

/**
 * Uploads an audio file and appends a track item to a presentation.
 * @param {string} presentationId 
 * @param {{ title: string, file: File }} payload 
 * @returns {Promise<{ item: any }>}
 */
export async function uploadTrack(presentationId, payload) {
  const formData = new FormData();
  formData.append('title', payload.title);
  formData.append('audio', payload.file);

  return requestJson(`/api/presentations/${encodeURIComponent(presentationId)}/tracks`, {
    method: 'POST',
    body: formData,
  });
}
