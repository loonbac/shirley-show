import { createSignal } from '../../state/create-signal.js';

/**
 * Signal tracking the latest agent-induced reload event.
 * Payload: `{ scope: 'catalog' | 'presentation', presentationId?: string, timestamp: number } | null`
 */
export const agentRefreshSignal = createSignal(null);

const refreshListeners = new Set();

/**
 * Emits an agent-induced view refresh event.
 * @param {{ scope: 'catalog' | 'presentation', presentationId?: string }} event
 */
export function emitAgentRefresh(event) {
  const payload = {
    scope: event.scope,
    ...(event.presentationId !== undefined ? { presentationId: event.presentationId } : {}),
    timestamp: Date.now(),
  };

  agentRefreshSignal.set(payload);

  const eventPayload = {
    scope: event.scope,
    ...(event.presentationId !== undefined ? { presentationId: event.presentationId } : {}),
  };

  for (const listener of refreshListeners) {
    try {
      listener(eventPayload);
    } catch (err) {
      console.error('Agent refresh listener error:', err);
    }
  }
}

/**
 * Subscribes to agent-induced view refresh events.
 * @param {(event: { scope: 'catalog' | 'presentation', presentationId?: string }) => void} listener
 * @returns {() => void} Unsubscribe callback.
 */
export function onAgentRefresh(listener) {
  refreshListeners.add(listener);
  return () => {
    refreshListeners.delete(listener);
  };
}
