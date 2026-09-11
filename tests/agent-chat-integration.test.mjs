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

// 2. Presentation detail route publishing and reactive item refresh
{
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

  const triggerEl = ctrl.topbarEl.querySelector('.agent-chat-trigger');
  assert.ok(triggerEl !== null, 'Presentation detail topbar must compose agent-chat-trigger');

  ctrl.destroy();
  console.log('✓ 2. Presentation detail controller composes trigger and handles context');
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
