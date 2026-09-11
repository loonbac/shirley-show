import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates an accessible alert/status banner atom with aria-live.
 * @param {{
 *   message?: string,
 *   type?: 'error' | 'success' | 'info',
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: { message?: string, type?: 'error' | 'success' | 'info' }) => void,
 * }}
 */
export function createStatusMessage(props = {}) {
  let { message = '', type = 'error' } = props;

  const textEl = createElement('span', {}, [message]);

  const el = createElement(
    'div',
    {
      role: 'alert',
      'aria-live': 'polite',
      class: ['status-message', `status-message-${type}`].join(' '),
      style: { display: message ? 'flex' : 'none' },
    },
    [textEl]
  );

  function update(nextProps = {}) {
    if (nextProps.message !== undefined) {
      message = nextProps.message;
      textEl.textContent = message;
      el.style.display = message ? 'flex' : 'none';
    }
    if (nextProps.type !== undefined) {
      type = nextProps.type;
      el.className = ['status-message', `status-message-${type}`].join(' ');
    }
  }

  return { el, update };
}
