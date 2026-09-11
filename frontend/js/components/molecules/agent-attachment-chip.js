import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { formatBytes } from '../../utils/format/bytes.js';

/**
 * Renders the audio attachment chip inside the composer.
 *
 * @param {{
 *   file: { name: string, size?: number },
 *   onRemove: () => void,
 *   disabled?: boolean,
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createAgentAttachmentChip(props) {
  let { file, onRemove, disabled = false } = props;

  const clipIcon = createIcon({ name: 'paperclip', size: 'sm', className: 'agent-attachment-chip-icon' });
  const nameEl = createElement('span', { class: 'agent-attachment-chip-name' }, [file ? file.name : '']);
  const sizeEl = createElement('span', { class: 'agent-attachment-chip-size' }, [
    file && file.size ? formatBytes(file.size) : '',
  ]);

  const closeIcon = createIcon({ name: 'close', size: 'sm' });
  const removeBtn = createElement(
    'button',
    {
      type: 'button',
      class: 'agent-attachment-chip-remove',
      'aria-label': 'Quitar archivo adjunto',
      title: 'Quitar archivo adjunto',
      disabled,
      onClick: (e) => {
        e.stopPropagation();
        if (typeof onRemove === 'function') {
          onRemove();
        }
      },
    },
    [closeIcon]
  );

  const infoEl = createElement('div', { class: 'agent-attachment-chip-info' }, [nameEl, sizeEl]);

  const el = createElement(
    'div',
    {
      class: `agent-attachment-chip ${disabled ? 'is-disabled' : ''}`,
      role: 'group',
      'aria-label': `Archivo adjunto: ${file ? file.name : ''}`,
    },
    [clipIcon, infoEl, removeBtn]
  );

  function update(nextProps = {}) {
    if (nextProps.file !== undefined) {
      file = nextProps.file;
      nameEl.textContent = file ? file.name : '';
      sizeEl.textContent = file && file.size ? formatBytes(file.size) : '';
      el.setAttribute('aria-label', `Archivo adjunto: ${file ? file.name : ''}`);
    }
    if (nextProps.onRemove !== undefined) {
      onRemove = nextProps.onRemove;
    }
    if (nextProps.disabled !== undefined) {
      disabled = nextProps.disabled;
      if (disabled) {
        removeBtn.setAttribute('disabled', '');
        el.classList.add('is-disabled');
      } else {
        removeBtn.removeAttribute('disabled');
        el.classList.remove('is-disabled');
      }
    }
  }

  return { el, update };
}
