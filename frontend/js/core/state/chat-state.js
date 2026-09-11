import { containsYouTubeUrl } from '../../utils/validation/contains-youtube-url.js';
import { isStagedAudioUnrelated } from '../../utils/validation/is-staged-audio-unrelated.js';

/**
 * Creates the in-memory chat state manager for Shirley Show AI Agent.
 * Completely decoupled from DOM nodes.
 *
 * @param {Object} [initialState={}]
 * @returns {Object} Chat state manager with command and subscription methods.
 */
export function createChatState(initialState = {}) {
  let state = {
    isOpen: false,
    pending: false,
    pendingLabel: 'Ressy está pensando...',
    draft: '',
    attachment: null,
    stagedAudioId: null,
    messages: [],
    activeContext: null,
    notice: null,
    ...initialState,
  };

  const listeners = new Set();
  let currentGenerationId = 0;
  let currentAbortController = null;

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
    state = { ...state, isOpen: false };
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

  function clearStagedAudioId() {
    state = { ...state, stagedAudioId: null };
    notify();
  }

  function setRouteContext(context) {
    state = {
      ...state,
      activeContext: context ? { id: context.id, title: context.title } : null,
    };
    notify();
  }

  function beginTurn({ message, attachment = null, pendingLabel = null }) {
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

    let turnPendingLabel = pendingLabel;
    if (!turnPendingLabel) {
      if (containsYouTubeUrl(message)) {
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

    const actions = (response && response.actions_taken) || [];
    const hasConsumedTrack = actions.some((act) => act.tool === 'add_track_from_staged' && act.status === 'success');

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
    if (response && response.active_presentation_id) {
      if (!updatedContext || updatedContext.id !== response.active_presentation_id) {
        // Look for title in creation actions if available
        const createAction = actions.find(
          (act) => act.tool === 'create_presentation' && act.presentation_id === response.active_presentation_id
        );
        const title = (createAction && createAction.details && createAction.details.title) || 'Nueva presentación';
        updatedContext = {
          id: response.active_presentation_id,
          title,
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
      stagedAudioId: nextStagedAudioId,
      draft: isProviderNotice ? state.draft : '',
      attachment: hasConsumedTrack ? null : state.attachment,
      activeContext: updatedContext,
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

    const notice = {
      code: (error && error.code) || 'agent_error',
      message: (error && error.message) || 'Error al procesar el mensaje.',
      retryable: error ? error.retryable !== false : true,
    };

    state = {
      ...state,
      pending: false,
      pendingLabel: 'Ressy está pensando...',
      notice,
      // Draft and attachment are strictly preserved
    };
    currentAbortController = null;
    notify();
  }

  function cancelTurn() {
    if (currentAbortController) {
      currentAbortController.abort();
      currentAbortController = null;
    }
    currentGenerationId += 1;
    if (state.pending) {
      state = { ...state, pending: false, pendingLabel: 'Ressy está pensando...' };
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
    setRouteContext,
    beginTurn,
    resolveTurn,
    failTurn,
    cancelTurn,
  };
}
