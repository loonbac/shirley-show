import { createAgentChatPanel } from '../components/organisms/agent-chat-panel.js';
import { sendAgentChatMessage, getGpuStatus } from '../data/agent-chat-api.js';
import { emitAgentRefresh } from '../core/state/agent-refresh.js';
import { activePresentationContextSignal } from '../core/state/active-presentation-context.js';
import { containsYouTubeUrl } from '../utils/validation/contains-youtube-url.js';
import { isVocalSeparationIntent } from '../utils/validation/is-vocal-separation-intent.js';
import { isStagedAudioUnrelated } from '../utils/validation/is-staged-audio-unrelated.js';

let activeAgentChatController = null;

/**
 * Returns the currently active agent chat controller singleton instance, if initialized.
 * @returns {ReturnType<typeof createAgentChatController> | null}
 */
export function getActiveAgentChatController() {
  return activeAgentChatController;
}

/**
 * Controller orchestrating the Ressy AI Agent conversational lifecycle.
 * Manages the singleton panel, network dispatch, reactive view refresh signals,
 * and ephemeral state synchronization.
 *
 * @param {{
 *   chatState: any,
 * }} deps
 * @returns {{
 *   open: (initialMessage?: string) => void,
 *   close: () => void,
 *   toggle: () => void,
 *   sendAndOpen: (message: string) => Promise<void> | void,
 *   subscribe: (listener: (state: any) => void) => () => void,
 *   panelEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createAgentChatController(deps) {
  const {
    chatState,
    getGpuStatusFn = getGpuStatus,
    sendAgentChatMessageFn = sendAgentChatMessage,
  } = deps;

  // Initialize with current route context if present
  const initialRouteContext = activePresentationContextSignal.get();
  if (initialRouteContext) {
    chatState.setRouteContext(initialRouteContext);
  }

  // Sync route context changes into chat state
  const unsubscribeContext = activePresentationContextSignal.subscribe((context) => {
    chatState.setRouteContext(context);
  });

  const panel = createAgentChatPanel({
    isOpen: chatState.get().isOpen,
    activeContext: chatState.get().activeContext,
    messages: chatState.get().messages,
    pending: chatState.get().pending,
    pendingLabel: chatState.get().pendingLabel,
    draft: chatState.get().draft,
    attachment: chatState.get().attachment,
    notice: chatState.get().notice,
    onClose: () => {
      chatState.close();
    },
    onSend: (message) => {
      handleSend(message);
    },
    onChangeDraft: (draft) => {
      chatState.setDraft(draft);
    },
    onAttach: (file) => {
      chatState.setAttachment(file);
    },
    onRemoveAttachment: () => {
      chatState.clearAttachment();
    },
    onRetry: () => {
      handleRetry();
    },
  });

  // Keep panel updated with chat state changes
  const unsubscribeState = chatState.subscribe((newState) => {
    panel.update({
      isOpen: newState.isOpen,
      activeContext: newState.activeContext,
      messages: newState.messages,
      pending: newState.pending,
      pendingLabel: newState.pendingLabel,
      draft: newState.draft,
      attachment: newState.attachment,
      notice: newState.notice,
    });
  });

  async function handleSend(userMessage) {
    const priorState = chatState.get();
    if (priorState.pending) return;

    const attachment = priorState.attachment;

    let isVocalIntent = isVocalSeparationIntent(userMessage);
    if (!isVocalIntent && priorState.vocalSeparationPending) {
      if (!isStagedAudioUnrelated(userMessage, attachment)) {
        isVocalIntent = true;
      }
    }

    const turn = chatState.beginTurn({
      message: userMessage,
      attachment,
    });

    if (isVocalIntent) {
      try {
        const gpuStatus = await getGpuStatusFn({ signal: turn.abortController.signal });
        if (gpuStatus && gpuStatus.mode === 'remote' && !gpuStatus.pc_online) {
          chatState.setPendingLabel('esperando la PC de Joshua…');
        } else {
          chatState.setPendingLabel('separando voces…');
        }
      } catch (_err) {
        chatState.setPendingLabel('separando voces…');
      }
    }

    const stagedAudioId = chatState.get().stagedAudioId;

    // Prune history for the backend: extract only textual user & assistant messages
    const history = priorState.messages
      .filter((m) => m.role === 'user' || m.role === 'assistant')
      .map((m) => ({ role: m.role, content: m.content }));

    const isEventContext = priorState.activeContext && priorState.activeContext.type === 'event';
    const presentationId = priorState.activeContext && !isEventContext ? priorState.activeContext.id : null;
    const eventId = isEventContext ? priorState.activeContext.id : null;

    try {
      const response = await sendAgentChatMessageFn({
        message: userMessage,
        history,
        presentationId,
        eventId,
        stagedAudioId,
        conversationId: priorState.conversationId || null,
        attachment,
        signal: turn.abortController.signal,
      });

      chatState.resolveTurn({
        generationId: turn.generationId,
        response,
      });

      // Emit reactive refresh signals for confirmed actions
      const actions = (response && response.actions_taken) || [];
      let refreshedCatalog = false;
      let refreshedEvents = false;

      for (const act of actions) {
        if (act.status !== 'success') continue;

        if ((act.tool === 'create_presentation' || act.tool === 'renombrar_presentacion') && !refreshedCatalog) {
          emitAgentRefresh({ scope: 'catalog' });
          refreshedCatalog = true;
        }

        if (act.tool === 'eliminar_presentacion') {
          const targetId = act.presentation_id || response.active_presentation_id;
          if (targetId) {
            emitAgentRefresh({ scope: 'presentation', presentationId: targetId, removed: true });
          }
          if (!refreshedCatalog) {
            emitAgentRefresh({ scope: 'catalog' });
            refreshedCatalog = true;
          }
        } else if (act.tool === 'eliminar_evento') {
          const targetEventId = act.event_id || response.active_event_id;
          if (targetEventId) {
            emitAgentRefresh({ scope: 'event', eventId: targetEventId, removed: true });
          }
          if (!refreshedEvents) {
            emitAgentRefresh({ scope: 'events' });
            refreshedEvents = true;
          }
        } else if (
          act.tool === 'add_lyrics' ||
          act.tool === 'add_note' ||
          act.tool === 'add_track_from_staged' ||
          act.tool === 'renombrar_presentacion' ||
          act.tool === 'reordenar_elemento' ||
          act.tool === 'editar_elemento' ||
          act.tool === 'eliminar_elemento'
        ) {
          const targetId = act.presentation_id || response.active_presentation_id;
          if (targetId) {
            emitAgentRefresh({ scope: 'presentation', presentationId: targetId });
          }
        } else if (
          act.tool === 'crear_evento' ||
          act.tool === 'editar_evento' ||
          act.tool === 'reabrir_evento' ||
          act.tool === 'finalizar_evento'
        ) {
          if (!refreshedEvents) {
            emitAgentRefresh({ scope: 'events' });
            refreshedEvents = true;
          }
          const targetEventId = act.event_id || response.active_event_id;
          if (targetEventId) {
            emitAgentRefresh({ scope: 'event', eventId: targetEventId });
          }
        } else if (
          act.tool === 'vender_entradas' ||
          act.tool === 'confirmar_pago_entrada' ||
          act.tool === 'registrar_gasto' ||
          act.tool === 'registrar_interesado' ||
          act.tool === 'checkin_entrada' ||
          act.tool === 'editar_gasto' ||
          act.tool === 'editar_interesado' ||
          act.tool === 'confirmar_interesado' ||
          act.tool === 'eliminar_entrada' ||
          act.tool === 'eliminar_gasto' ||
          act.tool === 'eliminar_interesado'
        ) {
          const targetEventId = act.event_id || response.active_event_id;
          if (targetEventId) {
            emitAgentRefresh({ scope: 'event', eventId: targetEventId });
          }
        }
      }
        } catch (err) {
      chatState.failTurn({
        generationId: turn.generationId,
        error: err,
      });
    }
  }

  function handleRetry() {
    const currentState = chatState.get();
    if (currentState.draft || currentState.attachment) {
      handleSend(currentState.draft);
    }
  }

  function open(initialMessage) {
    chatState.open();
    if (initialMessage && typeof initialMessage === 'string' && initialMessage.trim()) {
      return handleSend(initialMessage.trim());
    }
  }

  function close() {
    chatState.close();
  }

  function toggle() {
    if (chatState.get().isOpen) {
      close();
    } else {
      open();
    }
  }

  function sendAndOpen(message) {
    return open(message);
  }

  function destroy() {
    if (activeAgentChatController === controller) {
      activeAgentChatController = null;
    }
    unsubscribeContext();
    unsubscribeState();
    chatState.cancelTurn();
    panel.destroy();
  }

  const controller = {
    open,
    close,
    toggle,
    sendAndOpen,
    subscribe: (listener) => chatState.subscribe(listener),
    panelEl: panel.el,
    destroy,
  };

  activeAgentChatController = controller;

  return controller;
}
