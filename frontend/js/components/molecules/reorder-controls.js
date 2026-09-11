import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates atomic Up and Down reorder controls with boundary guards.
 * @param {{
 *   isFirst?: boolean,
 *   isLast?: boolean,
 *   disabled?: boolean,
 *   onMoveUp: () => void,
 *   onMoveDown: () => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: { isFirst?: boolean, isLast?: boolean, disabled?: boolean }) => void,
 * }}
 */
export function createReorderControls(props) {
  let {
    isFirst = false,
    isLast = false,
    disabled = false,
    onMoveUp,
    onMoveDown,
  } = props;

  const upIcon = createIcon({ name: 'arrow-up', size: 'sm' });
  const downIcon = createIcon({ name: 'arrow-down', size: 'sm' });

  const upBtn = createElement('button', {
    type: 'button',
    class: 'reorder-btn',
    title: 'Subir elemento',
    'aria-label': 'Subir elemento en la secuencia',
    disabled: isFirst || disabled,
    onClick: (e) => {
      e.stopPropagation();
      if (!isFirst && !disabled && typeof onMoveUp === 'function') {
        onMoveUp();
      }
    },
  }, [upIcon]);

  const downBtn = createElement('button', {
    type: 'button',
    class: 'reorder-btn',
    title: 'Bajar elemento',
    'aria-label': 'Bajar elemento en la secuencia',
    disabled: isLast || disabled,
    onClick: (e) => {
      e.stopPropagation();
      if (!isLast && !disabled && typeof onMoveDown === 'function') {
        onMoveDown();
      }
    },
  }, [downIcon]);

  const el = createElement('div', { class: 'reorder-controls' }, [upBtn, downBtn]);

  function update(nextProps = {}) {
    if (nextProps.isFirst !== undefined) isFirst = nextProps.isFirst;
    if (nextProps.isLast !== undefined) isLast = nextProps.isLast;
    if (nextProps.disabled !== undefined) disabled = nextProps.disabled;

    upBtn.disabled = isFirst || disabled;
    downBtn.disabled = isLast || disabled;
  }

  return { el, update };
}
