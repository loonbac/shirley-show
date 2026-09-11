import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createIconButton } from '../molecules/icon-button.js';
import { createDragHandle } from '../molecules/drag-handle.js';
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
 * Formats seconds to MM:SS string.
 * @param {number} seconds 
 * @returns {string}
 */
function formatDuration(seconds) {
  if (!seconds || !Number.isFinite(seconds) || seconds < 0) return '0:00';
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

/**
 * Creates a track item organism for preparation or stage mode.
 * In stage mode: editing, deleting, reordering controls and drag handles
 * are omitted entirely; renders a large (>= 48x48px) tactile play/pause button.
 *
 * @param {{
 *   item: { id: string, title: string, position: number, byte_size?: number, media_type?: string },
 *   mode?: 'preparation' | 'stage',
 *   isFirst?: boolean,
 *   isLast?: boolean,
 *   playbackState?: { currentTrackId: string | null, isPlaying: boolean, currentTime: number, duration: number | null, error: string | null },
 *   onTogglePlay: (item: any) => void,
 *   onSeek?: (seconds: number) => void,
 *   onMoveUp?: (item: any) => void,
 *   onMoveDown?: (item: any) => void,
 *   onDelete?: (item: any) => void,
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
    mode = 'preparation',
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

  if (mode === 'stage') {
    // STAGE MODE: Distraction-free, large play button >= 48x48px, no edit/delete/drag controls
    const playIcon = createIcon({ name: isPlaying ? 'pause' : 'play', size: 'lg' });
    const stagePlayBtn = createElement(
      'button',
      {
        type: 'button',
        class: `track-stage-play-btn ${isPlaying ? 'is-playing' : ''}`,
        'aria-label': isPlaying ? `Pausar pista ${item.title}` : `Reproducir pista ${item.title}`,
        onClick: (e) => {
          e.stopPropagation();
          onTogglePlay(item);
        },
      },
      [playIcon]
    );

    const titleEl = createElement('div', { class: 'track-stage-title' }, [item.title]);
    const durationText = duration ? ` / ${formatDuration(duration)}` : '';
    const timeEl = createElement('div', { class: 'track-stage-time' }, [
      `${formatDuration(currentTime)}${durationText}`,
    ]);

    const infoEl = createElement('div', { class: 'track-stage-info' }, [
      titleEl,
      timeEl,
    ]);

    const errorEl = createElement(
      'div',
      {
        class: 'track-item-error',
        role: 'alert',
        style: { display: errorMessage ? 'block' : 'none' },
      },
      [errorMessage || '']
    );

    const stageContent = createElement('div', { class: 'track-stage-content' }, [
      stagePlayBtn,
      infoEl,
      errorEl,
    ]);

    const el = createElement(
      'li',
      { class: 'track-item track-item--stage', dataset: { id: item.id } },
      [stageContent]
    );

    function updateStage(nextProps = {}) {
      if (nextProps.item !== undefined) {
        item = nextProps.item;
        titleEl.textContent = item.title;
      }
      if (nextProps.onTogglePlay !== undefined) {
        onTogglePlay = nextProps.onTogglePlay;
      }
      if (nextProps.playbackState !== undefined) {
        playbackState = nextProps.playbackState;
        const trackActive = playbackState.currentTrackId === item.id;
        const trackPlaying = trackActive && Boolean(playbackState.isPlaying);
        const trackCurrentTime = trackActive ? playbackState.currentTime || 0 : 0;
        const trackDuration = trackActive ? playbackState.duration : null;
        const trackError = trackActive ? playbackState.error : null;

        stagePlayBtn.className = `track-stage-play-btn ${trackPlaying ? 'is-playing' : ''}`;
        stagePlayBtn.setAttribute(
          'aria-label',
          trackPlaying ? `Pausar pista ${item.title}` : `Reproducir pista ${item.title}`
        );

        while (stagePlayBtn.firstChild) {
          stagePlayBtn.removeChild(stagePlayBtn.firstChild);
        }
        stagePlayBtn.appendChild(createIcon({ name: trackPlaying ? 'pause' : 'play', size: 'lg' }));

        const durStr = trackDuration ? ` / ${formatDuration(trackDuration)}` : '';
        timeEl.textContent = `${formatDuration(trackCurrentTime)}${durStr}`;

        if (trackError) {
          errorEl.textContent = trackError;
          errorEl.style.display = 'block';
        } else {
          errorEl.textContent = '';
          errorEl.style.display = 'none';
        }
      }
    }

    return { el, update: updateStage };
  }

  // PREPARATION MODE: Full controls with dedicated drag handle and fallback reorder buttons
  const dragHandle = createDragHandle({
    ariaLabel: `Arrastrar pista ${item.title}`,
  });

  const reorder = createReorderControls({
    isFirst,
    isLast,
    onMoveUp: () => onMoveUp && onMoveUp(item),
    onMoveDown: () => onMoveDown && onMoveDown(item),
  });

  const reorderZone = createElement('div', { class: 'track-item-reorder-zone' }, [
    dragHandle.el,
    reorder.el,
  ]);

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
    onClick: () => onDelete && onDelete(item),
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
    reorderZone,
    contentEl,
  ]);

  function update(nextProps = {}) {
    if (nextProps.item !== undefined) {
      item = nextProps.item;
      titleText.textContent = item.title;
      metaEl.textContent = item.byte_size ? formatBytes(item.byte_size) : '';
      editBtn.update({ ariaLabel: `Editar pista ${item.title}` });
      deleteBtn.update({ ariaLabel: `Eliminar pista ${item.title}` });
      dragHandle.update({ ariaLabel: `Arrastrar pista ${item.title}` });
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
