import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';

/**
 * Creates the top application bar organism.
 * Supports title, optional back button, main section selector (Repertorios | Eventos),
 * and action items.
 *
 * @param {{
 *   title?: string,
 *   onBack?: () => void,
 *   actions?: Array<HTMLElement>,
 *   activeSection?: 'presentations' | 'events' | null,
 *   onSectionChange?: (section: 'presentations' | 'events') => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: {
 *     title?: string,
 *     actions?: Array<HTMLElement>,
 *     activeSection?: 'presentations' | 'events' | null,
 *     onSectionChange?: (section: 'presentations' | 'events') => void,
 *   }) => void,
 * }}
 */
export function createAppTopbar(props) {
  let { title = '', onBack, actions = [], activeSection = null, onSectionChange } = props;

  const leftChildren = [];

  if (typeof onBack === 'function') {
    const backBtn = createIconButton({
      iconName: 'back',
      ariaLabel: 'Volver',
      onClick: onBack,
    });
    leftChildren.push(backBtn.el);
  }

  const titleEl = createElement('h1', { class: 'app-topbar-title' }, [title]);
  leftChildren.push(titleEl);

  let navContainer = null;
  let btnRepertorios = null;
  let btnEventos = null;

  function renderNav() {
    if (!activeSection) {
      return null;
    }
    btnRepertorios = createElement('button', {
      type: 'button',
      class: `app-topbar-nav-btn ${activeSection === 'presentations' ? 'is-active' : ''}`,
      'aria-current': activeSection === 'presentations' ? 'page' : undefined,
    }, ['Repertorios']);

    btnRepertorios.addEventListener('click', () => {
      if (typeof onSectionChange === 'function') {
        onSectionChange('presentations');
      }
    });

    btnEventos = createElement('button', {
      type: 'button',
      class: `app-topbar-nav-btn ${activeSection === 'events' ? 'is-active' : ''}`,
      'aria-current': activeSection === 'events' ? 'page' : undefined,
    }, ['Eventos']);

    btnEventos.addEventListener('click', () => {
      if (typeof onSectionChange === 'function') {
        onSectionChange('events');
      }
    });

    navContainer = createElement('nav', {
      class: 'app-topbar-nav',
      'aria-label': 'Secciones principales',
    }, [btnRepertorios, btnEventos]);

    return navContainer;
  }

  const leftContainer = createElement('div', { class: 'app-topbar-left' }, leftChildren);
  const centerContainer = createElement('div', { class: 'app-topbar-center' }, []);
  const initialNav = renderNav();
  if (initialNav) {
    centerContainer.appendChild(initialNav);
  }

  const actionsContainer = createElement('div', { class: 'app-topbar-actions' }, actions);

  const el = createElement('header', { class: 'app-topbar' }, [
    leftContainer,
    centerContainer,
    actionsContainer,
  ]);

  function update(nextProps = {}) {
    if (nextProps.title !== undefined) {
      title = nextProps.title;
      titleEl.textContent = title;
    }
    if (nextProps.onSectionChange !== undefined) {
      onSectionChange = nextProps.onSectionChange;
    }
    if (nextProps.activeSection !== undefined) {
      activeSection = nextProps.activeSection;
      centerContainer.innerHTML = '';
      const updatedNav = renderNav();
      if (updatedNav) {
        centerContainer.appendChild(updatedNav);
      }
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
