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

  const el = createElement(
    'div',
    {
      class: `agent-tool-card ${toolClass} ${isSuccess ? 'is-success' : 'is-error'}`,
      role: 'status',
    },
    [checkIcon, summaryEl]
  );

  function update(nextProps = {}) {
    if (nextProps.action) {
      action = nextProps.action;
      summaryEl.textContent = action.summary || formatDefaultSummary(action);
    }
  }

  return { el, update };
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
