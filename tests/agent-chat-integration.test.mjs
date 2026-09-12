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

const chatStateModulePath = resolve(process.cwd(), 'frontend/js/core/state/chat-state.js');
const { createChatState } = await import(pathToFileURL(chatStateModulePath).href);

const chatControllerModulePath = resolve(process.cwd(), 'frontend/js/controllers/agent-chat-controller.js');
const { createAgentChatController } = await import(pathToFileURL(chatControllerModulePath).href);

const chatPanelModulePath = resolve(process.cwd(), 'frontend/js/components/organisms/agent-chat-panel.js');
const { createAgentChatPanel } = await import(pathToFileURL(chatPanelModulePath).href);

const agentChatApiModulePath = resolve(process.cwd(), 'frontend/js/data/agent-chat-api.js');
const { sendAgentChatMessage } = await import(pathToFileURL(agentChatApiModulePath).href);

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

// 4. TRIANGULATE: Network error preserves stagedAudioId for retry
{
  const state = createChatState({
    stagedAudioId: 'staged-yt-preserve-123',
  });

  let attemptedPayload = null;
  globalThis.fetch = async (url, opts) => {
    attemptedPayload = JSON.parse(opts.body);
    throw new Error('Failed to fetch'); // Network error simulation
  };

  const ctrl = createAgentChatController({ chatState: state });
  state.open();
  state.setDraft('Reintentar agregar pista');

  // Send fails
  await ctrl.sendAndOpen('Reintentar agregar pista');

  // 1. Pending is cleared
  assert.equal(state.get().pending, false);
  // 2. stagedAudioId MUST be preserved for safe retry!
  assert.equal(
    state.get().stagedAudioId,
    'staged-yt-preserve-123',
    'stagedAudioId must be preserved across network errors'
  );
  // 3. Sent payload forwarded the staged_audio_id
  assert.equal(attemptedPayload.staged_audio_id, 'staged-yt-preserve-123');

  ctrl.destroy();
  console.log('✓ 4. Network error preserves stagedAudioId for retry');
}

// 5. TRIANGULATE: Teardown clears typing indicator and restores composer
{
  const panel = createAgentChatPanel({
    isOpen: true,
    activeContext: null,
    messages: [{ role: 'user', content: 'Descarga https://www.youtube.com/watch?v=dQw4w9WgXcQ' }],
    pending: true,
    pendingLabel: 'Ressy está descargando el audio…',
    draft: 'Borrador',
    onClose: () => {},
    onSend: () => {},
  });

  // While pending: indicator present and textarea disabled
  assert.ok(panel.transcriptEl.querySelector('.agent-typing-indicator') !== null);
  assert.equal(panel.composer.textareaEl.hasAttribute('disabled'), true);

  // Teardown / completion removes indicator and restores composer
  panel.update({ pending: false });
  assert.equal(panel.transcriptEl.querySelector('.agent-typing-indicator'), null);
  assert.equal(panel.composer.textareaEl.hasAttribute('disabled'), false);

  panel.destroy();
  console.log('✓ 5. Teardown clears typing indicator and restores composer');
}

// 6. TRIANGULATE: Spoofed or similar host (notyoutube.com) retains general label
{
  const state = createChatState();
  const turn = state.beginTurn({
    message: 'Descarga https://notyoutube.com/watch?v=fake1234567',
  });

  // General thinking label must be retained, NOT "descargando el audio…"
  assert.notEqual(state.get().pendingLabel, 'Ressy está descargando el audio…');
  assert.ok(
    state.get().pendingLabel.includes('pensando'),
    'Messages with similar or non-youtube hosts must keep general thinking label'
  );

  state.cancelTurn();
  console.log('✓ 6. Spoofed or non-youtube host retains general thinking label');
}

// 7. REGRESSION: Unrelated command clears staged capability and does NOT forward stale ID
{
  const state = createChatState({ stagedAudioId: 'staged-old-cap-to-invalidate' });
  let interceptedPayload = null;

  globalThis.fetch = async (_url, opts) => {
    interceptedPayload = JSON.parse(opts.body);
    return {
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      json: async () => ({ message: 'ok', actions_taken: [] }),
    };
  };

  const ctrl = createAgentChatController({ chatState: state });
  await ctrl.sendAndOpen('Crea una presentación para el show');

  assert.equal(
    interceptedPayload.staged_audio_id,
    null,
    'Stale staged_audio_id must be null in dispatch payload for unrelated command'
  );
  assert.equal(
    state.get().stagedAudioId,
    null,
    'Chat state stagedAudioId must be cleared'
  );

  ctrl.destroy();
  console.log('✓ 7. Unrelated command invalidates capability and sends null staged_audio_id');
}

// 8. REGRESSION: 3s timer typing indicator specializes only for YouTube URLs
{
  // 8a. Non-YouTube slow command retains generic thinking label after 3.1s
  {
    const state = createChatState();
    let resolveFetch;
    globalThis.fetch = () =>
      new Promise((res) => {
        resolveFetch = () =>
          res({
            ok: true,
            status: 200,
            headers: { get: () => 'application/json' },
            json: async () => ({ message: 'ok', actions_taken: [] }),
          });
      });

    const ctrl = createAgentChatController({ chatState: state });
    const pendingPromise = ctrl.sendAndOpen('Lista mis presentaciones');

    // Wait >3s to let timer fire
    await new Promise((r) => setTimeout(r, 3100));

    const typingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
    assert.ok(typingEl !== null, 'Typing indicator must be present while pending');
    assert.equal(
      typingEl.getAttribute('aria-label'),
      'Ressy está pensando...',
      'Non-YouTube slow request must retain generic label'
    );
    assert.ok(
      !typingEl.textContent.includes('descargando el audio'),
      'Non-YouTube slow request must never show download label'
    );

    resolveFetch();
    await pendingPromise;
    ctrl.destroy();
  }

  // 8b. YouTube slow command shows specialized download label after 3.1s
  {
    const state = createChatState();
    let resolveFetch;
    globalThis.fetch = () =>
      new Promise((res) => {
        resolveFetch = () =>
          res({
            ok: true,
            status: 200,
            headers: { get: () => 'application/json' },
            json: async () => ({ message: 'ok', actions_taken: [] }),
          });
      });

    const ctrl = createAgentChatController({ chatState: state });
    const pendingPromise = ctrl.sendAndOpen(
      'Descarga https://www.youtube.com/watch?v=dQw4w9WgXcQ'
    );

    await new Promise((r) => setTimeout(r, 3100));

    const typingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
    assert.ok(typingEl !== null, 'Typing indicator must be present while pending');
    assert.equal(
      typingEl.getAttribute('aria-label'),
      'Ressy está descargando el audio…',
      'YouTube slow request must show download label'
    );
    assert.ok(
      typingEl.textContent.includes('descargando el audio…'),
      'YouTube slow request must include download label text'
    );

    resolveFetch();
    await pendingPromise;
    ctrl.destroy();
  }

  console.log('✓ 8. 3s timer typing indicator specializes only for YouTube URLs');
}

// 9. Vocal separation intent displays specialized typing indicator and restores composer
{
  const state = createChatState();
  let resolveFetch;
  globalThis.fetch = (url) => {
    if (typeof url === 'string' && url.includes('/api/agent/gpu-status')) {
      return Promise.resolve({
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => ({ mode: 'local', pc_online: true }),
      });
    }
    return new Promise((res) => {
      resolveFetch = () =>
        res({
          ok: true,
          status: 200,
          headers: { get: () => 'application/json' },
          json: async () => ({
            message: 'Separación terminada.',
            actions_taken: [{ tool: 'separate_vocals', status: 'success' }],
            staged_audio_id: 'staged-int-vsep-999',
          }),
        });
    });
  };

  const ctrl = createAgentChatController({ chatState: state });
  const pendingPromise = ctrl.sendAndOpen('Separa los coros de esta pista');

  const typingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
  assert.ok(typingEl !== null, 'Typing indicator must be present while pending');
  assert.equal(typingEl.getAttribute('aria-live'), 'polite', 'Must have aria-live=polite');
  assert.ok(
    typingEl.getAttribute('aria-label').includes('separando voces…'),
    `aria-label must include 'separando voces…', got: ${typingEl.getAttribute('aria-label')}`
  );
  assert.ok(
    typingEl.textContent.includes('separando voces…'),
    `textContent must include 'separando voces…', got: ${typingEl.textContent}`
  );

  // After resolution: typing indicator removed, composer enabled
  await new Promise((r) => setTimeout(r, 10));
  resolveFetch();
  await pendingPromise;

  const postTypingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
  assert.equal(postTypingEl, null, 'Typing indicator must be removed after resolution');
  const textarea = ctrl.panelEl.querySelector('.agent-composer-textarea');
  assert.equal(textarea.hasAttribute('disabled'), false, 'Composer must be re-enabled');

  ctrl.destroy();
  console.log('✓ 9. Vocal separation intent displays specialized typing indicator and restores composer');
}

// 10. Vocal separation intent probes GPU status (offline -> espera; online -> genérico; local -> genérico)
{
  // 10a. Remote offline: displays 'esperando la PC de Joshua…'
  {
    const state = createChatState();
    let resolveTurnFetch;
    const sendPromise = new Promise((resolve) => {
      resolveTurnFetch = () =>
        resolve({
          message: 'Completado',
          actions_taken: [{ tool: 'separate_vocals', status: 'success' }],
          staged_audio_id: 'staged-sep-1',
        });
    });

    const ctrl = createAgentChatController({
      chatState: state,
      getGpuStatusFn: async () => ({ mode: 'remote', pc_online: false }),
      sendAgentChatMessageFn: () => sendPromise,
    });

    const pendingPromise = ctrl.sendAndOpen('Separa la voz de esta pista');
    await new Promise((r) => setTimeout(r, 10)); // allow async handleSend to complete probe

    const typingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
    assert.ok(typingEl !== null, 'Typing indicator must be present');
    assert.equal(typingEl.getAttribute('aria-live'), 'polite');
    assert.ok(
      typingEl.textContent.includes('esperando la PC de Joshua…'),
      `Expected 'esperando la PC de Joshua…', got: ${typingEl.textContent}`
    );

    resolveTurnFetch();
    await pendingPromise;
    ctrl.destroy();
  }

  // 10b. Remote online: displays 'separando voces…'
  {
    const state = createChatState();
    let resolveTurnFetch;
    const sendPromise = new Promise((resolve) => {
      resolveTurnFetch = () =>
        resolve({
          message: 'Completado',
          actions_taken: [{ tool: 'separate_vocals', status: 'success' }],
          staged_audio_id: 'staged-sep-2',
        });
    });

    const ctrl = createAgentChatController({
      chatState: state,
      getGpuStatusFn: async () => ({ mode: 'remote', pc_online: true }),
      sendAgentChatMessageFn: () => sendPromise,
    });

    const pendingPromise = ctrl.sendAndOpen('Separa coros');
    await new Promise((r) => setTimeout(r, 10));

    const typingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
    assert.ok(typingEl !== null);
    assert.ok(
      typingEl.textContent.includes('separando voces…'),
      `Expected 'separando voces…', got: ${typingEl.textContent}`
    );

    resolveTurnFetch();
    await pendingPromise;
    ctrl.destroy();
  }

  // 10c. Local mode: displays 'separando voces…'
  {
    const state = createChatState();
    let resolveTurnFetch;
    const sendPromise = new Promise((resolve) => {
      resolveTurnFetch = () =>
        resolve({
          message: 'Completado',
          actions_taken: [{ tool: 'separate_vocals', status: 'success' }],
          staged_audio_id: 'staged-sep-3',
        });
    });

    const ctrl = createAgentChatController({
      chatState: state,
      getGpuStatusFn: async () => ({ mode: 'local', pc_online: true }),
      sendAgentChatMessageFn: () => sendPromise,
    });

    const pendingPromise = ctrl.sendAndOpen('Separa pista instrumental');
    await new Promise((r) => setTimeout(r, 10));

    const typingEl = ctrl.panelEl.querySelector('.agent-typing-indicator');
    assert.ok(typingEl !== null);
    assert.ok(
      typingEl.textContent.includes('separando voces…'),
      `Expected 'separando voces…', got: ${typingEl.textContent}`
    );

    resolveTurnFetch();
    await pendingPromise;
    ctrl.destroy();
  }

  console.log('✓ 10. GPU status probe integration in controller (offline -> espera; online -> genérico; local -> genérico)');
}

// 11. Event context travels as event_id, not as presentation_id, to sendAgentChatMessage
{
  activePresentationContextSignal.set({
    id: 'evt-test-777',
    title: 'Festival Criollo',
    type: 'event',
  });

  const state = createChatState();

  let capturedOptions = null;
  const ctrl = createAgentChatController({
    chatState: state,
    sendAgentChatMessageFn: async (options) => {
      capturedOptions = options;
      return {
        message: 'Respuesta de evento',
        actions_taken: [],
        active_presentation_id: null,
        active_event_id: 'evt-test-777',
        notice: null,
      };
    },
  });

  await ctrl.sendAndOpen('¿Cuántas entradas se han vendido?');

  assert.ok(capturedOptions, 'sendAgentChatMessageFn must be invoked');
  assert.equal(capturedOptions.eventId, 'evt-test-777', 'Event context id must travel as eventId');
  assert.equal(capturedOptions.presentationId, null, 'Event context id must NOT travel as presentationId');

  ctrl.destroy();

  // Also test sendAgentChatMessage building JSON payload with event_id
  let sentBody = null;
  globalThis.fetch = async (url, opts) => {
    sentBody = JSON.parse(opts.body);
    return {
      ok: true,
      status: 200,
      headers: { get: () => 'application/json' },
      json: async () => ({
        message: 'OK',
        actions_taken: [],
        active_presentation_id: null,
        active_event_id: 'evt-test-777',
        notice: null,
      }),
    };
  };

  await sendAgentChatMessage({
    message: 'Hola Ressy',
    eventId: 'evt-test-777',
  });

  assert.equal(sentBody.event_id, 'evt-test-777', 'event_id must be sent in JSON payload');
  assert.equal(sentBody.presentation_id, null, 'presentation_id must be null for event context');

  console.log('✓ 11. Event context travels as event_id (not presentation_id) to agent chat API');
}

// 12. Browser preserves active_event_id from backend response and sends it on subsequent turns
{
  activePresentationContextSignal.set(null);

  const state = createChatState();
  assert.equal(state.get().activeContext, null, 'Initial activeContext must be null');

  let callCount = 0;
  let capturedCalls = [];

  const ctrl = createAgentChatController({
    chatState: state,
    sendAgentChatMessageFn: async (options) => {
      callCount += 1;
      capturedCalls.push(options);
      if (callCount === 1) {
        return {
          message: 'Evento creado exitosamente',
          actions_taken: [
            {
              tool: 'crear_evento',
              event_id: 'evt-rec-999',
              details: { nombre: 'Peña Criolla Barranco' },
            },
          ],
          active_presentation_id: null,
          active_event_id: 'evt-rec-999',
          notice: null,
        };
      }
      return {
        message: 'Reporte de entradas',
        actions_taken: [],
        active_presentation_id: null,
        active_event_id: 'evt-rec-999',
        notice: null,
      };
    },
  });

  // Turn 1: User asks to create an event without pre-existing context
  await ctrl.sendAndOpen('Crear peña criolla');

  const contextAfterTurn1 = state.get().activeContext;
  assert.ok(contextAfterTurn1, 'activeContext must be populated after receiving active_event_id');
  assert.equal(contextAfterTurn1.id, 'evt-rec-999');
  assert.equal(contextAfterTurn1.type, 'event');
  assert.equal(contextAfterTurn1.title, 'Peña Criolla Barranco');

  // Turn 2: Follow-up question must automatically carry the preserved eventId
  await ctrl.sendAndOpen('¿Cuántas entradas hay disponibles?');

  assert.equal(capturedCalls.length, 2, 'Two calls should have been made');
  const turn2Options = capturedCalls[1];
  assert.equal(turn2Options.eventId, 'evt-rec-999', 'Turn 2 must send preserved eventId');
  assert.equal(turn2Options.presentationId, null, 'Turn 2 presentationId must remain null');

  ctrl.destroy();
  console.log('✓ 12. active_event_id preserved in chatState and sent on subsequent turns');
}

console.log('All Agent Chat Route and Composition integration tests passed!');
