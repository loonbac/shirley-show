import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates the vertical timeline layout organism.
 * @param {{
 *   header?: HTMLElement,
 *   items?: Array<HTMLElement>,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   setHeader: (node: HTMLElement) => void,
 *   setItems: (itemNodes: Array<HTMLElement>) => void,
 * }}
 */
export function createTimelineLayout(props = {}) {
  const headerContainer = createElement('div', { class: 'timeline-header' }, [props.header].filter(Boolean));
  const itemsContainer = createElement('ol', { class: 'timeline-items' }, props.items || []);

  const el = createElement('div', { class: 'timeline-layout' }, [
    headerContainer,
    itemsContainer,
  ]);

  function setHeader(node) {
    headerContainer.innerHTML = '';
    if (node) headerContainer.appendChild(node);
  }

  function setItems(itemNodes) {
    itemsContainer.innerHTML = '';
    for (const node of itemNodes) {
      itemsContainer.appendChild(node);
    }
  }

  return { el, setHeader, setItems };
}
