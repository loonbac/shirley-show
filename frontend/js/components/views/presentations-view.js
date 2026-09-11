import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createSpinner } from '../atoms/spinner.js';
import { createStatusMessage } from '../atoms/status-message.js';
import { createEmptyState } from '../molecules/empty-state.js';
import { createPresentationCard } from '../organisms/presentation-card.js';
import { createWelcomeHeader } from '../organisms/welcome-header.js';

/**
 * Pure view for presentations home catalog.
 * Orchestrates welcome-header and rich presentation cards when presentations exist,
 * and maintains the warm canonical empty state when the catalog is empty.
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

    if (presentations.length === 0) {
      const createFirstBtn = createButton({
        text: 'Crear Primera Presentación',
        variant: 'primary',
        onClick: onCreateClick,
      });

      const empty = createEmptyState({
        iconName: 'track',
        title: 'Tu biblioteca está lista',
        description: 'Crea tu primera presentación para organizar tus pistas de audio, letras para el atril y notas de escenario.',
        actions: [createFirstBtn.el],
      });

      contentArea.appendChild(empty);
      return;
    }

    // When presentations exist, render the welcome header followed by the rich cards grid
    const welcomeHeader = createWelcomeHeader({
      greeting: '¡Hola, Shirley!',
      subtitle: 'Tu biblioteca de repertorios para el escenario',
      phrase: 'Organiza tus pistas, letras y notas para brillar en vivo.',
      totalPresentations,
      totalTracks,
      onCreatePresentation: onCreateClick,
    });

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

    contentArea.appendChild(welcomeHeader.el);
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

    renderContent();
  }

  return { el, update };
}
