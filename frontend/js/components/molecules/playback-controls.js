import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createRangeInput } from '../atoms/range-input.js';
import { formatDuration } from '../../utils/format/duration.js';

/**
 * Creates accessible playback controls with scrubber and time indicators.
 * @param {{
 *   isPlaying?: boolean,
 *   currentTime?: number,
 *   duration?: number | null,
 *   onTogglePlay: () => void,
 *   onSeek: (seconds: number) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: { isPlaying?: boolean, currentTime?: number, duration?: number | null }) => void,
 * }}
 */
export function createPlaybackControls(props) {
  let {
    isPlaying = false,
    currentTime = 0,
    duration = null,
    onTogglePlay,
    onSeek,
  } = props;

  let isScrubbing = false;

  let playIcon = createIcon({ name: isPlaying ? 'pause' : 'play' });

  const toggleBtn = createElement(
    'button',
    {
      type: 'button',
      class: 'playback-toggle-btn',
      'aria-label': isPlaying ? 'Pausar audio' : 'Reproducir audio',
      onClick: (e) => {
        e.stopPropagation();
        if (typeof onTogglePlay === 'function') {
          onTogglePlay();
        }
      },
    },
    [playIcon]
  );

  const currentTimeLabel = createElement('span', { class: 'playback-time-label' }, [
    formatDuration(currentTime),
  ]);

  const durationLabel = createElement('span', { class: 'playback-time-label' }, [
    formatDuration(duration),
  ]);

  const rangeInput = createRangeInput({
    min: 0,
    max: duration && duration > 0 ? duration : 100,
    value: currentTime,
    step: 0.1,
    disabled: !duration || duration <= 0,
    onInput: (val) => {
      isScrubbing = true;
      currentTimeLabel.textContent = formatDuration(val);
    },
    onChange: (val) => {
      isScrubbing = false;
      if (typeof onSeek === 'function') {
        onSeek(val);
      }
    },
  });

  const scrubberRow = createElement('div', { class: 'playback-scrubber-row' }, [
    currentTimeLabel,
    rangeInput.el,
    durationLabel,
  ]);

  const mainRow = createElement('div', { class: 'playback-main-row' }, [
    toggleBtn,
    scrubberRow,
  ]);

  const el = createElement('div', { class: 'playback-controls' }, [mainRow]);

  function update(nextProps = {}) {
    if (nextProps.isPlaying !== undefined && nextProps.isPlaying !== isPlaying) {
      isPlaying = nextProps.isPlaying;
      toggleBtn.setAttribute('aria-label', isPlaying ? 'Pausar audio' : 'Reproducir audio');
      toggleBtn.innerHTML = '';
      playIcon = createIcon({ name: isPlaying ? 'pause' : 'play' });
      toggleBtn.appendChild(playIcon);
    }

    if (nextProps.duration !== undefined) {
      duration = nextProps.duration;
      durationLabel.textContent = formatDuration(duration);
      if (duration && duration > 0) {
        rangeInput.setMax(duration);
        rangeInput.setDisabled(false);
      } else {
        rangeInput.setDisabled(true);
      }
    }

    if (nextProps.currentTime !== undefined && !isScrubbing) {
      currentTime = nextProps.currentTime;
      currentTimeLabel.textContent = formatDuration(currentTime);
      rangeInput.setValue(currentTime);
    }
  }

  return { el, update };
}
