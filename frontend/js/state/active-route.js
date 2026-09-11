import { createSignal } from './create-signal.js';

/**
 * Reactive signal for current client route.
 * Shape: { view: 'presentations' | 'presentation-detail', params: { id?: string }, error?: string }
 */
export const activeRouteSignal = createSignal({
  view: 'presentations',
  params: {},
  error: null,
});
