import { createSignal } from '../../state/create-signal.js';

/**
 * Signal tracking the latest agent-induced reload event.
 * Payload: `{ scope: 'catalog' | 'presentation' | 'events' | 'event', presentationId?: string, eventId?: string, removed?: boolean, timestamp: number } | null`
 */
export const agentRefreshSignal = createSignal(null);

const refreshListeners = new Set();

/**
 * Emits an agent-induced view refresh event.
 * @param {{ scope: 'catalog' | 'presentation' | 'events' | 'event', presentationId?: string, eventId?: string, removed?: boolean }} event
 */
export function emitAgentRefresh(event) {
  const payload = {
    scope: event.scope,
    ...(event.presentationId !== undefined ? { presentationId: event.presentationId } : {}),
    ...(event.eventId !== undefined ? { eventId: event.eventId } : {}),
    ...(event.removed !== undefined ? { removed: event.removed } : {}),
    timestamp: Date.now(),
  };

  agentRefreshSignal.set(payload);

  const eventPayload = {
    scope: event.scope,
    ...(event.presentationId !== undefined ? { presentationId: event.presentationId } : {}),
    ...(event.eventId !== undefined ? { eventId: event.eventId } : {}),
    ...(event.removed !== undefined ? { removed: event.removed } : {}),
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
 * @param {(event: { scope: 'catalog' | 'presentation' | 'events' | 'event', presentationId?: string, eventId?: string, removed?: boolean }) => void} listener
 * @returns {() => void} Unsubscribe callback.
 */
export function onAgentRefresh(listener) {
  refreshListeners.add(listener);
  return () => {
    refreshListeners.delete(listener);
  };
}
