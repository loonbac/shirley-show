import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates an atomic drag handle molecule with touch-action: none,
 * keyboard accessibility, and a clear visual grip affordance.
 *
 * @param {{
 *   ariaLabel?: string,
 *   title?: string,
 *   disabled?: boolean,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createDragHandle(props = {}) {
  let {
    ariaLabel = 'Arrastrar para reordenar',
    title = 'Arrastrar para reordenar',
    disabled = false,
  } = props;

  const gripIcon = createIcon({ name: 'grip', size: 'base' });

  const el = createElement(
    'button',
    {
      type: 'button',
      class: 'drag-handle',
      'aria-label': ariaLabel,
      title,
      disabled,
    },
    [gripIcon]
  );

  function update(nextProps = {}) {
    if (nextProps.ariaLabel !== undefined) {
      ariaLabel = nextProps.ariaLabel;
      el.setAttribute('aria-label', ariaLabel);
    }
    if (nextProps.title !== undefined) {
      title = nextProps.title;
      el.setAttribute('title', title);
    }
    if (nextProps.disabled !== undefined) {
      disabled = Boolean(nextProps.disabled);
      el.disabled = disabled;
    }
  }

  return { el, update };
}
