import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates the golden typing indicator displayed during active in-flight requests.
 * Complies with aria-live polite region accessibility requirements.
 *
 * @returns {{ el: HTMLElement }}
 */
export function createAgentTypingIndicator() {
  const avatar = createElement('div', { class: 'agent-message-avatar' }, [
    createIcon({ name: 'sparkles', size: 'sm' }),
  ]);

  const dot1 = createElement('span', { class: 'agent-typing-dot' });
  const dot2 = createElement('span', { class: 'agent-typing-dot' });
  const dot3 = createElement('span', { class: 'agent-typing-dot' });

  const dotsContainer = createElement('span', { class: 'agent-typing-dots', 'aria-hidden': 'true' }, [
    dot1,
    dot2,
    dot3,
  ]);

  const textLabel = createElement('span', { class: 'agent-typing-label' }, ['Shirley está pensando...']);

  const bubble = createElement('div', { class: 'agent-typing-bubble' }, [
    dotsContainer,
    textLabel,
  ]);

  const el = createElement(
    'div',
    {
      class: 'agent-typing-indicator',
      role: 'status',
      'aria-live': 'polite',
      'aria-label': 'Shirley está pensando...',
    },
    [avatar, bubble]
  );

  return { el };
}
