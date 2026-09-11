import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Pure organism for the warm library welcome header.
 * Displays a warm greeting, library summary metrics, and a prominent creation button.
 *
 * @param {{
 *   greeting?: string,
 *   subtitle?: string,
 *   phrase?: string,
 *   totalPresentations?: number,
 *   totalTracks?: number,
 *   onCreatePresentation: () => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (nextProps: Partial<typeof props>) => void,
 * }}
 */
export function createWelcomeHeader(props) {
  let {
    greeting = '¡Hola, Shirley!',
    subtitle = 'Tu biblioteca de repertorios para el escenario',
    phrase = 'Organiza tus pistas, letras y notas para brillar en vivo.',
    totalPresentations = 0,
    totalTracks = 0,
    onCreatePresentation,
  } = props;

  const greetingEl = createElement('h2', { class: 'welcome-header-title' }, [greeting]);
  const subtitleEl = createElement('p', { class: 'welcome-header-subtitle' }, [subtitle]);
  const phraseEl = createElement('p', { class: 'welcome-header-phrase' }, [phrase]);

  const introCol = createElement('div', { class: 'welcome-header-intro' }, [
    greetingEl,
    subtitleEl,
    phraseEl,
  ]);

  const presMetricText = createElement('span', {}, [
    `${totalPresentations} ${totalPresentations === 1 ? 'repertorio' : 'repertorios'}`,
  ]);
  const presMetric = createElement('div', { class: 'welcome-header-metric metric-presentations' }, [
    createIcon({ name: 'lyrics', size: 'sm' }),
    presMetricText,
  ]);

  const trackMetricText = createElement('span', {}, [
    `${totalTracks} ${totalTracks === 1 ? 'pista de audio' : 'pistas de audio'}`,
  ]);
  const trackMetric = createElement('div', { class: 'welcome-header-metric metric-tracks' }, [
    createIcon({ name: 'track', size: 'sm' }),
    trackMetricText,
  ]);

  const metricsBar = createElement('div', { class: 'welcome-header-metrics' }, [
    presMetric,
    trackMetric,
  ]);

  const createBtn = createButton({
    text: 'Nueva Presentación',
    variant: 'primary',
    icon: createIcon({ name: 'plus', size: 'base' }),
    onClick: () => {
      if (typeof onCreatePresentation === 'function') {
        onCreatePresentation();
      }
    },
  });
  createBtn.el.classList.add('welcome-header-create-btn');

  const actionCol = createElement('div', { class: 'welcome-header-actions' }, [
    createBtn.el,
  ]);

  const contentRow = createElement('div', { class: 'welcome-header-content' }, [
    introCol,
    actionCol,
  ]);

  const el = createElement('header', { class: 'welcome-header' }, [
    contentRow,
    metricsBar,
  ]);

  function update(nextProps = {}) {
    if (nextProps.greeting !== undefined) {
      greeting = nextProps.greeting;
      greetingEl.textContent = greeting;
    }
    if (nextProps.subtitle !== undefined) {
      subtitle = nextProps.subtitle;
      subtitleEl.textContent = subtitle;
    }
    if (nextProps.phrase !== undefined) {
      phrase = nextProps.phrase;
      phraseEl.textContent = phrase;
    }
    if (nextProps.totalPresentations !== undefined) {
      totalPresentations = nextProps.totalPresentations;
      presMetricText.textContent = `${totalPresentations} ${totalPresentations === 1 ? 'repertorio' : 'repertorios'}`;
    }
    if (nextProps.totalTracks !== undefined) {
      totalTracks = nextProps.totalTracks;
      trackMetricText.textContent = `${totalTracks} ${totalTracks === 1 ? 'pista de audio' : 'pistas de audio'}`;
    }
    if (nextProps.onCreatePresentation !== undefined) {
      onCreatePresentation = nextProps.onCreatePresentation;
    }
  }

  return { el, update };
}
