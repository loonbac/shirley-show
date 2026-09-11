/**
 * Headless Screen Wake Lock behavior.
 * Encapsulates acquisition of navigator.wakeLock.request('screen'),
 * silent degradation when unsupported or denied, clean release on deactivation,
 * and automatic re-acquisition on visibilitychange while active.
 *
 * Generation-aware: if deactivate() runs while an acquisition is in flight,
 * any sentinel resolving later is released immediately, and status never
 * transitions to 'acquired' while active is false.
 *
 * @param {{
 *   onChange?: (state: { active: boolean, status: 'idle' | 'acquired' | 'unsupported' | 'denied' | 'released' }) => void,
 * }} [options]
 * @returns {{
 *   activate: () => Promise<void>,
 *   deactivate: () => Promise<void>,
 *   destroy: () => void,
 *   getStatus: () => 'idle' | 'acquired' | 'unsupported' | 'denied' | 'released',
 *   isActive: () => boolean,
 * }}
 */
export function useWakeLock(options = {}) {
  const { onChange } = options;

  let isActive = false;
  let sentinel = null;
  let status = 'idle'; // 'idle' | 'acquired' | 'unsupported' | 'denied' | 'released'
  let generation = 0;

  function updateStatus(newStatus) {
    status = newStatus;
    if (typeof onChange === 'function') {
      onChange({ active: isActive, status });
    }
  }

  async function requestLock(targetGen) {
    if (typeof navigator === 'undefined' || !('wakeLock' in navigator)) {
      if (targetGen === generation && isActive) {
        updateStatus('unsupported');
      }
      return;
    }

    try {
      const acquiredSentinel = await navigator.wakeLock.request('screen');

      // Generation check: if deactivated or newer activation ran while waiting
      if (targetGen !== generation || !isActive) {
        try {
          await acquiredSentinel.release();
        } catch {
          // ignore release error on discarded sentinel
        }
        return;
      }

      sentinel = acquiredSentinel;
      updateStatus('acquired');

      acquiredSentinel.addEventListener('release', () => {
        if (sentinel === acquiredSentinel) {
          sentinel = null;
        }
        if (isActive && targetGen === generation) {
          updateStatus('released');
        }
      });
    } catch (err) {
      // Degrade silently without modal interruption
      if (targetGen === generation && isActive) {
        sentinel = null;
        updateStatus('denied');
      }
    }
  }

  async function activate() {
    if (isActive && sentinel) {
      return;
    }
    isActive = true;
    generation += 1;
    const targetGen = generation;
    await requestLock(targetGen);
  }

  async function deactivate() {
    isActive = false;
    generation += 1;
    if (sentinel) {
      const s = sentinel;
      sentinel = null;
      try {
        await s.release();
      } catch {
        // ignore
      }
    }
    updateStatus('idle');
  }

  async function onVisibilityChange() {
    if (isActive && typeof document !== 'undefined' && document.visibilityState === 'visible' && !sentinel) {
      generation += 1;
      const targetGen = generation;
      await requestLock(targetGen);
    }
  }

  if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
    document.addEventListener('visibilitychange', onVisibilityChange);
  }

  function destroy() {
    if (typeof document !== 'undefined' && typeof document.removeEventListener === 'function') {
      document.removeEventListener('visibilitychange', onVisibilityChange);
    }
    deactivate();
  }

  return {
    activate,
    deactivate,
    destroy,
    getStatus: () => status,
    isActive: () => isActive,
  };
}
