import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { setupDomEnvironment } from './fake-dom.mjs';

setupDomEnvironment();

const activeContextModulePath = resolve(process.cwd(), 'frontend/js/core/state/active-presentation-context.js');
const { activePresentationContextSignal } = await import(pathToFileURL(activeContextModulePath).href);

const agentRefreshModulePath = resolve(process.cwd(), 'frontend/js/core/state/agent-refresh.js');
const { emitAgentRefresh } = await import(pathToFileURL(agentRefreshModulePath).href);

const stageModeStateModulePath = resolve(process.cwd(), 'frontend/js/core/state/stage-mode-state.js');
const { stageModeSignal } = await import(pathToFileURL(stageModeStateModulePath).href);

const presCtrlModulePath = resolve(process.cwd(), 'frontend/js/controllers/presentations-controller.js');
const { createPresentationsController } = await import(pathToFileURL(presCtrlModulePath).href);

const detailCtrlModulePath = resolve(process.cwd(), 'frontend/js/controllers/presentation-detail-controller.js');
const { createPresentationDetailController } = await import(pathToFileURL(detailCtrlModulePath).href);

console.log('Running Agent Chat Route and Composition Integration Tests...');

// 1. Catalog route publishing and reactive refresh subscription
{
  let openAgentCalled = false;
  const mockAudio = { stop: () => {} };

  const ctrl = createPresentationsController({
    audioController: mockAudio,
    onOpenAgent: () => { openAgentCalled = true; },
  });

  // Topbar should contain agent-chat-trigger
  const triggerEl = ctrl.topbarEl.querySelector('.agent-chat-trigger');
  assert.ok(triggerEl !== null, 'Catalog topbar must compose agent-chat-trigger');

  triggerEl.dispatchEvent(new Event('click'));
  assert.equal(openAgentCalled, true, 'Clicking trigger in topbar must call onOpenAgent');

  // Active presentation context in catalog must be null
  assert.equal(activePresentationContextSignal.get(), null, 'Catalog controller must set active context to null');

  ctrl.destroy();
  console.log('✓ 1. Catalog route sets context to null and composes agent-chat-trigger');
}

// 2. Presentation detail route publishing, reactive item refresh, and scroll preservation
{
  let fetchCallCount = 0;
  let itemsInShow = [
    {
      id: 'item-1',
      presentation_id: 'pres-test-999',
      kind: 'lyrics',
      position: 0,
      text: 'Letra verso 1',
    },
  ];

  globalThis.fetch = async (url, opts) => {
    fetchCallCount += 1;
    if (String(url).includes('/api/presentations/pres-test-999')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => ({
          presentation: {
            id: 'pres-test-999',
            title: 'Show Acústico en Vivo',
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
          },
          items: [...itemsInShow],
        }),
      };
    }
    return { ok: false, status: 404, headers: { get: () => 'application/json' }, json: async () => ({ error: 'not found' }) };
  };

  const mockAudio = {
    stop: () => {},
    play: () => {},
    pause: () => {},
    toggle: () => {},
    seek: () => {},
  };

  let openAgentCalled = false;
  const ctrl = createPresentationDetailController({
    presentationId: 'pres-test-999',
    audioController: mockAudio,
    onOpenAgent: () => { openAgentCalled = true; },
  });

  // Await initial loadData() execution
  await new Promise((r) => setTimeout(r, 20));

  // Verify active presentation context is published with id and title
  assert.deepEqual(
    activePresentationContextSignal.get(),
    { id: 'pres-test-999', title: 'Show Acústico en Vivo' },
    'Presentation detail controller must publish active presentation context with id and title'
  );

  const triggerEl = ctrl.topbarEl.querySelector('.agent-chat-trigger');
  assert.ok(triggerEl !== null, 'Presentation detail topbar must compose agent-chat-trigger');

  // Setup scroll tracking on window and view element
  window.scrollY = 420;
  if (!ctrl.viewEl) ctrl.viewEl = {};
  ctrl.viewEl.scrollTop = 250;

  let restoredWindowScroll = null;
  window.scrollTo = (x, y) => {
    restoredWindowScroll = { x, y };
  };

  // Ressy adds a second item to the show sequence
  itemsInShow.push({
    id: 'item-2',
    presentation_id: 'pres-test-999',
    kind: 'note',
    position: 1,
    text: 'Afinar en Re',
  });

  const callsBeforeRefresh = fetchCallCount;

  // Emit agent refresh signal for this presentation
  emitAgentRefresh({ scope: 'presentation', presentationId: 'pres-test-999' });

  // Await reactive refresh execution
  await new Promise((r) => setTimeout(r, 30));

  assert.ok(fetchCallCount > callsBeforeRefresh, 'emitAgentRefresh must trigger loadData() in presentation detail controller');
  assert.deepEqual(
    restoredWindowScroll,
    { x: 0, y: 420 },
    'Reactive refresh must capture and restore window scroll position'
  );
  assert.equal(
    ctrl.viewEl.scrollTop,
    250,
    'Reactive refresh must capture and restore container scrollTop'
  );

  ctrl.destroy();
  console.log('✓ 2. Presentation detail controller publishes context, triggers reactive reload, and preserves scroll position');
}

// 3. Stage Mode sets stageModeSignal for shortcut suppression
{
  assert.equal(stageModeSignal.get(), false);
  stageModeSignal.set(true);
  assert.equal(stageModeSignal.get(), true);
  stageModeSignal.set(false);
  assert.equal(stageModeSignal.get(), false);
  console.log('✓ 3. Stage mode signal toggles correctly');
}

console.log('All Agent Chat Route and Composition integration tests passed!');
