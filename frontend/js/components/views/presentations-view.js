import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createSpinner } from '../atoms/spinner.js';
import { createStatusMessage } from '../atoms/status-message.js';
import { createEmptyState } from '../molecules/empty-state.js';
import { createPresentationCard } from '../organisms/presentation-card.js';
import { createWelcomeHeader } from '../organisms/welcome-header.js';

/**
 * Pure view for presentations home catalog.
 * Orchestrates the protagonist Shirley AI chat hero card and presentation cards,
 * maintaining the warm canonical catalog underneath on desktop and mobile.
 *
 * @param {{
 *   presentations?: Array<any>,
 *   totalPresentations?: number,
 *   totalTracks?: number,
 *   isLoading?: boolean,
 *   errorMessage?: string | null,
 *   onCreateClick: () => void,
 *   onSelect: (id: string) => void,
 *   onRename: (id: string, currentTitle: string) => void,
 *   onDelete: (id: string, title: string) => void,
 *   onSendMessage?: (message: string) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createPresentationsView(props) {
  let {
    presentations = [],
    totalPresentations = 0,
    totalTracks = 0,
    isLoading = false,
    errorMessage = null,
    onCreateClick,
    onSelect,
    onRename,
    onDelete,
    onSendMessage,
  } = props;

  const errorBanner = createStatusMessage({ message: errorMessage || '', type: 'error' });
  const contentArea = createElement('div', { class: 'presentations-content' });

  const el = createElement('div', { class: 'presentations-view' }, [
    errorBanner.el,
    contentArea,
  ]);

  function renderContent() {
    contentArea.innerHTML = '';

    if (isLoading) {
      const loadingBox = createElement(
        'div',
        { class: 'presentations-loading' },
        [createSpinner({ size: 'lg' })]
      );
      contentArea.appendChild(loadingBox);
      return;
    }

    // Always render the Shirley AI Chat Hero at the top
    const welcomeHeader = createWelcomeHeader({
      greeting: '¡Hola, Shirley!',
      subtitle: 'Tu asistente personal para el escenario',
      phrase: 'Pídeme lo que necesites como si me escribieras por WhatsApp: crear presentaciones, preparar canciones o buscar letras.',
      totalPresentations,
      totalTracks,
      onCreatePresentation: onCreateClick,
      onSendMessage,
    });
    contentArea.appendChild(welcomeHeader.el);

    if (presentations.length === 0) {
      const createFirstBtn = createButton({
        text: 'Crear Primera Presentación',
        variant: 'secondary',
        onClick: onCreateClick,
      });

      const empty = createEmptyState({
        iconName: 'track',
        title: 'Tu biblioteca está lista',
        description: 'Escríbele a Shirley arriba qué repertorio quieres preparar o usa la opción manual.',
        actions: [createFirstBtn.el],
      });

      contentArea.appendChild(empty);
      return;
    }

    const listEl = createElement('ul', { class: 'presentations-list' });
    for (const pres of presentations) {
      const card = createPresentationCard({
        presentation: pres,
        onSelect,
        onRename,
        onDelete,
      });
      listEl.appendChild(card);
    }

    contentArea.appendChild(listEl);
  }

  renderContent();

  function update(nextProps = {}) {
    if (nextProps.presentations !== undefined) presentations = nextProps.presentations;
    if (nextProps.totalPresentations !== undefined) totalPresentations = nextProps.totalPresentations;
    if (nextProps.totalTracks !== undefined) totalTracks = nextProps.totalTracks;
    if (nextProps.isLoading !== undefined) isLoading = nextProps.isLoading;
    if (nextProps.errorMessage !== undefined) {
      errorMessage = nextProps.errorMessage;
      errorBanner.update({ message: errorMessage || '' });
    }
    if (nextProps.onCreateClick !== undefined) onCreateClick = nextProps.onCreateClick;
    if (nextProps.onSelect !== undefined) onSelect = nextProps.onSelect;
    if (nextProps.onRename !== undefined) onRename = nextProps.onRename;
    if (nextProps.onDelete !== undefined) onDelete = nextProps.onDelete;
    if (nextProps.onSendMessage !== undefined) onSendMessage = nextProps.onSendMessage;

    renderContent();
  }

  return { el, update };
}
