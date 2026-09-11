import { createSignal } from '../../state/create-signal.js';

/**
 * Reactive signal for the currently active presentation context in the application.
 * Value is `{ id: string, title: string } | null`.
 */
export const activePresentationContextSignal = createSignal(null);
