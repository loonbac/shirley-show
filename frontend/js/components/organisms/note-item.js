import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createDragHandle } from '../molecules/drag-handle.js';
import { createReorderControls } from '../molecules/reorder-controls.js';

/**
 * Creates a stage note item organism for preparation or stage mode.
 * In stage mode: editing, deleting, reordering controls and drag handles
 * are omitted entirely; renders high-contrast warm-amber stage cue card.
 *
 * @param {{
 *   item: { id: string, body: string, position: number },
 *   mode?: 'preparation' | 'stage',
 *   isFirst?: boolean,
 *   isLast?: boolean,
 *   onMoveUp?: (item: any) => void,
 *   onMoveDown?: (item: any) => void,
 *   onDelete?: (item: any) => void,
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
    mode = 'preparation',
    isFirst = false,
    isLast = false,
    onMoveUp,
    onMoveDown,
    onDelete,
    onEdit,
  } = props;

  if (mode === 'stage') {
    // STAGE MODE: High-contrast warm-amber cue card, no mutation controls
    const cueBadge = createElement('span', { class: 'note-stage-badge' }, ['INDICACIÓN']);
    const bodyEl = createElement('div', { class: 'note-body note-body--stage' }, [item.body]);
    const el = createElement(
      'li',
      { class: 'note-item note-item--stage', dataset: { id: item.id } },
      [cueBadge, bodyEl]
    );

    function updateStage(nextProps = {}) {
      if (nextProps.item !== undefined) {
        item = nextProps.item;
        bodyEl.textContent = item.body;
      }
    }

    return { el, update: updateStage };
  }

  // PREPARATION MODE: Full controls with dedicated drag handle and fallback reorder buttons
  const dragHandle = createDragHandle({
    ariaLabel: 'Arrastrar nota',
  });

  const reorder = createReorderControls({
    isFirst,
    isLast,
    onMoveUp: () => onMoveUp && onMoveUp(item),
    onMoveDown: () => onMoveDown && onMoveDown(item),
  });

  const reorderZone = createElement('div', { class: 'note-item-reorder-zone' }, [
    dragHandle.el,
    reorder.el,
  ]);

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
    onClick: () => onDelete && onDelete(item),
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
    reorderZone,
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
