import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createIcon } from '../atoms/icon.js';
import { createSpinner } from '../atoms/spinner.js';
import { createStatusMessage } from '../atoms/status-message.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createQuickAddBar } from '../molecules/quick-add-bar.js';
import { createEmptyState } from '../molecules/empty-state.js';
import { createTimelineLayout } from '../layouts/timeline-layout.js';
import { createTrackItem } from '../organisms/track-item.js';
import { createLyricsItem } from './../organisms/lyrics-item.js';
import { createNoteItem } from '../organisms/note-item.js';
import { useDragAndDrop } from '../../behaviors/use-drag-and-drop.js';

/**
 * Pure view for presentation detail and show execution timeline.
 * @param {{
 *   presentation?: { id: string, title: string },
 *   items?: Array<any>,
 *   isLoading?: boolean,
 *   errorMessage?: string | null,
 *   playbackState?: any,
 *   onRename: () => void,
 *   onDeletePresentation: () => void,
 *   onAddItemClick: () => void,
 *   onEnterStageMode?: () => void,
 *   onDownloadAllTracks?: () => void,
 *   onDownloadTrack?: (item: any) => void,
 *   onQuickAdd?: (type: 'track' | 'lyrics' | 'note') => void,
 *   onEditItem?: (item: any) => void,
 *   onReorderItem?: (itemId: string, targetPosition: number) => void,
 *   onTogglePlay: (item: any) => void,
 *   onSeek: (seconds: number) => void,
 *   onMoveUp: (item: any) => void,
 *   onMoveDown: (item: any) => void,
 *   onDeleteItem: (item: any) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 *   destroy: () => void,
 * }}
 */
export function createPresentationDetailView(props) {
  let {
    presentation = null,
    items = [],
    isLoading = false,
    errorMessage = null,
    playbackState = {},
    onRename,
    onDeletePresentation,
    onAddItemClick,
    onEnterStageMode,
    onDownloadAllTracks,
    onDownloadTrack,
    onQuickAdd,
    onEditItem,
    onReorderItem,
    onTogglePlay,
    onSeek,
    onMoveUp,
    onMoveDown,
    onDeleteItem,
  } = props;

  const errorBanner = createStatusMessage({ message: errorMessage || '', type: 'error' });
  const container = createElement('div', { class: 'presentation-detail-view' }, [errorBanner.el]);

  let itemComponents = [];
  let dragBehavior = null;

  function renderView() {
    if (dragBehavior) {
      dragBehavior.destroy();
      dragBehavior = null;
    }

    // Clear everything except error banner
    while (container.childNodes.length > 1) {
      container.removeChild(container.lastChild);
    }

    if (isLoading && !presentation) {
      const loadingBox = createElement(
        'div',
        { style: { display: 'flex', justifyContent: 'center', padding: '40px' } },
        [createSpinner({ size: 'lg' })]
      );
      container.appendChild(loadingBox);
      return;
    }

    if (!presentation) {
      return;
    }

    // Detail Header
    const titleEl = createElement('h2', { class: 'presentation-title-text' }, [presentation.title]);

    const renameBtn = createIconButton({
      iconName: 'edit',
      ariaLabel: 'Renombrar presentación',
      onClick: onRename,
    });

    const deleteBtn = createIconButton({
      iconName: 'trash',
      ariaLabel: 'Eliminar presentación completa',
      variant: 'danger',
      onClick: onDeletePresentation,
    });

    const stageBtn = createButton({
      text: 'Modo Escenario',
      variant: 'secondary',
      icon: createIcon({ name: 'play', size: 'sm' }),
      onClick: () => {
        if (typeof onEnterStageMode === 'function') {
          onEnterStageMode();
        }
      },
    });

    const addBtn = createButton({
      text: 'Agregar Elemento',
      variant: 'primary',
      icon: createIcon({ name: 'plus', size: 'sm' }),
      onClick: onAddItemClick,
    });

    const headerLeft = createElement('div', { class: 'presentation-header-left' }, [
      titleEl,
      renameBtn.el,
      deleteBtn.el,
    ]);

    const headerRightChildren = [];

    const hasTracks = items.some((it) => it.type === 'track');
    if (hasTracks) {
      const downloadAllBtn = createButton({
        text: 'Descargar todas',
        variant: 'secondary',
        icon: createIcon({ name: 'download', size: 'sm' }),
        onClick: () => {
          if (typeof onDownloadAllTracks === 'function') {
            onDownloadAllTracks();
          }
        },
      });
      headerRightChildren.push(downloadAllBtn.el);
    }

    headerRightChildren.push(stageBtn.el);
    headerRightChildren.push(addBtn.el);

    const headerRight = createElement('div', { class: 'presentation-header-right' }, headerRightChildren);

    const detailHeader = createElement('div', { class: 'presentation-title-row' }, [
      headerLeft,
      headerRight,
    ]);

    // Prominent Quick Add Bar for immediate 1-tap addition
    const quickAddBar = createQuickAddBar({
      onAdd: (type) => {
        if (typeof onQuickAdd === 'function') {
          onQuickAdd(type);
        } else if (typeof onAddItemClick === 'function') {
          onAddItemClick();
        }
      },
    });

    const quickAddSection = createElement('div', { class: 'presentation-quick-add-section' }, [
      quickAddBar.el,
    ]);

    // Timeline or Empty State
    let timelineContent;

    if (items.length === 0) {
      const addFirstBtn = createButton({
        text: 'Agregar Primer Elemento',
        variant: 'primary',
        onClick: onAddItemClick,
      });

      timelineContent = createEmptyState({
        iconName: 'note',
        title: 'El show está vacío',
        description: 'Usa la barra superior para agregar pistas de audio, letras en tipografía grande o notas de escenario con indicaciones.',
        actions: [addFirstBtn.el],
      });
    } else {
      const timeline = createTimelineLayout();
      itemComponents = [];
      const itemNodes = [];

      items.forEach((item, index) => {
        const isFirst = index === 0;
        const isLast = index === items.length - 1;

        let comp;
        if (item.type === 'track') {
          comp = createTrackItem({
            item,
            isFirst,
            isLast,
            playbackState,
            onTogglePlay,
            onSeek,
            onMoveUp,
            onMoveDown,
            onDelete: onDeleteItem,
            onEdit: onEditItem,
            onDownload: onDownloadTrack,
          });
        } else if (item.type === 'lyrics') {
          comp = createLyricsItem({
            item,
            isFirst,
            isLast,
            onMoveUp,
            onMoveDown,
            onDelete: onDeleteItem,
            onEdit: onEditItem,
          });
        } else {
          comp = createNoteItem({
            item,
            isFirst,
            isLast,
            onMoveUp,
            onMoveDown,
            onDelete: onDeleteItem,
            onEdit: onEditItem,
          });
        }

        itemComponents.push({ id: item.id, comp });
        itemNodes.push(comp.el);
      });

      timeline.setItems(itemNodes);
      timelineContent = timeline.el;

      // Attach headless drag-and-drop to the timeline items list
      if (items.length > 1) {
        const itemsOl = timelineContent.querySelector('.timeline-items');
        if (itemsOl) {
          dragBehavior = useDragAndDrop({
            listEl: itemsOl,
            handleSelector: '.drag-handle',
            itemSelector: '[data-id]',
            onReorder: (itemId, targetPosition) => {
              if (typeof onReorderItem === 'function') {
                onReorderItem(itemId, targetPosition);
              }
            },
          });
        }
      }
    }

    container.appendChild(detailHeader);
    container.appendChild(quickAddSection);
    container.appendChild(timelineContent);
  }

  renderView();

  function update(nextProps = {}) {
    let needsFullReRender = false;

    if (nextProps.presentation !== undefined) {
      presentation = nextProps.presentation;
      needsFullReRender = true;
    }
    if (nextProps.items !== undefined) {
      items = nextProps.items;
      needsFullReRender = true;
    }
    if (nextProps.isLoading !== undefined) {
      isLoading = nextProps.isLoading;
      needsFullReRender = true;
    }
    if (nextProps.errorMessage !== undefined) {
      errorMessage = nextProps.errorMessage;
      errorBanner.update({ message: errorMessage || '' });
    }
    if (nextProps.onQuickAdd !== undefined) {
      onQuickAdd = nextProps.onQuickAdd;
    }
    if (nextProps.onEditItem !== undefined) {
      onEditItem = nextProps.onEditItem;
    }
    if (nextProps.onEnterStageMode !== undefined) {
      onEnterStageMode = nextProps.onEnterStageMode;
    }
    if (nextProps.onDownloadAllTracks !== undefined) {
      onDownloadAllTracks = nextProps.onDownloadAllTracks;
    }
    if (nextProps.onDownloadTrack !== undefined) {
      onDownloadTrack = nextProps.onDownloadTrack;
    }
    if (nextProps.onReorderItem !== undefined) {
      onReorderItem = nextProps.onReorderItem;
    }

    if (nextProps.playbackState !== undefined) {
      playbackState = nextProps.playbackState;
      if (!needsFullReRender && itemComponents.length > 0) {
        for (const { comp } of itemComponents) {
          if (typeof comp.update === 'function') {
            comp.update({ playbackState });
          }
        }
        return;
      }
    }

    if (needsFullReRender) {
      renderView();
    }
  }

  function destroy() {
    if (dragBehavior) {
      dragBehavior.destroy();
      dragBehavior = null;
    }
  }

  return { el: container, update, destroy };
}
