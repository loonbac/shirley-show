import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Pure organism for the warm library welcome header & Shirley AI Agent Hero.
 * Serves as the central protagonist of the application: large greeting, prominent
 * chat composer with real-world singer examples, send trigger that opens the slide-over chat,
 * aggregate repertoire metrics, and manual presentation creation action.
 *
 * @param {{
 *   greeting?: string,
 *   subtitle?: string,
 *   phrase?: string,
 *   totalPresentations?: number,
 *   totalTracks?: number,
 *   onCreatePresentation?: () => void,
 *   onSendMessage?: (message: string) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   update: (nextProps: Partial<typeof props>) => void,
 * }}
 */
export function createWelcomeHeader(props = {}) {
  let {
    greeting = '¡Hola, Shirley!',
    subtitle = 'Tu asistente personal para el escenario',
    phrase = 'Pídeme lo que necesites como si me escribieras por WhatsApp: crear presentaciones, preparar canciones o buscar letras.',
    totalPresentations = 0,
    totalTracks = 0,
    onCreatePresentation,
    onSendMessage,
  } = props;

  // 1. Header & Greeting with avatar badge
  const avatarIcon = createIcon({ name: 'sparkles', size: 'lg', className: 'welcome-header-avatar-icon' });
  const avatarEl = createElement('div', { class: 'welcome-header-avatar' }, [avatarIcon]);

  const greetingEl = createElement('h2', { class: 'welcome-header-title' }, [greeting]);
  const subtitleEl = createElement('p', { class: 'welcome-header-subtitle' }, [subtitle]);
  const phraseEl = createElement('p', { class: 'welcome-header-phrase' }, [phrase]);

  const introTextGroup = createElement('div', { class: 'welcome-header-text-group' }, [
    greetingEl,
    subtitleEl,
    phraseEl,
  ]);

  const introRow = createElement('div', { class: 'welcome-header-intro' }, [
    avatarEl,
    introTextGroup,
  ]);

  // 2. Chat Hero Composer (The central protagonist)
  const chatInput = createElement('textarea', {
    class: 'welcome-header-chat-input',
    placeholder: 'Escríbeme… ej: créame la presentación Cumpleaños Rosa',
    rows: '2',
    'aria-label': 'Escribe una instrucción o mensaje para Shirley',
  });

  const sendIcon = createIcon({ name: 'send', size: 'base', className: 'welcome-header-send-icon' });
  const sendLabel = createElement('span', { class: 'welcome-header-send-label' }, ['Enviar']);
  const sendBtn = createElement(
    'button',
    {
      type: 'submit',
      class: 'welcome-header-chat-send btn btn-primary',
      'aria-label': 'Enviar mensaje a Shirley',
    },
    [sendIcon, sendLabel]
  );

  function submitHeroMessage(explicitText) {
    const textToSubmit = typeof explicitText === 'string' ? explicitText : chatInput.value;
    const trimmed = (textToSubmit || '').trim();
    if (!trimmed) {
      chatInput.focus();
      return;
    }

    chatInput.value = '';
    if (typeof chatInput.style !== 'undefined') {
      chatInput.style.height = 'auto';
    }

    if (typeof onSendMessage === 'function') {
      onSendMessage(trimmed);
    }
  }

  chatInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submitHeroMessage();
    }
  });

  chatInput.addEventListener('input', () => {
    chatInput.style.height = 'auto';
    const newHeight = Math.min(Math.max(chatInput.scrollHeight, 56), 180);
    chatInput.style.height = `${newHeight}px`;
  });

  const chatInputBox = createElement('div', { class: 'welcome-header-chat-box' }, [
    chatInput,
    sendBtn,
  ]);

  const chatForm = createElement(
    'form',
    {
      class: 'welcome-header-chat-form',
      onSubmit: (e) => {
        e.preventDefault();
        submitHeroMessage();
      },
    },
    [chatInputBox]
  );

  // Quick suggestion chips for singer shortcuts
  const hintsLabel = createElement('span', { class: 'welcome-header-hints-label' }, ['Prueba pidiéndome:']);
  const suggestionItems = [
    'Créame la presentación Cumpleaños Rosa',
    'Agrega la letra de Sabor a Mí',
    'Prepara un repertorio para el sábado',
  ];

  const suggestionChips = suggestionItems.map((hintText) => {
    return createElement(
      'button',
      {
        type: 'button',
        class: 'welcome-header-hint-chip',
        onClick: () => {
          submitHeroMessage(hintText);
        },
      },
      [hintText]
    );
  });

  const hintsRow = createElement('div', { class: 'welcome-header-hints' }, [
    hintsLabel,
    ...suggestionChips,
  ]);

  const heroChatArea = createElement('div', { class: 'welcome-header-hero-chat' }, [
    chatForm,
    hintsRow,
  ]);

  // 3. Library metrics & secondary action bar
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

  const metricsGroup = createElement('div', { class: 'welcome-header-metrics-group' }, [
    presMetric,
    trackMetric,
  ]);

  const createBtn = createButton({
    text: 'Nueva Presentación',
    variant: 'secondary',
    icon: createIcon({ name: 'plus', size: 'base' }),
    onClick: () => {
      if (typeof onCreatePresentation === 'function') {
        onCreatePresentation();
      }
    },
  });
  createBtn.el.classList.add('welcome-header-create-btn');

  const footerRow = createElement('div', { class: 'welcome-header-footer' }, [
    metricsGroup,
    createBtn.el,
  ]);

  const el = createElement('section', { class: 'welcome-header welcome-header--hero', 'aria-label': 'Asistente Shirley' }, [
    introRow,
    heroChatArea,
    footerRow,
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
    if (nextProps.onSendMessage !== undefined) {
      onSendMessage = nextProps.onSendMessage;
    }
  }

  return { el, update };
}
