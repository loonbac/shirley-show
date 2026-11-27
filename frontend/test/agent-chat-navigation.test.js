import assert from 'node:assert/strict';
import { test } from 'node:test';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

const load = (relativePath) => import(pathToFileURL(resolve(process.cwd(), relativePath)).href);

const { createChatState } = await load('frontend/js/core/state/chat-state.js');
const { createAgentChatController } = await load('frontend/js/controllers/agent-chat-controller.js');
const { activePresentationContextSignal } = await load('frontend/js/core/state/active-presentation-context.js');
const { stageModeSignal } = await load('frontend/js/core/state/stage-mode-state.js');
const { onAgentRefresh } = await load('frontend/js/core/state/agent-refresh.js');

function resetSignals() {
  activePresentationContextSignal.set(null);
  stageModeSignal.set(false);
}

function badgeText(controller) {
  return controller.panelEl.querySelector('.agent-chat-context-badge').textContent.trim();
}

function deferred() {
  let resolveFn;
  const promise = new Promise((resolvePromise) => {
    resolveFn = resolvePromise;
  });
  return { promise, resolve: resolveFn };
}

function recordingSender(responses) {
  const calls = [];
  const send = async (options) => {
    calls.push(options);
    const next = responses.shift();
    if (typeof next === 'function') return next(options);
    return next;
  };
  return { calls, send };
}

const okResponse = (overrides = {}) => ({
  message: 'Listo',
  actions_taken: [],
  active_presentation_id: null,
  active_event_id: null,
  conversation_id: 'conv-navigation',
  notice: null,
  ...overrides,
});

test('S4: una separación en vuelo sobrevive a cambios de ruta sin abortar ni perder la conversación', async () => {
  resetSignals();
  activePresentationContextSignal.set({ id: 'pres-route', title: 'Show de ruta' });

  const chatState = createChatState({ conversationId: 'conv-navigation' });
  const late = deferred();
  const sender = recordingSender([
    okResponse({
      message: 'Destino recordado',
      active_presentation_id: 'pres-elegido',
      active_presentation_title: 'Peña del Viernes',
    }),
    late.promise,
    okResponse({
      message: 'Pista guardada en Peña del Viernes',
      actions_taken: [
        { tool: 'add_track_from_staged', status: 'success', presentation_id: 'pres-elegido' },
      ],
      active_presentation_id: 'pres-elegido',
    }),
  ]);
  const controller = createAgentChatController({
    chatState,
    sendAgentChatMessageFn: sender.send,
  });

  try {
    await controller.sendAndOpen('Guarda la separación en Peña del Viernes');
    assert.equal(chatState.get().activeContext.id, 'pres-elegido');

    const inFlight = controller.sendAndOpen('Separa otra pista y guárdala ahí');
    const sentSignal = sender.calls[1].signal;

    // Navegación real: detalle → catálogo → evento, con la petición todavía abierta.
    activePresentationContextSignal.set({ id: 'pres-otra', title: 'Otro show' });
    activePresentationContextSignal.set(null);
    activePresentationContextSignal.set({ id: 'evt-otro', title: 'Otro evento', type: 'event' });

    assert.equal(sentSignal.aborted, false, 'la ruta no debe abortar la separación en curso');
    assert.equal(chatState.get().pending, true, 'la ruta no debe limpiar el estado pendiente');
    assert.equal(chatState.get().messages.length, 3, 'la ruta no debe borrar mensajes');
    assert.equal(chatState.get().conversationId, 'conv-navigation', 'la ruta no debe borrar conversationId');
    assert.equal(
      chatState.get().activeContext && chatState.get().activeContext.id,
      'pres-elegido',
      'el repertorio elegido en la conversación gana sobre el contexto de la ruta'
    );

    late.resolve(okResponse({
      message: 'Pista guardada',
      actions_taken: [
        { tool: 'add_track_from_staged', status: 'success', presentation_id: 'pres-elegido' },
      ],
      active_presentation_id: 'pres-elegido',
    }));
    await inFlight;

    assert.equal(sentSignal.aborted, false, 'la respuesta no debe llegar cancelada');
    assert.equal(chatState.get().messages.length, 4, 'la respuesta se incorpora al historial');
    assert.equal(chatState.get().conversationId, 'conv-navigation');
    assert.equal(chatState.get().activeContext.id, 'pres-elegido');
  } finally {
    controller.destroy();
  }
});

test('S3/S4: el contexto de ruta sólo siembra el destino; después gana el elegido en la conversación', async () => {
  resetSignals();
  activePresentationContextSignal.set({ id: 'pres-semilla', title: 'Show semilla' });

  const chatState = createChatState();
  const sender = recordingSender([
    okResponse({ active_presentation_id: 'pres-elegido', active_presentation_title: 'Peña del Viernes' }),
    okResponse({ active_presentation_id: 'pres-elegido' }),
  ]);
  const controller = createAgentChatController({ chatState, sendAgentChatMessageFn: sender.send });

  try {
    assert.equal(chatState.get().activeContext.id, 'pres-semilla', 'la ruta inicial siembra el destino');

    activePresentationContextSignal.set({ id: 'pres-otra', title: 'Otro show' });
    assert.equal(
      chatState.get().activeContext.id,
      'pres-otra',
      'antes de elegir destino, la ruta todavía puede cambiarlo'
    );

    await controller.sendAndOpen('Guárdalo en Peña del Viernes');
    assert.equal(chatState.get().activeContext.id, 'pres-elegido');
    assert.equal(sender.calls[0].presentationId, 'pres-otra', 'el primer turno usa el contexto de ruta vigente');

    activePresentationContextSignal.set({ id: 'pres-tercera', title: 'Tercer show' });
    activePresentationContextSignal.set(null);
    activePresentationContextSignal.set({ id: 'evt-navegado', title: 'Evento navegado', type: 'event' });

    assert.equal(chatState.get().activeContext.id, 'pres-elegido', 'ninguna ruta sobrescribe el destino elegido');

    await controller.sendAndOpen('Agrega otra pista');
    assert.equal(
      sender.calls[1].presentationId,
      'pres-elegido',
      'el siguiente turno sigue guardando en el repertorio elegido'
    );
    assert.equal(sender.calls[1].eventId, null, 'la ruta de evento no sustituye al destino conversado');
  } finally {
    controller.destroy();
  }
});

test('S4: el badge existente muestra el destino elegido y conserva el modo catálogo sin destino', async () => {
  resetSignals();
  const chatState = createChatState();
  const sender = recordingSender([
    okResponse({ active_presentation_id: 'pres-elegido', active_presentation_title: 'Peña del Viernes' }),
  ]);
  const controller = createAgentChatController({ chatState, sendAgentChatMessageFn: sender.send });

  try {
    assert.equal(badgeText(controller), 'Modo catálogo (creará un nuevo show)');

    await controller.sendAndOpen('Guarda la pista en Peña del Viernes');
    assert.match(
      badgeText(controller),
      /Peña del Viernes/,
      'el badge debe nombrar el repertorio destino elegido en la conversación'
    );

    activePresentationContextSignal.set(null);
    activePresentationContextSignal.set({ id: 'pres-otra', title: 'Show de otra ruta' });
    assert.match(badgeText(controller), /Peña del Viernes/, 'navegar no debe cambiar el destino mostrado');
  } finally {
    controller.destroy();
  }
});

test('S4: sin título en la respuesta, el badge muestra el destino elegido en vez de un nombre inventado', async () => {
  resetSignals();
  const chatState = createChatState();
  const sender = recordingSender([
    okResponse({ active_presentation_id: 'pres-elegido' }),
  ]);
  const controller = createAgentChatController({ chatState, sendAgentChatMessageFn: sender.send });

  try {
    await controller.sendAndOpen('Guarda la pista en Peña del Viernes');
    assert.match(badgeText(controller), /pres-elegido/);
    assert.doesNotMatch(badgeText(controller), /Nueva presentación/);
  } finally {
    controller.destroy();
  }
});

test('S4: un resultado sin actions_taken refresca igualmente la presentación destino', async () => {
  resetSignals();
  const chatState = createChatState();
  const sender = recordingSender([
    okResponse({ active_presentation_id: 'pres-elegido', active_presentation_title: 'Peña del Viernes' }),
    okResponse({ message: 'Separación terminada', actions_taken: [], active_presentation_id: 'pres-elegido' }),
  ]);
  const controller = createAgentChatController({ chatState, sendAgentChatMessageFn: sender.send });

  const refreshes = [];
  const unsubscribe = onAgentRefresh((event) => refreshes.push(event));

  try {
    await controller.sendAndOpen('Guarda en Peña del Viernes');
    refreshes.length = 0;

    // El usuario ya se fue al catálogo cuando termina la separación.
    activePresentationContextSignal.set(null);
    await controller.sendAndOpen('Separa los coros y guárdalos ahí');

    assert.ok(
      refreshes.some((event) => event.scope === 'presentation' && event.presentationId === 'pres-elegido'),
      'la vista del repertorio destino debe refrescar aunque no haya actions_taken y el usuario esté en otra ruta'
    );
  } finally {
    unsubscribe();
    controller.destroy();
  }
});

test('PRESERVE: el contexto de evento de ruta sigue viajando como event_id sin destino elegido', async () => {
  resetSignals();
  activePresentationContextSignal.set({ id: 'evt-presupuesto', title: 'Peña Criolla', type: 'event' });

  const chatState = createChatState();
  const sender = recordingSender([
    okResponse({ active_event_id: 'evt-presupuesto' }),
    okResponse({ active_event_id: 'evt-presupuesto' }),
  ]);
  const controller = createAgentChatController({ chatState, sendAgentChatMessageFn: sender.send });

  try {
    await controller.sendAndOpen('¿Cuántas entradas se vendieron?');
    await controller.sendAndOpen('Y los gastos?');

    for (const call of sender.calls) {
      assert.equal(call.eventId, 'evt-presupuesto', 'el evento de ruta se envía como eventId');
      assert.equal(call.presentationId, null, 'un evento nunca se envía como presentationId');
    }
    assert.equal(chatState.get().activeContext.type, 'event');
  } finally {
    controller.destroy();
  }
});

test('PRESERVE: Stage Mode esconde el bubble pero no cancela la petición en curso', async () => {
  resetSignals();
  const chatState = createChatState();
  const late = deferred();
  const sender = recordingSender([late.promise]);
  const controller = createAgentChatController({ chatState, sendAgentChatMessageFn: sender.send });

  try {
    const inFlight = controller.sendAndOpen('Separa los coros');
    stageModeSignal.set(true);

    assert.ok(controller.floatingTriggerEl.getAttribute('class').includes('is-hidden'));
    assert.equal(sender.calls[0].signal.aborted, false, 'Stage Mode no aborta la separación');
    assert.equal(chatState.get().pending, true);

    late.resolve(okResponse({ message: 'Separación lista' }));
    await inFlight;

    stageModeSignal.set(false);
    assert.ok(!controller.floatingTriggerEl.getAttribute('class').includes('is-hidden'));
  } finally {
    controller.destroy();
  }
});
