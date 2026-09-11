/**
 * Headless Screen Wake Lock behavior.
 * Encapsulates acquisition of navigator.wakeLock.request('screen'),
 * silent degradation when unsupported or denied, clean release on deactivation,
 * and automatic re-acquisition on visibilitychange while active.
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

  function updateStatus(newStatus) {
    status = newStatus;
    if (typeof onChange === 'function') {
      onChange({ active: isActive, status });
    }
  }

  async function requestLock() {
    if (!('wakeLock' in navigator)) {
      updateStatus('unsupported');
      return;
    }

    try {
      sentinel = await navigator.wakeLock.request('screen');
      updateStatus('acquired');

      sentinel.addEventListener('release', () => {
        sentinel = null;
        if (isActive) {
          updateStatus('released');
        }
      });
    } catch (err) {
      // Degrade silently without modal interruption
      sentinel = null;
      updateStatus('denied');
    }
  }

  async function activate() {
    isActive = true;
    await requestLock();
  }

  async function deactivate() {
    isActive = false;
    if (sentinel) {
      try {
        await sentinel.release();
      } catch {
        // ignore
      }
      sentinel = null;
    }
    updateStatus('idle');
  }

  async function onVisibilityChange() {
    if (isActive && document.visibilityState === 'visible' && !sentinel) {
      await requestLock();
    }
  }

  document.addEventListener('visibilitychange', onVisibilityChange);

  function destroy() {
    document.removeEventListener('visibilitychange', onVisibilityChange);
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
