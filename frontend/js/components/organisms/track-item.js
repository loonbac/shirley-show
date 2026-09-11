import { createElement } from '../../utils/dom/create-element.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createReorderControls } from '../molecules/reorder-controls.js';
import { createPlaybackControls } from '../molecules/playback-controls.js';

/**
 * Formats bytes to human readable string (KB / MB).
 * @param {number} bytes 
 * @returns {string}
 */
function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return '';
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Creates a track item organism for the timeline with tactile actions and quick-editing.
 * @param {{
 *   item: { id: string, title: string, position: number, byte_size?: number, media_type?: string },
 *   isFirst?: boolean,
 *   isLast?: boolean,
 *   playbackState?: { currentTrackId: string | null, isPlaying: boolean, currentTime: number, duration: number | null, error: string | null },
 *   onTogglePlay: (item: any) => void,
 *   onSeek: (seconds: number) => void,
 *   onMoveUp: (item: any) => void,
 *   onMoveDown: (item: any) => void,
 *   onDelete: (item: any) => void,
 *   onEdit?: (item: any) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createTrackItem(props) {
  let {
    item,
    isFirst = false,
    isLast = false,
    playbackState = {},
    onTogglePlay,
    onSeek,
    onMoveUp,
    onMoveDown,
    onDelete,
    onEdit,
  } = props;

  const isCurrentTrack = playbackState.currentTrackId === item.id;
  const isPlaying = isCurrentTrack && Boolean(playbackState.isPlaying);
  const currentTime = isCurrentTrack ? playbackState.currentTime || 0 : 0;
  const duration = isCurrentTrack ? playbackState.duration : null;
  const errorMessage = isCurrentTrack ? playbackState.error : null;

  const reorder = createReorderControls({
    isFirst,
    isLast,
    onMoveUp: () => onMoveUp(item),
    onMoveDown: () => onMoveDown(item),
  });

  const titleBadge = createElement('span', { class: 'track-item-badge' }, ['Pista']);
  const titleText = createElement('span', { class: 'track-item-title-text' }, [item.title]);
  const titleEl = createElement('span', { class: 'track-item-title' }, [
    titleBadge,
    titleText,
  ]);

  const metaText = item.byte_size ? formatBytes(item.byte_size) : '';
  const metaEl = createElement('span', { class: 'track-item-meta' }, [metaText]);

  const editBtn = createIconButton({
    iconName: 'edit',
    ariaLabel: `Editar pista ${item.title}`,
    title: 'Editar pista',
    onClick: () => {
      if (typeof onEdit === 'function') {
        onEdit(item);
      }
    },
  });

  const deleteBtn = createIconButton({
    iconName: 'trash',
    ariaLabel: `Eliminar pista ${item.title}`,
    title: 'Eliminar pista',
    variant: 'danger',
    onClick: () => onDelete(item),
  });

  const headerActions = createElement('div', { class: 'track-item-actions' }, [
    metaEl,
    editBtn.el,
    deleteBtn.el,
  ]);

  const headerEl = createElement('div', { class: 'track-item-header' }, [
    titleEl,
    headerActions,
  ]);

  const playback = createPlaybackControls({
    isPlaying,
    currentTime,
    duration,
    onTogglePlay: () => onTogglePlay(item),
    onSeek,
  });

  const errorEl = createElement(
    'div',
    {
      class: 'track-item-error',
      role: 'alert',
      style: { display: errorMessage ? 'block' : 'none' },
    },
    [errorMessage || '']
  );

  const contentEl = createElement('div', { class: 'track-item-content' }, [
    headerEl,
    playback.el,
    errorEl,
  ]);

  const el = createElement('li', { class: 'track-item', dataset: { id: item.id } }, [
    reorder.el,
    contentEl,
  ]);

  function update(nextProps = {}) {
    if (nextProps.item !== undefined) {
      item = nextProps.item;
      titleText.textContent = item.title;
      metaEl.textContent = item.byte_size ? formatBytes(item.byte_size) : '';
      editBtn.update({ ariaLabel: `Editar pista ${item.title}` });
      deleteBtn.update({ ariaLabel: `Eliminar pista ${item.title}` });
    }
    if (nextProps.onEdit !== undefined) onEdit = nextProps.onEdit;
    if (nextProps.isFirst !== undefined) isFirst = nextProps.isFirst;
    if (nextProps.isLast !== undefined) isLast = nextProps.isLast;

    reorder.update({ isFirst, isLast });

    if (nextProps.playbackState !== undefined) {
      playbackState = nextProps.playbackState;
      const thisTrackActive = playbackState.currentTrackId === item.id;
      const trackPlaying = thisTrackActive && Boolean(playbackState.isPlaying);
      const trackCurrentTime = thisTrackActive ? playbackState.currentTime || 0 : 0;
      const trackDuration = thisTrackActive ? playbackState.duration : null;
      const trackError = thisTrackActive ? playbackState.error : null;

      playback.update({
        isPlaying: trackPlaying,
        currentTime: trackCurrentTime,
        duration: trackDuration,
      });

      if (trackError) {
        errorEl.textContent = trackError;
        errorEl.style.display = 'block';
      } else {
        errorEl.textContent = '';
        errorEl.style.display = 'none';
      }
    }
  }

  return { el, update };
}
