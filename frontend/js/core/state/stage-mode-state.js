import { createSignal } from '../../state/create-signal.js';

/**
 * Reactive signal indicating whether the user is currently performing in Stage Mode.
 * Used for suppressing non-essential global shortcuts like Ctrl+K.
 */
export const stageModeSignal = createSignal(false);

/**
 * Returns whether Stage Mode is currently active.
 * @returns {boolean}
 */
export function isStageModeActive() {
  return stageModeSignal.get();
}

/**
 * Sets the active state of Stage Mode.
 * @param {boolean} active
 */
export function setStageModeActive(active) {
  stageModeSignal.set(Boolean(active));
}
