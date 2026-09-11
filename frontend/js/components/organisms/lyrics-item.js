import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createReorderControls } from '../molecules/reorder-controls.js';

/**
 * Creates a lyrics item organism with large, stage-readable typography.
 * @param {{
 *   item: { id: string, body: string, position: number },
 *   isFirst?: boolean,
 *   isLast?: boolean,
 *   onMoveUp: (item: any) => void,
 *   onMoveDown: (item: any) => void,
 *   onDelete: (item: any) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createLyricsItem(props) {
  let {
    item,
    isFirst = false,
    isLast = false,
    onMoveUp,
    onMoveDown,
    onDelete,
  } = props;

  const reorder = createReorderControls({
    isFirst,
    isLast,
    onMoveUp: () => onMoveUp(item),
    onMoveDown: () => onMoveDown(item),
  });

  const badgeEl = createElement('span', { class: 'lyrics-item-badge' }, ['Letra']);

  const deleteBtn = createIconButton({
    iconName: 'trash',
    ariaLabel: 'Eliminar letra',
    variant: 'danger',
    onClick: () => onDelete(item),
  });

  const headerEl = createElement('div', { class: 'lyrics-item-header' }, [
    badgeEl,
    deleteBtn.el,
  ]);

  const bodyEl = createElement('div', { class: 'lyrics-body' }, [item.body]);

  const contentEl = createElement('div', { class: 'lyrics-item-content' }, [
    headerEl,
    bodyEl,
  ]);

  const el = createElement('li', { class: 'lyrics-item', dataset: { id: item.id } }, [
    reorder.el,
    contentEl,
  ]);

  function update(nextProps = {}) {
    if (nextProps.item !== undefined) {
      item = nextProps.item;
      bodyEl.textContent = item.body;
    }
    if (nextProps.isFirst !== undefined) isFirst = nextProps.isFirst;
    if (nextProps.isLast !== undefined) isLast = nextProps.isLast;

    reorder.update({ isFirst, isLast });
  }

  return { el, update };
}
