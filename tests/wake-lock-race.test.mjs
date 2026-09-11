import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

// Mock DOM and Navigator environment
class FakeDocument extends EventTarget {
  visibilityState = 'visible';
}

class FakeSentinel extends EventTarget {
  released = false;
  async release() {
    this.released = true;
    this.dispatchEvent(new Event('release'));
  }
}

globalThis.document = new FakeDocument();

const modulePath = resolve(process.cwd(), 'frontend/js/behaviors/use-wake-lock.js');
const { useWakeLock } = await import(pathToFileURL(modulePath).href);

console.log('Running Wake-Lock Behavioral Regression Tests...');

// Test 1: Deactivate during pending request (generation race)
{
  let resolveRequest;
  const sentinel = new FakeSentinel();
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      wakeLock: {
        request: () => new Promise((res) => { resolveRequest = res; }),
      },
    },
  });

  const wake = useWakeLock();
  const activation = wake.activate();

  // Deactivate before request resolves
  await wake.deactivate();
  assert.equal(wake.isActive(), false, 'isActive must be false after deactivate');
  assert.equal(wake.getStatus(), 'idle', 'status must be idle after deactivate');

  // Request resolves post-deactivation
  resolveRequest(sentinel);
  await activation;

  // Stale resolution must release sentinel immediately and never mark acquired
  assert.equal(wake.isActive(), false, 'isActive must remain false after stale resolution');
  assert.equal(wake.getStatus(), 'idle', 'status must not transition to acquired when inactive');
  assert.equal(sentinel.released, true, 'sentinel must be released immediately upon resolving post-deactivation');

  wake.destroy();
  console.log('✓ Test 1: In-flight acquisition released immediately on post-deactivation resolution');
}

// Test 2: Standard acquire and release lifecycle
{
  const sentinel = new FakeSentinel();
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      wakeLock: {
        request: async () => sentinel,
      },
    },
  });

  let lastChange = null;
  const wake = useWakeLock({
    onChange: (state) => { lastChange = state; },
  });

  await wake.activate();
  assert.equal(wake.isActive(), true);
  assert.equal(wake.getStatus(), 'acquired');
  assert.deepEqual(lastChange, { active: true, status: 'acquired' });
  assert.equal(sentinel.released, false);

  await wake.deactivate();
  assert.equal(wake.isActive(), false);
  assert.equal(wake.getStatus(), 'idle');
  assert.deepEqual(lastChange, { active: false, status: 'idle' });
  assert.equal(sentinel.released, true);

  wake.destroy();
  console.log('✓ Test 2: Standard acquire and release lifecycle transitions correctly');
}

// Test 3: Unsupported wakeLock degrades silently
{
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {},
  });

  const wake = useWakeLock();
  await wake.activate();
  assert.equal(wake.isActive(), true);
  assert.equal(wake.getStatus(), 'unsupported');

  await wake.deactivate();
  assert.equal(wake.isActive(), false);
  assert.equal(wake.getStatus(), 'idle');

  wake.destroy();
  console.log('✓ Test 3: Unsupported navigator.wakeLock degrades silently to unsupported');
}

// Test 4: Denied/rejected wakeLock degrades silently
{
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      wakeLock: {
        request: async () => { throw new Error('NotAllowedError'); },
      },
    },
  });

  const wake = useWakeLock();
  await wake.activate();
  assert.equal(wake.isActive(), true);
  assert.equal(wake.getStatus(), 'denied');

  await wake.deactivate();
  assert.equal(wake.isActive(), false);
  assert.equal(wake.getStatus(), 'idle');

  wake.destroy();
  console.log('✓ Test 4: Denied permission degrades silently without throwing');
}

// Test 5: External release and visibilitychange re-acquisition
{
  let sentinelCount = 0;
  const sentinels = [new FakeSentinel(), new FakeSentinel()];
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      wakeLock: {
        request: async () => sentinels[sentinelCount++],
      },
    },
  });

  const wake = useWakeLock();
  await wake.activate();
  assert.equal(wake.getStatus(), 'acquired');
  assert.equal(sentinels[0].released, false);

  // System/browser releases lock externally while active
  await sentinels[0].release();
  assert.equal(wake.isActive(), true);
  assert.equal(wake.getStatus(), 'released');

  // Visibility changes back to visible -> auto re-acquires
  globalThis.document.visibilityState = 'visible';
  globalThis.document.dispatchEvent(new Event('visibilitychange'));

  // Yield microtasks for async handler
  await new Promise((resolve) => setTimeout(resolve, 10));

  assert.equal(wake.isActive(), true);
  assert.equal(wake.getStatus(), 'acquired');
  assert.equal(sentinels[1].released, false);

  wake.destroy();
  assert.equal(sentinels[1].released, true);
  console.log('✓ Test 5: External release detected and reacquired on visibilitychange');
}

console.log('All 5 wake-lock behavioral tests passed cleanly!');
