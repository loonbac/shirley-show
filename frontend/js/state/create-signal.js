/**
 * Creates an atomic reactive signal.
 * @template T
 * @param {T} initialValue 
 * @returns {{ get: () => T, set: (next: T | ((prev: T) => T)) => void, subscribe: (listener: (val: T) => void) => () => void }}
 */
export function createSignal(initialValue) {
  let value = initialValue;
  const listeners = new Set();

  function get() {
    return value;
  }

  function set(next) {
    const nextValue = typeof next === 'function' ? next(value) : next;
    if (nextValue !== value) {
      value = nextValue;
      for (const listener of listeners) {
        try {
          listener(value);
        } catch (err) {
          console.error('Signal listener error:', err);
        }
      }
    }
  }

  function subscribe(listener) {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }

  return { get, set, subscribe };
}
