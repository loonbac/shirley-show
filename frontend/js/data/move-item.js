import { requestJson } from './request-json.js';

/**
 * Moves an item in the presentation sequence, either by direction ('up' | 'down')
 * or by absolute target position (number).
 *
 * @param {string} presentationId 
 * @param {string} itemId 
 * @param {'up' | 'down' | number | { direction?: 'up' | 'down', target_position?: number, targetPosition?: number }} selector
 * @param {number} [targetPosition]
 * @returns {Promise<{ items: Array<any> }>}
 */
export async function moveItem(presentationId, itemId, selector, targetPosition) {
  let body = {};

  if (typeof selector === 'object' && selector !== null) {
    if (selector.direction !== undefined) {
      body.direction = selector.direction;
    }
    if (selector.target_position !== undefined) {
      body.target_position = selector.target_position;
    } else if (selector.targetPosition !== undefined) {
      body.target_position = selector.targetPosition;
    }
  } else if (typeof selector === 'string') {
    body = { direction: selector };
  } else if (typeof selector === 'number') {
    body = { target_position: selector };
  } else if (typeof targetPosition === 'number') {
    body = { target_position: targetPosition };
  }

  return requestJson(
    `/api/presentations/${encodeURIComponent(presentationId)}/items/${encodeURIComponent(itemId)}/move`,
    {
      method: 'POST',
      body,
    }
  );
}
