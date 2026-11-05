import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Visual confirmation card for a confirmed tool action executed by Ressy.
 *
 * @param {{
 *   action: {
 *     tool: string,
 *     summary?: string,
 *     status?: string,
 *     presentation_id?: string,
 *     details?: any,
 *   },
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createAgentToolCard(props) {
  let { action } = props;

  const isSuccess = action.status === 'success';
  const toolClass = `agent-tool-card--${(action.tool || 'generic').replace(/_/g, '-')}`;

  const checkIcon = createIcon({
    name: 'check',
    size: 'sm',
    className: 'agent-tool-card-icon',
  });

  const summaryText = action.summary || formatDefaultSummary(action);
  const summaryEl = createElement('span', { class: 'agent-tool-card-summary' }, [summaryText]);

  const rowChildren = [checkIcon, summaryEl];
  const row = createElement('div', { class: 'agent-tool-card-row' }, rowChildren);

  const bodyChildren = [row];
  const audioEl = buildAudioActions(action);
  if (audioEl) {
    bodyChildren.push(audioEl);
  }

  const el = createElement(
    'div',
    {
      class: `agent-tool-card ${toolClass} ${isSuccess ? 'is-success' : 'is-error'}`,
      role: 'status',
    },
    bodyChildren
  );

  function update(nextProps = {}) {
    if (nextProps.action) {
      action = nextProps.action;
      summaryEl.textContent = action.summary || formatDefaultSummary(action);
    }
  }

  return { el, update };
}

/**
 * Builds the download and playback controls for a track the agent just added.
 *
 * The point of the separation work is the audio file itself, so the turn that
 * produced it must let the user take it and hear it without hunting for the
 * track in the presentation. Only a successful `add_track_from_staged` carries
 * a real `item_id`; anything else gets nothing rather than a broken link.
 *
 * @param {Record<string, any>} action
 * @returns {HTMLElement | null}
 */
function buildAudioActions(action) {
  if (!action || action.status !== 'success') return null;
  if (action.tool !== 'add_track_from_staged') return null;

  const itemId = action.details && action.details.item_id;
  if (!itemId || typeof itemId !== 'string') return null;

  const encoded = encodeURIComponent(itemId);
  const streamUrl = `/api/tracks/${encoded}/audio`;

  const audioEl = createElement('audio', {
    class: 'agent-tool-card-audio',
    controls: true,
    preload: 'none',
    src: streamUrl,
  });

  const downloadLink = createElement(
    'a',
    {
      class: 'agent-tool-card-download',
      href: `${streamUrl}?download=1`,
      download: '',
      title: 'Descargar el audio',
    },
    [createIcon({ name: 'download', size: 'sm' }), 'Descargar']
  );

  return createElement('div', { class: 'agent-tool-card-audio-actions' }, [audioEl, downloadLink]);
}

function formatDefaultSummary(action) {
  switch (action.tool) {
    case 'create_presentation':
      return 'Presentación creada';
    case 'add_lyrics':
      return 'Letra añadida al show';
    case 'add_note':
      return 'Nota añadida al show';
    case 'add_track_from_staged':
      return 'Pista de audio añadida al show';
    default:
      return 'Acción completada';
  }
}
