import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createTrackItem } from './track-item.js';
import { createLyricsItem } from './lyrics-item.js';
import { createNoteItem } from './note-item.js';

/**
 * Pure Stage Mode organism rendering a full-viewport distraction-free performance surface.
 * Omit all edit/delete/reorder controls.
 * Features safe-area topbar, prominent exit button, A- / A+ lyrics scaling controls,
 * subtle Screen Wake Lock indicator, and continuous single-source audio control.
 *
 * @param {{
 *   presentation: { id: string, title: string },
 *   items?: Array<any>,
 *   playbackState?: any,
 *   wakeLockStatus?: 'idle' | 'acquired' | 'unsupported' | 'denied' | 'released',
 *   onExit: () => void,
 *   onTogglePlay: (item: any) => void,
 *   onSeek?: (seconds: number) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 *   destroy: () => void,
 * }}
 */
export function createStageModeView(props) {
  let {
    presentation,
    items = [],
    playbackState = {},
    wakeLockStatus = 'idle',
    onExit,
    onTogglePlay,
    onSeek,
  } = props;

  let lyricsScale = 1.25;
  const MIN_SCALE = 0.8;
  const MAX_SCALE = 2.2;
  const SCALE_STEP = 0.2;

  let itemComponents = [];

  // Root fixed overlay
  const overlayEl = createElement('div', {
    class: 'stage-mode-overlay',
    role: 'dialog',
    'aria-modal': 'true',
    'aria-label': `Modo Escenario: ${presentation ? presentation.title : 'Presentación'}`,
  });

  overlayEl.style.setProperty('--stage-lyrics-scale', String(lyricsScale));

  // Exit Action Button (>= 48px touch target)
  const exitIcon = createIcon({ name: 'close', size: 'base' });
  const exitTextFull = createElement('span', { class: 'stage-mode-exit-text-full' }, ['Salir de Escenario']);
  const exitTextShort = createElement('span', { class: 'stage-mode-exit-text-short' }, ['Salir']);
  const exitBtn = createElement(
    'button',
    {
      type: 'button',
      class: 'stage-mode-exit-btn',
      'aria-label': 'Salir de Modo Escenario',
      onClick: () => {
        if (typeof onExit === 'function') {
          onExit();
        }
      },
    },
    [exitIcon, exitTextFull, exitTextShort]
  );

  // Title
  const titleEl = createElement('h1', { class: 'stage-mode-title' }, [
    presentation ? presentation.title : '',
  ]);

  // Typography scaling controls A- and A+ (>= 48px touch targets)
  const scaleDownBtn = createElement(
    'button',
    {
      type: 'button',
      class: 'stage-mode-scale-btn',
      title: 'Reducir tamaño de letra',
      'aria-label': 'Reducir tamaño de letra de las canciones',
      onClick: () => {
        lyricsScale = Math.max(MIN_SCALE, Math.round((lyricsScale - SCALE_STEP) * 100) / 100);
        overlayEl.style.setProperty('--stage-lyrics-scale', String(lyricsScale));
      },
    },
    ['A-']
  );

  const scaleUpBtn = createElement(
    'button',
    {
      type: 'button',
      class: 'stage-mode-scale-btn',
      title: 'Aumentar tamaño de letra',
      'aria-label': 'Aumentar tamaño de letra de las canciones',
      onClick: () => {
        lyricsScale = Math.min(MAX_SCALE, Math.round((lyricsScale + SCALE_STEP) * 100) / 100);
        overlayEl.style.setProperty('--stage-lyrics-scale', String(lyricsScale));
      },
    },
    ['A+']
  );

  const scaleControls = createElement('div', { class: 'stage-mode-scale-group' }, [
    scaleDownBtn,
    scaleUpBtn,
  ]);

  // Wake Lock Indicator
  const wakeLockDot = createElement('span', { class: 'stage-wake-lock-dot' });
  const wakeLockText = createElement('span', { class: 'stage-wake-lock-text' }, [
    wakeLockStatus === 'acquired' ? 'Pantalla activa' : '',
  ]);
  const wakeLockIndicator = createElement(
    'div',
    {
      class: `stage-wake-lock-indicator stage-wake-lock--${wakeLockStatus}`,
      'aria-live': 'polite',
    },
    [wakeLockDot, wakeLockText]
  );

  function updateWakeLockUI() {
    wakeLockIndicator.className = `stage-wake-lock-indicator stage-wake-lock--${wakeLockStatus}`;
    if (wakeLockStatus === 'acquired') {
      wakeLockText.textContent = 'Pantalla activa';
      wakeLockIndicator.title = 'Screen Wake Lock activo (la pantalla no se apagará)';
    } else {
      wakeLockText.textContent = '';
      wakeLockIndicator.title = '';
    }
  }
  updateWakeLockUI();

  // Topbar header
  const topbarRight = createElement('div', { class: 'stage-mode-topbar-right' }, [
    wakeLockIndicator,
    scaleControls,
    exitBtn,
  ]);

  const topbarEl = createElement('header', { class: 'stage-mode-topbar' }, [
    titleEl,
    topbarRight,
  ]);

  // Scroll Area
  const scrollArea = createElement('main', { class: 'stage-mode-scroll-area' });

  function renderItems() {
    while (scrollArea.firstChild) {
      scrollArea.removeChild(scrollArea.firstChild);
    }
    itemComponents = [];

    if (!items || items.length === 0) {
      const emptyEl = createElement('div', { class: 'stage-mode-empty' }, [
        createElement('p', {}, ['El show no tiene elementos todavía.']),
        createElement('p', { class: 'stage-mode-empty-sub' }, [
          'Sal de Modo Escenario para agregar pistas de audio, letras o notas.',
        ]),
      ]);
      scrollArea.appendChild(emptyEl);
      return;
    }

    const listEl = createElement('ul', { class: 'stage-mode-items' });

    items.forEach((item) => {
      let comp;
      if (item.type === 'track') {
        comp = createTrackItem({
          item,
          mode: 'stage',
          playbackState,
          onTogglePlay,
          onSeek,
        });
      } else if (item.type === 'lyrics') {
        comp = createLyricsItem({
          item,
          mode: 'stage',
        });
      } else {
        comp = createNoteItem({
          item,
          mode: 'stage',
        });
      }

      itemComponents.push({ id: item.id, comp });
      listEl.appendChild(comp.el);
    });

    scrollArea.appendChild(listEl);
  }

  renderItems();

  overlayEl.appendChild(topbarEl);
  overlayEl.appendChild(scrollArea);

  // Keyboard shortcut: Escape to exit Stage Mode
  function onKeyDown(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (typeof onExit === 'function') {
        onExit();
      }
    }
  }
  window.addEventListener('keydown', onKeyDown);

  function update(nextProps = {}) {
    let needsItemRerender = false;

    if (nextProps.presentation !== undefined) {
      presentation = nextProps.presentation;
      titleEl.textContent = presentation ? presentation.title : '';
    }

    if (nextProps.onExit !== undefined) onExit = nextProps.onExit;
    if (nextProps.onTogglePlay !== undefined) onTogglePlay = nextProps.onTogglePlay;
    if (nextProps.onSeek !== undefined) onSeek = nextProps.onSeek;

    if (nextProps.wakeLockStatus !== undefined) {
      wakeLockStatus = nextProps.wakeLockStatus;
      updateWakeLockUI();
    }

    if (nextProps.items !== undefined) {
      items = nextProps.items;
      needsItemRerender = true;
    }

    if (nextProps.playbackState !== undefined) {
      playbackState = nextProps.playbackState;
      if (!needsItemRerender && itemComponents.length > 0) {
        for (const { comp } of itemComponents) {
          if (typeof comp.update === 'function') {
            comp.update({ playbackState });
          }
        }
        return;
      }
    }

    if (needsItemRerender) {
      renderItems();
    }
  }

  function destroy() {
    window.removeEventListener('keydown', onKeyDown);
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    }
  }

  return {
    el: overlayEl,
    update,
    destroy,
  };
}
