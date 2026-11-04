import { createElement } from '../../utils/dom/create-element.js';
import { renderMarkdown } from '../../utils/markdown/render-markdown.js';
import { createIcon } from '../atoms/icon.js';
import { createAgentToolCard } from './agent-tool-card.js';

/**
 * Renders an individual conversation turn message bubble.
 *
 * Assistant replies are rendered as Markdown so headings, lists, tables and
 * code blocks survive. The renderer returns DOM nodes and never parses an HTML
 * string, so model output cannot inject markup. User messages stay plain text:
 * they are typed input, not a document.
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

  const textEl = createElement('div', { class: 'agent-message-text' });

  /** Fills the bubble body according to the message role. */
  function fillText() {
    while (textEl.firstChild) {
      textEl.removeChild(textEl.firstChild);
    }
    textEl.className = isUser
      ? 'agent-message-text'
      : 'agent-message-text agent-message-text--markdown';

    if (isUser) {
      textEl.appendChild(document.createTextNode(message.content || ''));
      return;
    }

    const rendered = renderMarkdown(message.content || '');
    if (rendered.length === 0) {
      textEl.appendChild(document.createTextNode(message.content || ''));
      return;
    }
    for (const node of rendered) {
      textEl.appendChild(node);
    }
  }

  fillText();

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
      fillText();
    }
  }

  return { el, update };
}
