import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates an animated loading indicator atom.
 * @param {{
 *   size?: 'sm' | 'base' | 'lg',
 * }} [props={}] 
 * @returns {HTMLElement}
 */
export function createSpinner(props = {}) {
  const { size = 'base' } = props;

  return createElement('div', {
    class: ['spinner', size === 'base' ? '' : `spinner-${size}`].filter(Boolean),
    role: 'status',
    'aria-label': 'Cargando...',
  });
}
