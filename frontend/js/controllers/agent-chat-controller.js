import { createAgentChatPanel } from '../components/organisms/agent-chat-panel.js';
import { sendAgentChatMessage } from '../data/agent-chat-api.js';
import { emitAgentRefresh } from '../core/state/agent-refresh.js';
import { activePresentationContextSignal } from '../core/state/active-presentation-context.js';

/**
 * Controller orchestrating the Shirley AI Agent conversational lifecycle.
 * Manages the singleton panel, network dispatch, reactive view refresh signals,
 * and ephemeral state synchronization.
 *
 * @param {{
 *   chatState: any,
 * }} deps
 * @returns {{
 *   open: () => void,
 *   close: () => void,
 *   toggle: () => void,
 *   panelEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createAgentChatController(deps) {
  const { chatState } = deps;

  // Sync route context changes into chat state
  const unsubscribeContext = activePresentationContextSignal.subscribe((context) => {
    chatState.setRouteContext(context);
  });

  const panel = createAgentChatPanel({
    isOpen: chatState.get().isOpen,
    activeContext: chatState.get().activeContext,
    messages: chatState.get().messages,
    pending: chatState.get().pending,
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
      draft: newState.draft,
      attachment: newState.attachment,
      notice: newState.notice,
    });
  });

  async function handleSend(userMessage) {
    const currentState = chatState.get();
    if (currentState.pending) return;

    const attachment = currentState.attachment;
    const turn = chatState.beginTurn({
      message: userMessage,
      attachment,
    });

    // Prune history for the backend: extract only textual user & assistant messages
    const history = currentState.messages
      .filter((m) => m.role === 'user' || m.role === 'assistant')
      .map((m) => ({ role: m.role, content: m.content }));

    const presentationId = currentState.activeContext ? currentState.activeContext.id : null;

    try {
      const response = await sendAgentChatMessage({
        message: userMessage,
        history,
        presentationId,
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

      for (const act of actions) {
        if (act.tool === 'create_presentation' && !refreshedCatalog) {
          emitAgentRefresh({ scope: 'catalog' });
          refreshedCatalog = true;
        } else if (
          act.tool === 'add_lyrics' ||
          act.tool === 'add_note' ||
          act.tool === 'add_track_from_staged'
        ) {
          const targetId = act.presentation_id || response.active_presentation_id;
          if (targetId) {
            emitAgentRefresh({ scope: 'presentation', presentationId: targetId });
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

  function open() {
    chatState.open();
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

  function destroy() {
    unsubscribeContext();
    unsubscribeState();
    chatState.cancelTurn();
    panel.destroy();
  }

  return {
    open,
    close,
    toggle,
    panelEl: panel.el,
    destroy,
  };
}
