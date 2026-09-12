import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

const ATTENTION_STORAGE_KEY = 'ressy_bubble_attention_dismissed';

function hasDismissedAttention() {
  try {
    if (typeof localStorage !== 'undefined') {
      return localStorage.getItem(ATTENTION_STORAGE_KEY) === 'true';
    }
  } catch {
    // fallback for environments without localStorage
  }
  return false;
}

function dismissAttention() {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(ATTENTION_STORAGE_KEY, 'true');
    }
  } catch {
    // fallback for environments without localStorage
  }
}

/**
 * Creates the global floating bubble launcher for the Ressy AI Assistant.
 * Floating in the bottom-right corner, always visible and writeable across views (except Stage Mode).
 * Features smooth gold pulse, avatar icon toggling (sparkles / close), and first-time attention badge.
 *
 * @param {{
 *   onClick?: () => void,
 *   isOpen?: boolean,
 *   isHidden?: boolean,
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   badgeEl: HTMLElement,
 *   update: (props: { onClick?: () => void, isOpen?: boolean, isHidden?: boolean }) => void,
 *   dismissAttentionBadge: () => void,
 * }}
 */
export function createAgentFloatingTrigger(props = {}) {
  let { onClick, isOpen = false, isHidden = false } = props;
  let showBadge = !hasDismissedAttention();

  const iconSparkles = createIcon({ name: 'sparkles', size: 'lg', className: 'agent-chat-floating-icon-sparkles' });
  const iconClose = createIcon({ name: 'close', size: 'base', className: 'agent-chat-floating-icon-close' });

  const avatarEl = createElement('div', { class: 'agent-chat-floating-avatar' }, [iconSparkles, iconClose]);

  const badgeEl = createElement(
    'span',
    {
      class: `agent-chat-floating-badge ${showBadge ? '' : 'is-dismissed'}`,
      'aria-label': 'Asistente Ressy disponible',
    },
    ['✨ Ressy']
  );

  const srText = createElement('span', { class: 'agent-chat-floating-sr-only' }, [
    isOpen ? 'Cerrar asistente Ressy' : 'Abrir asistente Ressy',
  ]);

  function handleDismissBadge() {
    if (showBadge) {
      showBadge = false;
      dismissAttention();
      badgeEl.classList.add('is-dismissed');
    }
  }

  const el = createElement(
    'button',
    {
      type: 'button',
      class: `agent-chat-floating-trigger ${isOpen ? 'is-open' : ''} ${isHidden ? 'is-hidden' : ''}`,
      'aria-label': isOpen ? 'Cerrar asistente Ressy' : 'Abrir asistente Ressy',
      'aria-haspopup': 'dialog',
      'aria-expanded': isOpen ? 'true' : 'false',
      title: 'Asistente Ressy (Ctrl+K / ⌘K)',
      onClick: (e) => {
        e.stopPropagation();
        handleDismissBadge();
        if (typeof onClick === 'function') {
          onClick();
        }
      },
    },
    [avatarEl, badgeEl, srText]
  );

  function update(nextProps = {}) {
    if (nextProps.onClick !== undefined) {
      onClick = nextProps.onClick;
    }
    if (nextProps.isOpen !== undefined) {
      isOpen = Boolean(nextProps.isOpen);
      if (isOpen) {
        handleDismissBadge();
      }
    }
    if (nextProps.isHidden !== undefined) {
      isHidden = Boolean(nextProps.isHidden);
    }

    el.className = `agent-chat-floating-trigger ${isOpen ? 'is-open' : ''} ${isHidden ? 'is-hidden' : ''}`;
    el.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
    el.setAttribute('aria-label', isOpen ? 'Cerrar asistente Ressy' : 'Abrir asistente Ressy');
    srText.textContent = isOpen ? 'Cerrar asistente Ressy' : 'Abrir asistente Ressy';
  }

  return {
    el,
    badgeEl,
    update,
    dismissAttentionBadge: handleDismissBadge,
  };
}

/**
 * Creates the primary navigation trigger for the Ressy AI Assistant chat.
 * Enforces a minimum tactile target of >= 48x48px, high-contrast visible label "Ressy",
 * and prominent gold brand styling.
 *
 * @param {{
 *   onClick: () => void,
 *   isOpen?: boolean,
 *   variant?: 'standard' | 'floating',
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createAgentChatTrigger(props = {}) {
  if (props.variant === 'floating') {
    return createAgentFloatingTrigger(props);
  }

  let { onClick, isOpen = false } = props;

  const icon = createIcon({ name: 'sparkles', size: 'base', className: 'agent-chat-trigger-icon' });
  const srPrefix = createElement('span', { class: 'agent-chat-trigger-sr-only' }, ['Asistente ']);
  const labelText = createElement('span', { class: 'agent-chat-trigger-text' }, ['Ressy']);
  const kbdHint = createElement('kbd', { class: 'agent-chat-trigger-kbd' }, ['⌘K']);

  const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform || '');
  kbdHint.textContent = isMac ? '⌘K' : 'Ctrl+K';

  const el = createElement(
    'button',
    {
      type: 'button',
      class: `agent-chat-trigger ${isOpen ? 'is-active' : ''}`,
      'aria-label': 'Abrir asistente Ressy',
      'aria-haspopup': 'dialog',
      'aria-expanded': isOpen ? 'true' : 'false',
      onClick: (e) => {
        e.stopPropagation();
        if (typeof onClick === 'function') {
          onClick();
        }
      },
    },
    [icon, srPrefix, labelText, kbdHint]
  );

  function update(nextProps = {}) {
    if (nextProps.onClick !== undefined) {
      onClick = nextProps.onClick;
    }
    if (nextProps.isOpen !== undefined) {
      isOpen = nextProps.isOpen;
      el.className = `agent-chat-trigger ${isOpen ? 'is-active' : ''}`;
      el.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
    }
  }

  return { el, update };
}
