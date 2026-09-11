import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';

/**
 * Creates a catalog presentation card organism.
 * @param {{
 *   presentation: { id: string, title: string, created_at: string, updated_at: string },
 *   onSelect: (id: string) => void,
 *   onRename: (id: string, currentTitle: string) => void,
 *   onDelete: (id: string, title: string) => void,
 * }} props 
 * @returns {HTMLElement}
 */
export function createPresentationCard(props) {
  const { presentation, onSelect, onRename, onDelete } = props;

  const titleLink = createElement(
    'a',
    {
      href: `#/presentations/${encodeURIComponent(presentation.id)}`,
      class: 'presentation-card-link',
      onClick: (e) => {
        e.preventDefault();
        onSelect(presentation.id);
      },
    },
    [presentation.title]
  );

  const formattedDate = presentation.updated_at
    ? new Date(presentation.updated_at).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      })
    : '';

  const metaEl = createElement('span', { class: 'presentation-card-meta' }, [
    formattedDate ? `Actualizado: ${formattedDate}` : '',
  ]);

  const bodyEl = createElement('div', { class: 'presentation-card-body' }, [
    titleLink,
    metaEl,
  ]);

  const renameBtn = createIconButton({
    iconName: 'edit',
    ariaLabel: `Renombrar ${presentation.title}`,
    onClick: () => onRename(presentation.id, presentation.title),
  });

  const deleteBtn = createIconButton({
    iconName: 'trash',
    ariaLabel: `Eliminar ${presentation.title}`,
    variant: 'danger',
    onClick: () => onDelete(presentation.id, presentation.title),
  });

  const actionsEl = createElement('div', { class: 'presentation-card-actions' }, [
    renameBtn.el,
    deleteBtn.el,
  ]);

  return createElement('li', { class: 'presentation-card' }, [bodyEl, actionsEl]);
}
