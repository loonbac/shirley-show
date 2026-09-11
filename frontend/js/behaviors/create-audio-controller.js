import { playbackSignal } from '../state/playback-state.js';

/**
 * Creates the single global audio controller enforcing exclusive playback.
 * NEVER autoplays. Exactly one HTMLAudioElement exists for the application lifecycle.
 */
export function createAudioController() {
  const audio = new Audio();
  audio.preload = 'metadata';

  // Event handlers updating the reactive playback state
  const onPlay = () => {
    playbackSignal.set((prev) => ({
      ...prev,
      isPlaying: true,
      error: null,
    }));
  };

  const onPause = () => {
    playbackSignal.set((prev) => ({
      ...prev,
      isPlaying: false,
    }));
  };

  const onTimeUpdate = () => {
    playbackSignal.set((prev) => ({
      ...prev,
      currentTime: audio.currentTime || 0,
    }));
  };

  const onDurationChange = () => {
    const dur = Number.isFinite(audio.duration) && audio.duration > 0 ? audio.duration : null;
    playbackSignal.set((prev) => ({
      ...prev,
      duration: dur,
    }));
  };

  const onEnded = () => {
    playbackSignal.set((prev) => ({
      ...prev,
      isPlaying: false,
      currentTime: 0,
    }));
  };

  const onError = () => {
    playbackSignal.set((prev) => ({
      ...prev,
      isPlaying: false,
      error: 'Audio no disponible o no compatible',
    }));
  };

  audio.addEventListener('play', onPlay);
  audio.addEventListener('pause', onPause);
  audio.addEventListener('timeupdate', onTimeUpdate);
  audio.addEventListener('loadedmetadata', onDurationChange);
  audio.addEventListener('durationchange', onDurationChange);
  audio.addEventListener('ended', onEnded);
  audio.addEventListener('error', onError);

  /**
   * Starts playback for a given track item.
   * If another track is active, it is immediately stopped and unloaded.
   * @param {{ id: string, audio_url?: string }} track 
   */
  function play(track) {
    if (!track || !track.id) return;

    const current = playbackSignal.get();
    const isSameTrack = current.currentTrackId === track.id;

    if (!isSameTrack) {
      // Different track: pause previous, reset time, assign new source
      audio.pause();
      audio.currentTime = 0;
      const audioUrl = track.audio_url || `/api/tracks/${track.id}/audio`;
      audio.src = audioUrl;
      audio.load();

      playbackSignal.set({
        currentTrackId: track.id,
        isPlaying: false,
        currentTime: 0,
        duration: null,
        error: null,
      });
    }

    const playPromise = audio.play();
    if (playPromise !== undefined) {
      playPromise.catch((err) => {
        console.warn('Playback error or aborted:', err);
        playbackSignal.set((prev) => ({
          ...prev,
          isPlaying: false,
          error: 'Audio no disponible o no compatible',
        }));
      });
    }
  }

  /**
   * Pauses currently active audio without resetting position.
   */
  function pause() {
    audio.pause();
  }

  /**
   * Toggles play/pause for a track.
   * @param {{ id: string, audio_url?: string }} track 
   */
  function toggle(track) {
    if (!track || !track.id) return;
    const current = playbackSignal.get();
    if (current.currentTrackId === track.id && current.isPlaying) {
      pause();
    } else {
      play(track);
    }
  }

  /**
   * Seeks to a target time in seconds.
   * @param {number} timeInSeconds 
   */
  function seek(timeInSeconds) {
    if (Number.isFinite(timeInSeconds) && timeInSeconds >= 0) {
      try {
        audio.currentTime = timeInSeconds;
        playbackSignal.set((prev) => ({
          ...prev,
          currentTime: timeInSeconds,
        }));
      } catch (err) {
        console.warn('Seek error:', err);
      }
    }
  }

  /**
   * Stops playback, releases audio src, and resets state.
   * Called on navigation, item deletion, or presentation deletion.
   */
  function stop() {
    audio.pause();
    try {
      audio.currentTime = 0;
      audio.removeAttribute('src');
      audio.load();
    } catch {
      // ignore
    }

    playbackSignal.set({
      currentTrackId: null,
      isPlaying: false,
      currentTime: 0,
      duration: null,
      error: null,
    });
  }

  /**
   * Cleans up all listeners and releases the audio element.
   */
  function destroy() {
    stop();
    audio.removeEventListener('play', onPlay);
    audio.removeEventListener('pause', onPause);
    audio.removeEventListener('timeupdate', onTimeUpdate);
    audio.removeEventListener('loadedmetadata', onDurationChange);
    audio.removeEventListener('durationchange', onDurationChange);
    audio.removeEventListener('ended', onEnded);
    audio.removeEventListener('error', onError);
  }

  return {
    play,
    pause,
    toggle,
    seek,
    stop,
    destroy,
    getAudioElement: () => audio,
  };
}
