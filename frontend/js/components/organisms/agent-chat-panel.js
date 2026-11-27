import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createAgentMessageBubble } from '../molecules/agent-message-bubble.js';
import { createAgentTypingIndicator } from '../molecules/agent-typing-indicator.js';
import { createAgentChatComposer } from './agent-chat-composer.js';
import { useFocusTrap } from '../../behaviors/use-focus-trap.js';
import { useStickToBottom } from '../../behaviors/use-stick-to-bottom.js';

// Safe DOM helpers that support standard DOM and test environments like FakeNode
function removeNode(node) {
  if (!node) return;
  if (typeof node.remove === 'function') {
    node.remove();
  } else if (node.parentNode && typeof node.parentNode.removeChild === 'function') {
    node.parentNode.removeChild(node);
  }
}

function insertNodeBefore(parent, node, refNode) {
  if (!parent || !node) return;
  if (typeof parent.insertBefore === 'function') {
    parent.insertBefore(node, refNode || null);
    return;
  }
  // Fallback for test harnesses (FakeNode) lacking native insertBefore
  if (node.parentNode && typeof node.parentNode.removeChild === 'function') {
    node.parentNode.removeChild(node);
  }
  node.parentNode = parent;
  if (!Array.isArray(parent.childNodes)) {
    parent.childNodes = [];
  }
  if (!refNode) {
    parent.childNodes.push(node);
  } else {
    const idx = parent.childNodes.indexOf(refNode);
    if (idx !== -1) {
      parent.childNodes.splice(idx, 0, node);
    } else {
      parent.childNodes.push(node);
    }
  }
}

function replaceNode(parent, newChild, oldChild) {
  if (!parent || !newChild || !oldChild) return;
  if (typeof parent.replaceChild === 'function') {
    parent.replaceChild(newChild, oldChild);
    return;
  }
  insertNodeBefore(parent, newChild, oldChild);
  removeNode(oldChild);
}

function getNextSibling(parent, node) {
  if (!node) return parent ? (parent.firstChild || null) : null;
  if ('nextSibling' in node && node.nextSibling !== undefined) {
    return node.nextSibling;
  }
  if (parent && Array.isArray(parent.childNodes)) {
    const idx = parent.childNodes.indexOf(node);
    if (idx !== -1 && idx + 1 < parent.childNodes.length) {
      return parent.childNodes[idx + 1];
    }
  }
  return null;
}

function getLastChild(parent) {
  if (!parent) return null;
  return parent.lastElementChild || parent.lastChild || null;
}

/**
 * Organism managing the large Ressy AI conversational interface.
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
    pendingLabel = 'Ressy está pensando...',
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
    'Asistente Ressy',
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
    'aria-label': 'Historial de conversación con Ressy',
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

  // Empty state element (created once)
  const emptyIcon = createIcon({ name: 'bot', size: 'lg' });
  const emptyTitle = createElement('p', { class: 'agent-chat-empty-title' }, ['Ressy']);
  const emptyDesc = createElement('p', { class: 'agent-chat-empty-desc' }, [
    'Asistente de repertorio. Crea presentaciones, agrega letras, notas o pistas de audio con reproducción en vivo.',
  ]);
  const emptyEl = createElement('div', { class: 'agent-chat-empty-state' }, [emptyIcon, emptyTitle, emptyDesc]);

  // Persistent typing indicator (created once, updated via setLabel, attached/detached without touching bubbles)
  const typingIndicator = createAgentTypingIndicator({ label: pendingLabel });

  // Map of messageId -> { message, lastContent, component }
  const messageNodes = new Map();

  function syncMessages(newMessages) {
    if (!newMessages || newMessages.length === 0) {
      for (const [, item] of messageNodes) {
        if (item.component.el.parentNode === transcriptEl) {
          removeNode(item.component.el);
        }
      }
      messageNodes.clear();

      if (emptyEl.parentNode !== transcriptEl) {
        if (typingIndicator.el.parentNode === transcriptEl) {
          insertNodeBefore(transcriptEl, emptyEl, typingIndicator.el);
        } else {
          transcriptEl.appendChild(emptyEl);
        }
      }
      return;
    }

    if (emptyEl.parentNode === transcriptEl) {
      removeNode(emptyEl);
    }

    const newIds = new Set();
    for (let i = 0; i < newMessages.length; i++) {
      const msg = newMessages[i];
      const id = msg.id || `msg-${i}`;
      newIds.add(id);
    }

    for (const [id, item] of messageNodes) {
      if (!newIds.has(id)) {
        if (item.component.el.parentNode === transcriptEl) {
          removeNode(item.component.el);
        }
        messageNodes.delete(id);
      }
    }

    let prevNode = null;
    for (let i = 0; i < newMessages.length; i++) {
      const msg = newMessages[i];
      const id = msg.id || `msg-${i}`;
      let item = messageNodes.get(id);

      if (!item) {
        const component = createAgentMessageBubble({ message: msg });
        item = { message: msg, lastContent: msg.content, component };
        messageNodes.set(id, item);
      } else {
        const contentChanged = item.message !== msg || item.lastContent !== msg.content;
        const actionsChanged = JSON.stringify(item.message.actions) !== JSON.stringify(msg.actions);
        const attachmentChanged = JSON.stringify(item.message.attachment) !== JSON.stringify(msg.attachment);

        if (actionsChanged || attachmentChanged) {
          const newComponent = createAgentMessageBubble({ message: msg });
          if (item.component.el.parentNode === transcriptEl) {
            replaceNode(transcriptEl, newComponent.el, item.component.el);
          }
          item = { message: msg, lastContent: msg.content, component: newComponent };
          messageNodes.set(id, item);
        } else if (contentChanged) {
          item.component.update({ message: msg });
          item.message = msg;
          item.lastContent = msg.content;
        }
      }

      const el = item.component.el;
      const expectedNextSibling = getNextSibling(transcriptEl, prevNode);
      if (el !== expectedNextSibling) {
        insertNodeBefore(transcriptEl, el, expectedNextSibling);
      }
      prevNode = el;
    }

    if (typingIndicator.el.parentNode === transcriptEl) {
      if (getLastChild(transcriptEl) !== typingIndicator.el) {
        transcriptEl.appendChild(typingIndicator.el);
      }
    }
  }

  function syncTypingIndicator(isPending, label) {
    if (label && typeof typingIndicator.setLabel === 'function') {
      typingIndicator.setLabel(label);
    }

    if (isPending) {
      if (typingIndicator.el.parentNode !== transcriptEl) {
        transcriptEl.appendChild(typingIndicator.el);
      } else if (getLastChild(transcriptEl) !== typingIndicator.el) {
        transcriptEl.appendChild(typingIndicator.el);
      }
    } else {
      if (typingIndicator.el.parentNode === transcriptEl) {
        removeNode(typingIndicator.el);
      }
    }
  }

  function getNoticeMessage(code) {
    switch (code) {
      case 'provider_auth':
        return 'No se pudo autenticar con el proveedor de IA. Por favor, verifica la clave de API configurada en el servidor.';
      case 'provider_rate_limited':
        return 'Ressy está experimentando una alta demanda en este momento. Por favor, intenta de nuevo en unos instantes o utiliza los controles directos.';
      case 'provider_unavailable':
        return 'El proveedor de IA no está disponible temporalmente. Por favor, intenta de nuevo más tarde.';
      case 'agent_timeout':
        return 'La solicitud tardó más del límite de tiempo (120 s) y fue cancelada.';
      case 'iteration_limit':
        return 'Se alcanzó el límite de pasos permitidos para este turno. Si necesitas continuar, indícalo en un nuevo mensaje.';
      default:
        return 'Ocurrió un error al procesar la solicitud con el asistente de IA.';
    }
  }

  function renderNotice() {
    if (notice && (notice.message || notice.code)) {
      const text = notice.message || getNoticeMessage(notice.code);
      const lastMsg = messages[messages.length - 1];
      const hasCommittedActions = lastMsg && Array.isArray(lastMsg.actions) && lastMsg.actions.length > 0;
      const canRetry = Boolean(notice.retryable && !hasCommittedActions && typeof onRetry === 'function');

      const currentTextEl = noticeBannerEl.querySelector('.agent-chat-notice-text');
      const currentRetryBtn = noticeBannerEl.querySelector('.agent-chat-retry-btn');
      if (
        noticeBannerEl.style.display === 'flex' &&
        currentTextEl &&
        currentTextEl.textContent === text &&
        Boolean(currentRetryBtn) === canRetry
      ) {
        return;
      }

      noticeBannerEl.innerHTML = '';
      const noticeText = createElement('span', { class: 'agent-chat-notice-text' }, [text]);
      const children = [noticeText];

      if (canRetry) {
        const retryBtn = createElement(
          'button',
          {
            type: 'button',
            class: 'agent-chat-retry-btn',
            onClick: () => {
              if (typeof onRetry === 'function') {
                onRetry();
              }
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
      if (noticeBannerEl.style.display !== 'none') {
        noticeBannerEl.innerHTML = '';
        noticeBannerEl.style.display = 'none';
      }
    }
  }

  // Initial render
  syncMessages(messages);
  syncTypingIndicator(pending, pendingLabel);
  renderNotice();
  stickToBottom.scrollToBottom(false);

  if (isOpen) {
    focusTrap.activate();
  }

  function update(nextProps = {}) {
    let messagesChanged = false;
    let pendingChanged = false;
    let pendingLabelChanged = false;
    let noticeChanged = false;

    if (nextProps.isOpen !== undefined && nextProps.isOpen !== isOpen) {
      isOpen = nextProps.isOpen;
      el.className = `agent-chat-panel ${isOpen ? 'is-open' : 'is-closed'}`;
      el.setAttribute('aria-hidden', isOpen ? 'false' : 'true');

      if (isOpen) {
        focusTrap.activate();
        stickToBottom.scrollToBottom(false);
      } else {
        focusTrap.deactivate();
      }
    }

    if (nextProps.activeContext !== undefined && nextProps.activeContext !== activeContext) {
      activeContext = nextProps.activeContext;
      contextBadgeEl.textContent = formatContextText(activeContext);
    }

    if (
      nextProps.messages !== undefined &&
      (nextProps.messages !== messages || nextProps.messages.length !== messageNodes.size)
    ) {
      messages = nextProps.messages;
      messagesChanged = true;
    }

    if (nextProps.pending !== undefined && nextProps.pending !== pending) {
      pending = nextProps.pending;
      pendingChanged = true;
    }

    if (nextProps.pendingLabel !== undefined && nextProps.pendingLabel !== pendingLabel) {
      pendingLabel = nextProps.pendingLabel;
      pendingLabelChanged = true;
    }

    if (nextProps.draft !== undefined) {
      draft = nextProps.draft;
    }

    if (nextProps.attachment !== undefined) {
      attachment = nextProps.attachment;
    }

    if (nextProps.notice !== undefined && nextProps.notice !== notice) {
      notice = nextProps.notice;
      noticeChanged = true;
    }

    if (nextProps.onClose !== undefined) onClose = nextProps.onClose;
    if (nextProps.onSend !== undefined) onSend = nextProps.onSend;
    if (nextProps.onChangeDraft !== undefined) onChangeDraft = nextProps.onChangeDraft;
    if (nextProps.onAttach !== undefined) onAttach = nextProps.onAttach;
    if (nextProps.onAttachClick !== undefined) onAttachClick = nextProps.onAttachClick;
    if (nextProps.onRemoveAttachment !== undefined) onRemoveAttachment = nextProps.onRemoveAttachment;
    if (nextProps.onRetry !== undefined && nextProps.onRetry !== onRetry) {
      onRetry = nextProps.onRetry;
      noticeChanged = true;
    }

    composer.update({
      draft: nextProps.draft,
      attachment: nextProps.attachment,
      pending: nextProps.pending,
      onSend: nextProps.onSend,
      onChangeDraft: nextProps.onChangeDraft,
      onAttach: nextProps.onAttach,
      onAttachClick: nextProps.onAttachClick,
      onRemoveAttachment: nextProps.onRemoveAttachment,
    });

    if (messagesChanged) {
      syncMessages(messages);
      if (notice) {
        noticeChanged = true;
      }
    }

    if (pendingChanged || pendingLabelChanged) {
      syncTypingIndicator(pending, pendingLabel);
    }

    if (noticeChanged) {
      renderNotice();
    }

    if (messagesChanged || pendingChanged) {
      stickToBottom.scrollToBottom(false);
    }
  }

  function destroy() {
    focusTrap.destroy();
    stickToBottom.destroy();
    messageNodes.clear();
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
  if (context && context.id) {
    return `Contexto: ${context.id}`;
  }
  return 'Modo catálogo (creará un nuevo show)';
}
