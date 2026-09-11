import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates the outer application shell layout.
 * @param {{
 *   header?: HTMLElement,
 *   main?: HTMLElement,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   setHeader: (node: HTMLElement) => void,
 *   setMain: (node: HTMLElement) => void,
 * }}
 */
export function createShellLayout(props = {}) {
  const headerContainer = createElement('header', { class: 'shell-header' }, [props.header].filter(Boolean));
  const mainContainer = createElement('main', { class: 'shell-main' }, [props.main].filter(Boolean));

  const el = createElement('div', { class: 'shell-layout' }, [
    headerContainer,
    mainContainer,
  ]);

  function setHeader(node) {
    headerContainer.innerHTML = '';
    if (node) headerContainer.appendChild(node);
  }

  function setMain(node) {
    mainContainer.innerHTML = '';
    if (node) mainContainer.appendChild(node);
  }

  return { el, setHeader, setMain };
}
