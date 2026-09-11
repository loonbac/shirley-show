import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createReorderControls } from '../molecules/reorder-controls.js';

/**
 * Creates a stage note item organism with warm-amber styling, tactile controls, and quick-editing.
 * @param {{
 *   item: { id: string, body: string, position: number },
 *   isFirst?: boolean,
 *   isLast?: boolean,
 *   onMoveUp: (item: any) => void,
 *   onMoveDown: (item: any) => void,
 *   onDelete: (item: any) => void,
 *   onEdit?: (item: any) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createNoteItem(props) {
  let {
    item,
    isFirst = false,
    isLast = false,
    onMoveUp,
    onMoveDown,
    onDelete,
    onEdit,
  } = props;

  const reorder = createReorderControls({
    isFirst,
    isLast,
    onMoveUp: () => onMoveUp(item),
    onMoveDown: () => onMoveDown(item),
  });

  const badgeEl = createElement('span', { class: 'note-item-badge' }, ['Nota de Escenario']);

  const editBtn = createIconButton({
    iconName: 'edit',
    ariaLabel: 'Editar nota',
    title: 'Editar nota',
    onClick: () => {
      if (typeof onEdit === 'function') {
        onEdit(item);
      }
    },
  });

  const deleteBtn = createIconButton({
    iconName: 'trash',
    ariaLabel: 'Eliminar nota',
    title: 'Eliminar nota',
    variant: 'danger',
    onClick: () => onDelete(item),
  });

  const actionsEl = createElement('div', { class: 'note-item-actions' }, [
    editBtn.el,
    deleteBtn.el,
  ]);

  const headerEl = createElement('div', { class: 'note-item-header' }, [
    badgeEl,
    actionsEl,
  ]);

  const bodyEl = createElement('div', { class: 'note-body' }, [item.body]);

  const contentEl = createElement('div', { class: 'note-item-content' }, [
    headerEl,
    bodyEl,
  ]);

  const el = createElement('li', { class: 'note-item', dataset: { id: item.id } }, [
    reorder.el,
    contentEl,
  ]);

  function update(nextProps = {}) {
    if (nextProps.item !== undefined) {
      item = nextProps.item;
      bodyEl.textContent = item.body;
    }
    if (nextProps.onEdit !== undefined) onEdit = nextProps.onEdit;
    if (nextProps.isFirst !== undefined) isFirst = nextProps.isFirst;
    if (nextProps.isLast !== undefined) isLast = nextProps.isLast;

    reorder.update({ isFirst, isLast });
  }

  return { el, update };
}
