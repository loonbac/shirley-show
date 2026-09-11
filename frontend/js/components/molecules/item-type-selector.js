import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates a segmented tab selector for item types: Pista, Letra, Nota.
 * @param {{
 *   selected?: 'track' | 'lyrics' | 'note',
 *   onChange?: (type: 'track' | 'lyrics' | 'note') => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   getSelected: () => 'track' | 'lyrics' | 'note',
 *   setSelected: (type: 'track' | 'lyrics' | 'note') => void,
 * }}
 */
export function createItemTypeSelector(props = {}) {
  let selected = props.selected || 'track';
  const onChange = props.onChange;

  const buttons = {};

  const types = [
    { type: 'track', label: 'Pista de Audio', icon: 'track' },
    { type: 'lyrics', label: 'Letra', icon: 'lyrics' },
    { type: 'note', label: 'Nota de Escenario', icon: 'note' },
  ];

  const container = createElement('div', {
    class: 'item-type-selector',
    role: 'tablist',
    'aria-label': 'Seleccionar tipo de elemento',
  });

  function updateActiveClasses() {
    for (const [t, btn] of Object.entries(buttons)) {
      if (t === selected) {
        btn.classList.add('active');
        btn.setAttribute('aria-selected', 'true');
      } else {
        btn.classList.remove('active');
        btn.setAttribute('aria-selected', 'false');
      }
    }
  }

  for (const { type, label, icon } of types) {
    const iconEl = createIcon({ name: icon, size: 'sm' });
    const btn = createElement(
      'button',
      {
        type: 'button',
        role: 'tab',
        dataset: { type },
        class: ['item-type-btn', type === selected ? 'active' : ''].filter(Boolean),
        'aria-selected': type === selected ? 'true' : 'false',
        onClick: () => {
          if (selected !== type) {
            selected = type;
            updateActiveClasses();
            if (typeof onChange === 'function') {
              onChange(type);
            }
          }
        },
      },
      [iconEl, createElement('span', {}, [label])]
    );

    buttons[type] = btn;
    container.appendChild(btn);
  }

  function getSelected() {
    return selected;
  }

  function setSelected(type) {
    if (selected !== type) {
      selected = type;
      updateActiveClasses();
    }
  }

  return { el: container, getSelected, setSelected };
}
