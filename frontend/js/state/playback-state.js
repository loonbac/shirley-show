import { createSignal } from './create-signal.js';

/**
 * Reactive signal for audio playback state across the application.
 * Exclusive playback invariant: at most ONE audio track is playing.
 * Shape: {
 *   currentTrackId: string | null,
 *   isPlaying: boolean,
 *   currentTime: number,
 *   duration: number | null,
 *   error: string | null,
 * }
 */
export const playbackSignal = createSignal({
  currentTrackId: null,
  isPlaying: false,
  currentTime: 0,
  duration: null,
  error: null,
});
