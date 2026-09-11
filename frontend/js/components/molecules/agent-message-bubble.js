import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createAgentToolCard } from './agent-tool-card.js';

/**
 * Renders an individual conversation turn message bubble.
 * Strictly sanitizes model content by creating plain text nodes to prevent XSS.
 *
 * @param {{
 *   message: {
 *     id: string,
 *     role: 'user' | 'assistant',
 *     content: string,
 *     actions?: Array<any>,
 *     attachment?: { name: string, size?: number } | null,
 *     timestamp?: number,
 *   },
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createAgentMessageBubble(props) {
  let { message } = props;

  const isUser = message.role === 'user';
  const bubbleRoleClass = isUser ? 'agent-message-bubble--user' : 'agent-message-bubble--assistant';

  const avatar = isUser
    ? null
    : createElement('div', { class: 'agent-message-avatar' }, [
        createIcon({ name: 'sparkles', size: 'sm' }),
      ]);

  // Plain text node creation strictly prevents script execution
  const textEl = createElement('div', { class: 'agent-message-text' }, [message.content || '']);

  const metaChildren = [];
  if (message.attachment) {
    const attachIcon = createIcon({ name: 'paperclip', size: 'sm' });
    const attachLabel = createElement('span', {}, [message.attachment.name]);
    metaChildren.push(
      createElement('div', { class: 'agent-message-attachment-ref' }, [attachIcon, attachLabel])
    );
  }

  const actionsContainer = createElement('div', { class: 'agent-message-actions' });
  if (Array.isArray(message.actions) && message.actions.length > 0) {
    for (const act of message.actions) {
      const card = createAgentToolCard({ action: act });
      actionsContainer.appendChild(card.el);
    }
  }

  const bodyElements = [textEl];
  if (metaChildren.length > 0) {
    bodyElements.push(...metaChildren);
  }
  if (Array.isArray(message.actions) && message.actions.length > 0) {
    bodyElements.push(actionsContainer);
  }

  const contentEl = createElement('div', { class: 'agent-message-content' }, bodyElements);

  const elements = [];
  if (avatar) elements.push(avatar);
  elements.push(contentEl);

  const el = createElement(
    'div',
    {
      class: `agent-message-bubble ${bubbleRoleClass}`,
      dataset: { id: message.id },
    },
    elements
  );

  function update(nextProps = {}) {
    if (nextProps.message) {
      message = nextProps.message;
      textEl.textContent = message.content || '';
    }
  }

  return { el, update };
}
