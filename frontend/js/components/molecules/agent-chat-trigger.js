import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates the primary navigation trigger for the Shirley AI Assistant chat.
 * Enforces a minimum tactile target of >= 48x48px, high-contrast visible label "Shirley",
 * and prominent gold brand styling.
 *
 * @param {{
 *   onClick: () => void,
 *   isOpen?: boolean,
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createAgentChatTrigger(props = {}) {
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
