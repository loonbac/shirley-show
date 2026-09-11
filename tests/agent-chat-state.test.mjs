import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';

const chatStateModulePath = resolve(process.cwd(), 'frontend/js/core/state/chat-state.js');
const { createChatState } = await import(pathToFileURL(chatStateModulePath).href);

const activeContextModulePath = resolve(process.cwd(), 'frontend/js/core/state/active-presentation-context.js');
const { activePresentationContextSignal } = await import(pathToFileURL(activeContextModulePath).href);

const agentRefreshModulePath = resolve(process.cwd(), 'frontend/js/core/state/agent-refresh.js');
const { emitAgentRefresh, onAgentRefresh } = await import(pathToFileURL(agentRefreshModulePath).href);

const stageModeStateModulePath = resolve(process.cwd(), 'frontend/js/core/state/stage-mode-state.js');
const { stageModeSignal } = await import(pathToFileURL(stageModeStateModulePath).href);

const bytesFormatModulePath = resolve(process.cwd(), 'frontend/js/utils/format/bytes.js');
const { formatBytes } = await import(pathToFileURL(bytesFormatModulePath).href);

const agentApiModulePath = resolve(process.cwd(), 'frontend/js/data/agent-chat-api.js');
const { sendAgentChatMessage, validateAudioAttachment } = await import(pathToFileURL(agentApiModulePath).href);

console.log('Running Agent Chat State and API Client Unit Tests...');

// 1. Format bytes utility
{
  assert.equal(formatBytes(0), '');
  assert.equal(formatBytes(-10), '');
  assert.equal(formatBytes(512), '0.5 KB');
  assert.equal(formatBytes(1024), '1.0 KB');
  assert.equal(formatBytes(1024 * 1024), '1.0 MB');
  assert.equal(formatBytes(5.4 * 1024 * 1024), '5.4 MB');
  console.log('✓ 1. formatBytes formats bytes accurately');
}

// 2. Chat state transitions
{
  const state = createChatState();
  assert.equal(state.get().isOpen, false);
  assert.equal(state.get().pending, false);
  assert.equal(state.get().draft, '');
  assert.equal(state.get().attachment, null);
  assert.deepEqual(state.get().messages, []);

  // Open & close
  state.open();
  assert.equal(state.get().isOpen, true);
  state.close();
  assert.equal(state.get().isOpen, false);

  // Set draft
  state.setDraft('Crea un show');
  assert.equal(state.get().draft, 'Crea un show');

  // Set and replace attachment
  const file1 = { name: 'track1.mp3', size: 1024 };
  const file2 = { name: 'track2.wav', size: 2048 };
  state.setAttachment(file1);
  assert.deepEqual(state.get().attachment, file1);
  state.setAttachment(file2);
  assert.deepEqual(state.get().attachment, file2);

  state.clearAttachment();
  assert.equal(state.get().attachment, null);

  // Set route context
  state.setRouteContext({ id: 'show-1', title: 'Show Acústico' });
  assert.deepEqual(state.get().activeContext, { id: 'show-1', title: 'Show Acústico' });

  // Begin turn
  const turn = state.beginTurn({ message: 'Añade una nota', attachment: null });
  assert.equal(state.get().pending, true);
  assert.equal(state.get().messages.length, 1);
  assert.equal(state.get().messages[0].role, 'user');
  assert.equal(state.get().messages[0].content, 'Añade una nota');
  assert.ok(turn.generationId > 0);
  assert.ok(turn.abortController instanceof AbortController);

  // Resolving turn with valid generationId
  state.resolveTurn({
    generationId: turn.generationId,
    response: {
      message: 'Nota agregada con éxito.',
      actions_taken: [
        {
          tool: 'add_note',
          presentation_id: 'show-1',
          summary: 'Nota añadida',
          status: 'success',
          details: { item_id: 'note-1', kind: 'note' },
        },
      ],
      active_presentation_id: 'show-1',
      notice: null,
    },
  });

  assert.equal(state.get().pending, false);
  assert.equal(state.get().messages.length, 2);
  assert.equal(state.get().messages[1].role, 'assistant');
  assert.equal(state.get().messages[1].content, 'Nota agregada con éxito.');
  assert.equal(state.get().messages[1].actions.length, 1);
  assert.equal(state.get().draft, '');

  console.log('✓ 2. Chat state transitions (open/close, draft, attachment, begin/resolve turn)');
}

// 3. Late response discarding via generation ID
{
  const state = createChatState();
  const turn1 = state.beginTurn({ message: 'Turn 1' });
  assert.equal(state.get().pending, true);

  // User or system cancels or starts a new turn, invalidating turn1
  state.cancelTurn();
  assert.equal(state.get().pending, false);
  assert.equal(turn1.abortController.signal.aborted, true);

  // Turn 1 resolves late
  state.resolveTurn({
    generationId: turn1.generationId,
    response: {
      message: 'Late response 1',
      actions_taken: [],
      active_presentation_id: null,
      notice: null,
    },
  });

  // Stale resolution must NOT append messages
  assert.equal(state.get().messages.length, 1); // Only user message from turn1
  assert.equal(state.get().messages[0].content, 'Turn 1');

  console.log('✓ 3. Discarding late responses via generation ID and AbortController');
}

// 4. Failed turn retains draft and attachment
{
  const state = createChatState();
  state.setDraft('Intento fallido');
  const dummyFile = { name: 'audio.mp3', size: 4096 };
  state.setAttachment(dummyFile);

  const turn = state.beginTurn({ message: 'Intento fallido', attachment: dummyFile });
  assert.equal(state.get().pending, true);

  state.failTurn({
    generationId: turn.generationId,
    error: {
      code: 'network_error',
      message: 'No se pudo conectar con el servidor.',
      retryable: true,
    },
  });

  assert.equal(state.get().pending, false);
  assert.equal(state.get().draft, 'Intento fallido', 'Draft must be retained on failure');
  assert.deepEqual(state.get().attachment, dummyFile, 'Attachment must be retained on failure');
  assert.ok(state.get().notice !== null);
  assert.equal(state.get().notice.code, 'network_error');
  assert.equal(state.get().notice.retryable, true);

  console.log('✓ 4. Failed turn retains draft text and attachment with retryable notice');
}

// 5. Attachment cleared only if add_track_from_staged succeeded
{
  const state = createChatState();
  const trackFile = { name: 'pista.mp3', size: 1024 * 1024 };
  state.setAttachment(trackFile);

  const turn1 = state.beginTurn({ message: 'Sube esta pista', attachment: trackFile });
  // Response has actions_taken with create_presentation, but NOT add_track_from_staged
  state.resolveTurn({
    generationId: turn1.generationId,
    response: {
      message: 'Creé la presentación pero no pude añadir la pista.',
      actions_taken: [{ tool: 'create_presentation', status: 'success' }],
      active_presentation_id: 'new-id',
      notice: null,
    },
  });

  // Since add_track_from_staged was not executed, attachment remains in state
  assert.deepEqual(state.get().attachment, trackFile, 'Attachment must be retained when add_track_from_staged did not execute');

  const turn2 = state.beginTurn({ message: 'Ahora sí añade la pista', attachment: trackFile });
  state.resolveTurn({
    generationId: turn2.generationId,
    response: {
      message: 'Pista añadida con éxito.',
      actions_taken: [{ tool: 'add_track_from_staged', status: 'success' }],
      active_presentation_id: 'new-id',
      notice: null,
    },
  });

  assert.equal(state.get().attachment, null, 'Attachment must be cleared when add_track_from_staged succeeded');
  console.log('✓ 5. Attachment retention semantics based on add_track_from_staged execution');
}

// 6. Active presentation context and stage mode signals
{
  activePresentationContextSignal.set({ id: 'pres-123', title: 'Concierto 2026' });
  assert.deepEqual(activePresentationContextSignal.get(), { id: 'pres-123', title: 'Concierto 2026' });

  activePresentationContextSignal.set(null);
  assert.equal(activePresentationContextSignal.get(), null);

  stageModeSignal.set(true);
  assert.equal(stageModeSignal.get(), true);
  stageModeSignal.set(false);
  assert.equal(stageModeSignal.get(), false);

  let receivedRefresh = null;
  const unsubscribe = onAgentRefresh((event) => {
    receivedRefresh = event;
  });

  emitAgentRefresh({ scope: 'catalog' });
  assert.deepEqual(receivedRefresh, { scope: 'catalog' });

  emitAgentRefresh({ scope: 'presentation', presentationId: 'p-1' });
  assert.deepEqual(receivedRefresh, { scope: 'presentation', presentationId: 'p-1' });

  unsubscribe();
  console.log('✓ 6. Signals: active context, stage mode, and agent refresh');
}

// 7. Client-side audio file validation & rejection before dispatch
{
  // Valid files
  assert.equal(validateAudioAttachment({ name: 'song.mp3', size: 10 * 1024 * 1024 }).valid, true);
  assert.equal(validateAudioAttachment({ name: 'track.WAV', size: 50 * 1024 * 1024 }).valid, true);
  assert.equal(validateAudioAttachment({ name: 'audio.m4a', size: 100 * 1024 * 1024 }).valid, true);

  // File too large (> 100 MiB = 104,857,600 bytes)
  const tooLarge = validateAudioAttachment({ name: 'big.mp3', size: 104857601 });
  assert.equal(tooLarge.valid, false);
  assert.equal(tooLarge.error.code, 'file_too_large');

  // Unsupported format
  const unsupported1 = validateAudioAttachment({ name: 'document.pdf', size: 1024 });
  assert.equal(unsupported1.valid, false);
  assert.equal(unsupported1.error.code, 'unsupported_format');

  const unsupported2 = validateAudioAttachment({ name: 'audio.ogg', size: 1024 });
  assert.equal(unsupported2.valid, false);
  assert.equal(unsupported2.error.code, 'unsupported_format');

  // Network dispatch immediately rejected before fetch
  let dispatchThrew = false;
  try {
    await sendAgentChatMessage({
      message: 'Prueba',
      attachment: { name: 'virus.exe', size: 500 },
    });
  } catch (err) {
    dispatchThrew = true;
    assert.equal(err.code, 'unsupported_format');
  }
  assert.equal(dispatchThrew, true, 'sendAgentChatMessage must reject invalid attachment before dispatch');

  console.log('✓ 7. Client-side file validation (size <= 100 MiB, format mp3/wav/m4a)');
}

// 8. Retry semantics: retry offered ONLY when actions_taken is empty
{
  const state = createChatState();
  const turn = state.beginTurn({ message: 'Crea show y añade pista' });

  // Scenario A: Turn finishes with partial failure notice, but has confirmed actions
  state.resolveTurn({
    generationId: turn.generationId,
    response: {
      message: 'Creé el show pero ocurrió un error con la pista.',
      actions_taken: [{ tool: 'create_presentation', status: 'success' }],
      active_presentation_id: 'pres-abc',
      notice: { code: 'tool_failure', retryable: true }, // notice says retryable
    },
  });

  // When actions_taken is non-empty, retry MUST NOT be offered as a safe blind retry
  const lastMsg = state.get().messages[state.get().messages.length - 1];
  const canSafelyRetry = (lastMsg.actions || []).length === 0 && Boolean(state.get().notice && state.get().notice.retryable);
  assert.equal(canSafelyRetry, false, 'Blind retry must NOT be offered if actions_taken is non-empty');

  // Scenario B: Turn failed with zero actions taken
  const turn2 = state.beginTurn({ message: 'Listar shows' });
  state.failTurn({
    generationId: turn2.generationId,
    error: { code: 'provider_rate_limited', message: 'Límite excedido', retryable: true },
  });
  const canRetryTurn2 = Boolean(state.get().notice && state.get().notice.retryable);
  assert.equal(canRetryTurn2, true, 'Retry IS offered when zero actions were taken');

  console.log('✓ 8. Retry offered only when actions_taken is empty');
}

// 9. resolveTurn preserves draft on provider notices
{
  const providerCodes = ['provider_auth', 'provider_rate_limited', 'provider_unavailable', 'agent_timeout'];
  for (const code of providerCodes) {
    const state = createChatState();
    state.setDraft(`Mensaje guardado para ${code}`);
    const turn = state.beginTurn({ message: `Mensaje guardado para ${code}` });
    state.resolveTurn({
      generationId: turn.generationId,
      response: {
        message: 'Error de proveedor',
        actions_taken: [],
        active_presentation_id: null,
        notice: { code, retryable: true },
      },
    });

    assert.equal(
      state.get().draft,
      `Mensaje guardado para ${code}`,
      `Draft must be preserved when notice.code is ${code}`
    );
    assert.equal(state.get().notice.code, code);
  }
  console.log('✓ 9. resolveTurn preserves draft on provider notices (provider_auth, rate_limited, unavailable, timeout)');
}

console.log('All Agent Chat State and API Client unit tests passed!');
