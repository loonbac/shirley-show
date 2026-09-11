import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createDragHandle } from '../molecules/drag-handle.js';
import { createReorderControls } from '../molecules/reorder-controls.js';

/**
 * Creates a lyrics item organism for preparation or stage mode.
 * In stage mode: editing, deleting, reordering controls and drag handles
 * are omitted entirely; renders readable large display lyrics.
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
export function createLyricsItem(props) {
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
    // STAGE MODE: Distraction-free, large display typography, no mutation controls
    const bodyEl = createElement('div', { class: 'lyrics-body lyrics-body--stage' }, [item.body]);
    const el = createElement(
      'li',
      { class: 'lyrics-item lyrics-item--stage', dataset: { id: item.id } },
      [bodyEl]
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
    ariaLabel: 'Arrastrar letra',
  });

  const reorder = createReorderControls({
    isFirst,
    isLast,
    onMoveUp: () => onMoveUp && onMoveUp(item),
    onMoveDown: () => onMoveDown && onMoveDown(item),
  });

  const reorderZone = createElement('div', { class: 'lyrics-item-reorder-zone' }, [
    dragHandle.el,
    reorder.el,
  ]);

  const badgeEl = createElement('span', { class: 'lyrics-item-badge' }, ['Letra']);

  const editBtn = createIconButton({
    iconName: 'edit',
    ariaLabel: 'Editar letra',
    title: 'Editar letra',
    onClick: () => {
      if (typeof onEdit === 'function') {
        onEdit(item);
      }
    },
  });

  const deleteBtn = createIconButton({
    iconName: 'trash',
    ariaLabel: 'Eliminar letra',
    title: 'Eliminar letra',
    variant: 'danger',
    onClick: () => onDelete && onDelete(item),
  });

  const actionsEl = createElement('div', { class: 'lyrics-item-actions' }, [
    editBtn.el,
    deleteBtn.el,
  ]);

  const headerEl = createElement('div', { class: 'lyrics-item-header' }, [
    badgeEl,
    actionsEl,
  ]);

  const bodyEl = createElement('div', { class: 'lyrics-body' }, [item.body]);

  const contentEl = createElement('div', { class: 'lyrics-item-content' }, [
    headerEl,
    bodyEl,
  ]);

  const el = createElement('li', { class: 'lyrics-item', dataset: { id: item.id } }, [
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
