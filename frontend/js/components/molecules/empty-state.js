import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates an informative empty state molecule with an optional action button.
 * @param {{
 *   iconName?: string,
 *   title: string,
 *   description?: string,
 *   actions?: Array<HTMLElement>,
 * }} props 
 * @returns {HTMLElement}
 */
export function createEmptyState(props) {
  const { iconName, title, description, actions = [] } = props;

  const children = [];

  if (iconName) {
    const iconEl = createIcon({ name: iconName, size: 'xl', className: 'empty-state-icon' });
    children.push(iconEl);
  }

  children.push(createElement('h3', { class: 'empty-state-title' }, [title]));

  if (description) {
    children.push(createElement('p', { class: 'empty-state-description' }, [description]));
  }

  if (actions && actions.length > 0) {
    children.push(createElement('div', { class: 'empty-state-actions' }, actions));
  }

  return createElement('div', { class: 'empty-state' }, children);
}
