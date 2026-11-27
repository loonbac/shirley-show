import { containsYouTubeUrl } from '../../utils/validation/contains-youtube-url.js';
import { isStagedAudioUnrelated } from '../../utils/validation/is-staged-audio-unrelated.js';
import { isVocalSeparationIntent } from '../../utils/validation/is-vocal-separation-intent.js';
import { resolveVocalPendingLabel } from '../../data/agent-chat-api.js';

/**
 * Creates the in-memory chat state manager for Shirley Show AI Agent.
 * Completely decoupled from DOM nodes.
 *
 * @param {Object} [initialState={}]
 * @param {Object} [options={}]
 * @param {number} [options.remoteWaitDelayMs=3000] Delay before pending vocal separation switches label to waiting PC
 * @returns {Object} Chat state manager with command and subscription methods.
 */
export function createChatState(initialState = {}, options = {}) {
  const remoteWaitDelayMs = options.remoteWaitDelayMs !== undefined ? options.remoteWaitDelayMs : 3000;
  let wakeWaitTimer = null;

  let state = {
    isOpen: false,
    pending: false,
    pendingLabel: 'Ressy está pensando...',
    vocalSeparationPending: false,
    draft: '',
    attachment: null,
    stagedAudioId: null,
    messages: [],
    activeContext: null,
    conversationId: null,
    conversationPresentationId: null,
    sessionId: initialState.sessionId || null,
    notice: null,
    ...initialState,
  };

  const listeners = new Set();
  let currentGenerationId = 0;
  let currentAbortController = null;

  function clearWakeWaitTimer() {
    if (wakeWaitTimer) {
      clearTimeout(wakeWaitTimer);
      wakeWaitTimer = null;
    }
  }

  function notify() {
    for (const listener of listeners) {
      try {
        listener(state);
      } catch (err) {
        console.error('ChatState listener error:', err);
      }
    }
  }

  function get() {
    return state;
  }

  function subscribe(listener) {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }

  function open() {
    if (state.isOpen) return;
    state = { ...state, isOpen: true };
    notify();
  }

  function close() {
    if (!state.isOpen) return;
    clearWakeWaitTimer();
    state = { ...state, isOpen: false, pendingLabel: 'Ressy está pensando...' };
    notify();
  }

  function setDraft(draft) {
    state = { ...state, draft: typeof draft === 'string' ? draft : '' };
    notify();
  }

  function setAttachment(file) {
    state = { ...state, attachment: file || null };
    notify();
  }

  function clearAttachment() {
    state = { ...state, attachment: null };
    notify();
  }

  function setStagedAudioId(id) {
    state = { ...state, stagedAudioId: typeof id === 'string' ? id : null };
    notify();
  }

  function setSessionId(id) {
    state = { ...state, sessionId: typeof id === 'string' ? id : null };
    notify();
  }

  function clearStagedAudioId() {
    state = { ...state, stagedAudioId: null };
    notify();
  }

  function setRouteContext(context) {
    // A destination named in the conversation outlives every route change:
    // route context only seeds the destination while none was chosen yet.
    if (state.conversationPresentationId) {
      return;
    }
    state = {
      ...state,
      activeContext: context
        ? {
            id: context.id,
            title: context.title,
            ...(context.type ? { type: context.type } : {}),
          }
        : null,
    };
    notify();
  }

  function setPendingLabel(label) {
    if (state.pending && typeof label === 'string') {
      state = { ...state, pendingLabel: label };
      notify();
    }
  }

  function beginTurn({ message, attachment = null, pendingLabel = null, gpuStatus = null }) {
    clearWakeWaitTimer();
    if (state.pending && currentAbortController) {
      currentAbortController.abort();
    }

    currentGenerationId += 1;
    const generationId = currentGenerationId;
    currentAbortController = new AbortController();

    let activeStagedAudioId = state.stagedAudioId;
    if (activeStagedAudioId && isStagedAudioUnrelated(message, attachment)) {
      activeStagedAudioId = null;
    }

    let isVocalIntent = isVocalSeparationIntent(message);
    if (!isVocalIntent && state.vocalSeparationPending) {
      if (!isStagedAudioUnrelated(message, attachment)) {
        isVocalIntent = true;
      }
    }

    let turnPendingLabel = pendingLabel;
    const attachmentName = (attachment && attachment.name || '').toLowerCase();
    const attachmentType = (attachment && attachment.type || '').toLowerCase();
    const isImageAttachment =
      attachmentName.endsWith('.jpg') ||
      attachmentName.endsWith('.jpeg') ||
      attachmentName.endsWith('.png') ||
      attachmentName.endsWith('.webp') ||
      attachmentType.startsWith('image/');

    if (!turnPendingLabel) {
      if (isImageAttachment) {
        turnPendingLabel = 'Ressy está subiendo el afiche…';
      } else if (gpuStatus && isVocalIntent) {
        turnPendingLabel = resolveVocalPendingLabel(gpuStatus);
      } else if (isVocalIntent) {
        turnPendingLabel = 'separando voces…';
      } else if (containsYouTubeUrl(message)) {
        turnPendingLabel = 'Ressy está descargando el audio…';
      } else {
        turnPendingLabel = 'Ressy está pensando...';
      }
    }

    const userMessage = {
      id: typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : `msg-${Date.now()}-${Math.random()}`,
      role: 'user',
      content: message || '',
      attachment: attachment ? { name: attachment.name, size: attachment.size } : null,
      timestamp: Date.now(),
    };

    state = {
      ...state,
      pending: true,
      pendingLabel: turnPendingLabel,
      vocalSeparationPending: isVocalIntent,
      stagedAudioId: activeStagedAudioId,
      notice: null,
      messages: [...state.messages, userMessage],
    };
    notify();

    return {
      generationId,
      abortController: currentAbortController,
    };
  }

  function resolveTurn({ generationId, response }) {
    // Discard stale or superseded responses
    if (generationId !== currentGenerationId) {
      return;
    }
    clearWakeWaitTimer();

    const actions = (response && response.actions_taken) || [];
    const hasConsumedTrack = actions.some((act) => act.tool === 'add_track_from_staged' && act.status === 'success');
    const hasConsumedPoster = actions.some((act) => act.tool === 'subir_afiche' && act.status === 'success');
    const hasConsumedAttachment = hasConsumedTrack || hasConsumedPoster;
    let nextConversationId = state.conversationId;
    if (response && response.conversation_id) {
      nextConversationId = response.conversation_id;
    }

    let nextStagedAudioId = state.stagedAudioId;
    if (response && response.staged_audio_id !== undefined) {
      nextStagedAudioId = response.staged_audio_id;
    }
    if (hasConsumedTrack) {
      nextStagedAudioId = null;
    }

    const assistantMessage = {
      id: typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : `msg-${Date.now()}-${Math.random()}`,
      role: 'assistant',
      content: (response && response.message) || '',
      actions,
      timestamp: Date.now(),
    };

    let updatedContext = state.activeContext;
    let conversationPresentationId = state.conversationPresentationId;
    const activePresentationId = response && response.active_presentation_id ? response.active_presentation_id : null;
    const activeEventId = response && response.active_event_id ? response.active_event_id : null;
    const isPresentationDeleted = actions.some(
      (act) =>
        act.tool === 'eliminar_presentacion' &&
        act.status === 'success' &&
        (act.presentation_id === activePresentationId ||
          act.presentation_id === (conversationPresentationId || state.activeContext?.id))
    );
    const isEventDeleted = actions.some(
      (act) => act.tool === 'eliminar_evento' && act.status === 'success' && act.event_id === state.activeContext?.id
    );
    if (isPresentationDeleted || isEventDeleted) {
      updatedContext = null;
      if (isPresentationDeleted) {
        conversationPresentationId = null;
      }
    } else if (activePresentationId) {
      conversationPresentationId = activePresentationId;
      if (!updatedContext || updatedContext.id !== activePresentationId || updatedContext.type === 'event') {
        // Prefer an explicit title, then a title carried by the matching action.
        const titleAction = actions.find(
          (act) =>
            act.presentation_id === activePresentationId &&
            act.details &&
            (act.details.title || act.details.titulo || act.details.nombre)
        );
        const responseTitle =
          response.active_presentation_title ||
          (response.active_presentation && response.active_presentation.title);
        const isCreation = actions.some(
          (act) =>
            act.tool === 'create_presentation' &&
            act.status === 'success' &&
            act.presentation_id === activePresentationId
        );
        const title =
          responseTitle ||
          (titleAction && (titleAction.details.title || titleAction.details.titulo || titleAction.details.nombre)) ||
          (isCreation ? 'Nueva presentación' : activePresentationId);
        updatedContext = {
          id: activePresentationId,
          title,
        };
      }
    } else if (activeEventId) {
      conversationPresentationId = null;
      if (!updatedContext || updatedContext.id !== activeEventId || updatedContext.type !== 'event') {
        const eventAction = actions.find(
          (act) =>
            (act.tool === 'crear_evento' || act.event_id === response.active_event_id) &&
            act.details &&
            (act.details.nombre || act.details.title)
        );
        const title =
          (eventAction && eventAction.details && (eventAction.details.nombre || eventAction.details.title)) ||
          (updatedContext && updatedContext.id === response.active_event_id && updatedContext.title) ||
          'Evento';
        updatedContext = {
          id: response.active_event_id,
          title,
          type: 'event',
        };
      }
    }

    const notice = (response && response.notice) || null;
    const isProviderNotice = notice && (
      notice.code === 'provider_auth' ||
      notice.code === 'provider_rate_limited' ||
      notice.code === 'provider_unavailable' ||
      notice.code === 'agent_timeout' ||
      notice.code === 'provider_error'
    );

    state = {
      ...state,
      pending: false,
      pendingLabel: 'Ressy está pensando...',
      vocalSeparationPending: false,
      stagedAudioId: nextStagedAudioId,
      draft: isProviderNotice ? state.draft : '',
      attachment: hasConsumedAttachment ? null : state.attachment,
      activeContext: updatedContext,
      conversationId: nextConversationId,
      conversationPresentationId,
      notice,
      messages: [...state.messages, assistantMessage],
    };
    currentAbortController = null;
    notify();
  }

  function failTurn({ generationId, error }) {
    if (generationId !== currentGenerationId) {
      return;
    }
    clearWakeWaitTimer();

    const notice = {
      code: (error && error.code) || 'agent_error',
      message: (error && error.message) || 'Error al procesar el mensaje.',
      retryable: error ? error.retryable !== false : true,
    };

    state = {
      ...state,
      pending: false,
      pendingLabel: 'Ressy está pensando...',
      vocalSeparationPending: false,
      notice,
      // Draft and attachment are strictly preserved
    };
    currentAbortController = null;
    notify();
  }

  function cancelTurn() {
    clearWakeWaitTimer();
    if (currentAbortController) {
      currentAbortController.abort();
      currentAbortController = null;
    }
    currentGenerationId += 1;
    if (state.pending || state.vocalSeparationPending) {
      state = { ...state, pending: false, pendingLabel: 'Ressy está pensando...', vocalSeparationPending: false };
      notify();
    }
  }

  return {
    get,
    subscribe,
    open,
    close,
    setDraft,
    setAttachment,
    clearAttachment,
    setStagedAudioId,
    clearStagedAudioId,
    setSessionId,
    setRouteContext,
    setPendingLabel,
    beginTurn,
    resolveTurn,
    failTurn,
    cancelTurn,
  };
}
