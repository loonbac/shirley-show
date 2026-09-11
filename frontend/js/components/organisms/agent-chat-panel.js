import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createAgentMessageBubble } from '../molecules/agent-message-bubble.js';
import { createAgentTypingIndicator } from '../molecules/agent-typing-indicator.js';
import { createAgentChatComposer } from './agent-chat-composer.js';
import { useFocusTrap } from '../../behaviors/use-focus-trap.js';
import { useStickToBottom } from '../../behaviors/use-stick-to-bottom.js';

/**
 * Organism managing the large Shirley AI conversational interface.
 * Implements 580px slide-over on desktop, 100% overlay on mobile,
 * focus trap, active repertoire context badge, and auto-scrolling transcript.
 *
 * @param {{
 *   isOpen?: boolean,
 *   activeContext?: { id: string, title: string } | null,
 *   messages?: Array<any>,
 *   pending?: boolean,
 *   draft?: string,
 *   attachment?: any,
 *   notice?: { code: string, message: string, retryable: boolean } | null,
 *   onClose: () => void,
 *   onSend: (message: string) => void,
 *   onChangeDraft?: (text: string) => void,
 *   onAttach?: (file: File) => void,
 *   onAttachClick?: () => void,
 *   onRemoveAttachment?: () => void,
 *   onRetry?: () => void,
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   dialogEl: HTMLElement,
 *   contextBadgeEl: HTMLElement,
 *   closeBtnEl: HTMLElement,
 *   transcriptEl: HTMLElement,
 *   composer: any,
 *   update: (props: Partial<typeof props>) => void,
 *   destroy: () => void,
 * }}
 */
export function createAgentChatPanel(props) {
  let {
    isOpen = false,
    activeContext = null,
    messages = [],
    pending = false,
    draft = '',
    attachment = null,
    notice = null,
    onClose,
    onSend,
    onChangeDraft,
    onAttach,
    onAttachClick,
    onRemoveAttachment,
    onRetry,
  } = props;

  // Header Title
  const titleIcon = createIcon({ name: 'sparkles', size: 'base', className: 'agent-header-icon' });
  const titleText = createElement('h2', { id: 'agent-chat-title', class: 'agent-chat-header-title' }, [
    'Asistente Shirley',
  ]);
  const titleGroup = createElement('div', { class: 'agent-chat-title-group' }, [titleIcon, titleText]);

  // Context Badge
  const contextBadgeEl = createElement('div', {
    class: 'agent-chat-context-badge',
    role: 'status',
  }, [formatContextText(activeContext)]);

  // Accessible Close Button (touch target >= 48x48px)
  const closeIcon = createIcon({ name: 'close', size: 'base' });
  const closeBtnEl = createElement(
    'button',
    {
      type: 'button',
      class: 'agent-chat-close-btn',
      'aria-label': 'Cerrar asistente',
      title: 'Cerrar asistente (Escape)',
      onClick: () => {
        if (typeof onClose === 'function') {
          onClose();
        }
      },
    },
    [closeIcon]
  );

  const headerLeft = createElement('div', { class: 'agent-chat-header-left' }, [titleGroup, contextBadgeEl]);
  const headerRight = createElement('div', { class: 'agent-chat-header-right' }, [closeBtnEl]);
  const headerEl = createElement('header', { class: 'agent-chat-header' }, [headerLeft, headerRight]);

  // Transcript container
  const transcriptEl = createElement('div', {
    class: 'agent-chat-transcript',
    tabindex: '0',
    'aria-label': 'Historial de conversación con Shirley',
  });

  // Stick to bottom auto-scroll behavior
  const stickToBottom = useStickToBottom(transcriptEl);

  // Notice / error banner container
  const noticeBannerEl = createElement('div', {
    class: 'agent-chat-notice-banner',
    style: { display: 'none' },
  });

  // Composer organism
  const composer = createAgentChatComposer({
    draft,
    attachment,
    pending,
    onSend,
    onChangeDraft,
    onAttach,
    onAttachClick,
    onRemoveAttachment,
  });

  // Modal dialog surface
  const dialogEl = createElement(
    'section',
    {
      class: 'agent-chat-surface',
      role: 'dialog',
      'aria-modal': 'true',
      'aria-labelledby': 'agent-chat-title',
    },
    [headerEl, transcriptEl, noticeBannerEl, composer.el]
  );

  // Focus trap behavior
  const focusTrap = useFocusTrap(dialogEl, {
    onEscape: () => {
      if (typeof onClose === 'function') {
        onClose();
      }
    },
  });

  // Semi-transparent backdrop overlay
  const backdropEl = createElement('div', {
    class: 'agent-chat-backdrop',
    'aria-hidden': 'true',
    onClick: () => {
      if (typeof onClose === 'function') {
        onClose();
      }
    },
  });

  // Root container (hosts backdrop + sliding surface)
  const el = createElement(
    'div',
    {
      class: `agent-chat-panel ${isOpen ? 'is-open' : 'is-closed'}`,
      'aria-hidden': isOpen ? 'false' : 'true',
    },
    [backdropEl, dialogEl]
  );

  function renderTranscript() {
    transcriptEl.innerHTML = '';

    if (messages.length === 0) {
      const emptyIcon = createIcon({ name: 'sparkles', size: 'lg' });
      const emptyTitle = createElement('p', { class: 'agent-chat-empty-title' }, ['¡Hola! Soy Shirley']);
      const emptyDesc = createElement('p', { class: 'agent-chat-empty-desc' }, [
        'Puedo ayudarte a crear shows, agregar letras, notas o pistas de audio con playback listo.',
      ]);
      const emptyEl = createElement('div', { class: 'agent-chat-empty-state' }, [emptyIcon, emptyTitle, emptyDesc]);
      transcriptEl.appendChild(emptyEl);
    } else {
      for (const msg of messages) {
        const bubble = createAgentMessageBubble({ message: msg });
        transcriptEl.appendChild(bubble.el);
      }
    }

    if (pending) {
      const typing = createAgentTypingIndicator();
      transcriptEl.appendChild(typing.el);
    }

    renderNotice();
    stickToBottom.scrollToBottom(pending);
  }

  function getNoticeMessage(code) {
    switch (code) {
      case 'provider_auth':
        return 'No se pudo autenticar con el proveedor de IA. Por favor, verifica la clave de API configurada en el servidor.';
      case 'provider_rate_limited':
        return 'Shirley está experimentando una alta demanda en este momento. Por favor, intenta de nuevo en unos instantes o utiliza los controles directos.';
      case 'provider_unavailable':
        return 'El proveedor de IA no está disponible temporalmente. Por favor, intenta de nuevo más tarde.';
      case 'agent_timeout':
        return 'La solicitud tardó más del límite de tiempo (30 s) y fue cancelada.';
      case 'iteration_limit':
        return 'Se alcanzó el límite de pasos permitidos para este turno. Si necesitas continuar, indícalo en un nuevo mensaje.';
      default:
        return 'Ocurrió un error al procesar la solicitud con el asistente de IA.';
    }
  }

  function renderNotice() {
    noticeBannerEl.innerHTML = '';
    if (notice && (notice.message || notice.code)) {
      const text = notice.message || getNoticeMessage(notice.code);
      const noticeText = createElement('span', { class: 'agent-chat-notice-text' }, [text]);
      const children = [noticeText];

      // Safe retry rule: offer retry button ONLY if retryable and last message has no confirmed actions
      const lastMsg = messages[messages.length - 1];
      const hasCommittedActions = lastMsg && Array.isArray(lastMsg.actions) && lastMsg.actions.length > 0;

      if (notice.retryable && !hasCommittedActions && typeof onRetry === 'function') {
        const retryBtn = createElement(
          'button',
          {
            type: 'button',
            class: 'agent-chat-retry-btn',
            onClick: () => {
              onRetry();
            },
          },
          ['Reintentar']
        );
        children.push(retryBtn);
      }

      for (const child of children) {
        noticeBannerEl.appendChild(child);
      }
      noticeBannerEl.style.display = 'flex';
    } else {
      noticeBannerEl.style.display = 'none';
    }
  }

  // Initial render
  renderTranscript();

  if (isOpen) {
    focusTrap.activate();
  }

  function update(nextProps = {}) {
    let transcriptNeedsUpdate = false;

    if (nextProps.isOpen !== undefined && nextProps.isOpen !== isOpen) {
      isOpen = nextProps.isOpen;
      el.className = `agent-chat-panel ${isOpen ? 'is-open' : 'is-closed'}`;
      el.setAttribute('aria-hidden', isOpen ? 'false' : 'true');

      if (isOpen) {
        focusTrap.activate();
        transcriptNeedsUpdate = true;
      } else {
        focusTrap.deactivate();
      }
    }

    if (nextProps.activeContext !== undefined) {
      activeContext = nextProps.activeContext;
      contextBadgeEl.textContent = formatContextText(activeContext);
    }

    if (nextProps.messages !== undefined && nextProps.messages !== messages) {
      messages = nextProps.messages;
      transcriptNeedsUpdate = true;
    }

    if (nextProps.pending !== undefined && nextProps.pending !== pending) {
      pending = nextProps.pending;
      composer.update({ pending });
      transcriptNeedsUpdate = true;
    }

    if (nextProps.draft !== undefined) {
      draft = nextProps.draft;
      composer.update({ draft });
    }

    if (nextProps.attachment !== undefined) {
      attachment = nextProps.attachment;
      composer.update({ attachment });
    }

    if (nextProps.notice !== undefined) {
      notice = nextProps.notice;
      transcriptNeedsUpdate = true;
    }

    if (nextProps.onClose !== undefined) onClose = nextProps.onClose;
    if (nextProps.onSend !== undefined) {
      onSend = nextProps.onSend;
      composer.update({ onSend });
    }
    if (nextProps.onChangeDraft !== undefined) {
      onChangeDraft = nextProps.onChangeDraft;
      composer.update({ onChangeDraft });
    }
    if (nextProps.onAttach !== undefined) {
      onAttach = nextProps.onAttach;
      composer.update({ onAttach });
    }
    if (nextProps.onRemoveAttachment !== undefined) {
      onRemoveAttachment = nextProps.onRemoveAttachment;
      composer.update({ onRemoveAttachment });
    }
    if (nextProps.onRetry !== undefined) onRetry = nextProps.onRetry;

    if (transcriptNeedsUpdate) {
      renderTranscript();
    }
  }

  function destroy() {
    focusTrap.destroy();
    stickToBottom.destroy();
  }

  return {
    el,
    dialogEl,
    contextBadgeEl,
    closeBtnEl,
    transcriptEl,
    composer,
    update,
    destroy,
  };
}

function formatContextText(context) {
  if (context && context.title) {
    return `Contexto: ${context.title}`;
  }
  return 'Modo catálogo (creará un nuevo show)';
}
