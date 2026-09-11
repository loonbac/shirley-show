import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates the quick-add bar molecule with dedicated tactile actions for Track, Lyrics, and Note.
 * @param {{
 *   onAdd: (type: 'track' | 'lyrics' | 'note') => void,
 * }} props 
 * @returns {{ el: HTMLElement }}
 */
export function createQuickAddBar(props) {
  const { onAdd } = props;

  function createQuickAddButton(type, label, iconName) {
    const icon = createIcon({ name: iconName, size: 'sm' });
    const labelSpan = createElement('span', { class: 'quick-add-btn-label' }, [label]);

    return createElement(
      'button',
      {
        type: 'button',
        class: `quick-add-btn quick-add-btn-${type}`,
        'aria-label': `Agregar ${label.replace('+', '').trim()}`,
        onClick: () => {
          if (typeof onAdd === 'function') {
            onAdd(type);
          }
        },
      },
      [icon, labelSpan]
    );
  }

  const trackBtn = createQuickAddButton('track', '+ Pista', 'track');
  const lyricsBtn = createQuickAddButton('lyrics', '+ Letra', 'lyrics');
  const noteBtn = createQuickAddButton('note', '+ Nota', 'note');

  const el = createElement(
    'div',
    {
      class: 'quick-add-bar',
      role: 'region',
      'aria-label': 'Acciones rápidas de agregado',
    },
    [trackBtn, lyricsBtn, noteBtn]
  );

  return { el };
}
