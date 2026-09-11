import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';

/**
 * Creates the top application bar organism.
 * @param {{
 *   title: string,
 *   onBack?: () => void,
 *   actions?: Array<HTMLElement>,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: { title?: string, actions?: Array<HTMLElement> }) => void,
 * }}
 */
export function createAppTopbar(props) {
  let { title, onBack, actions = [] } = props;

  const leftChildren = [];

  if (typeof onBack === 'function') {
    const backBtn = createIconButton({
      iconName: 'back',
      ariaLabel: 'Volver a presentaciones',
      onClick: onBack,
    });
    leftChildren.push(backBtn.el);
  }

  const titleEl = createElement('h1', { class: 'app-topbar-title' }, [title]);
  leftChildren.push(titleEl);

  const leftContainer = createElement('div', { class: 'app-topbar-left' }, leftChildren);
  const actionsContainer = createElement('div', { class: 'app-topbar-actions' }, actions);

  const el = createElement('header', { class: 'app-topbar' }, [
    leftContainer,
    actionsContainer,
  ]);

  function update(nextProps = {}) {
    if (nextProps.title !== undefined) {
      title = nextProps.title;
      titleEl.textContent = title;
    }
    if (nextProps.actions !== undefined) {
      actions = nextProps.actions;
      actionsContainer.innerHTML = '';
      for (const action of actions) {
        actionsContainer.appendChild(action);
      }
    }
  }

  return { el, update };
}
