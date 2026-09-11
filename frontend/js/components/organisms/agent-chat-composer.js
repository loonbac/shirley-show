import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';
import { createAgentAttachmentChip } from '../molecules/agent-attachment-chip.js';
import { validateAudioAttachment } from '../../data/agent-chat-api.js';

/**
 * Organism managing the input composer: auto-growing textarea, audio file picker,
 * attachment chip, size guard warning, and submit dispatch.
 *
 * @param {{
 *   draft?: string,
 *   attachment?: any,
 *   pending?: boolean,
 *   onSend: (message: string) => void,
 *   onChangeDraft?: (text: string) => void,
 *   onAttach?: (file: File) => void,
 *   onAttachClick?: () => void,
 *   onRemoveAttachment?: () => void,
 * }} props
 * @returns {{
 *   el: HTMLElement,
 *   textareaEl: HTMLTextAreaElement,
 *   attachBtnEl: HTMLButtonElement,
 *   sendBtnEl: HTMLButtonElement,
 *   fileInputEl: HTMLInputElement,
 *   update: (props: Partial<typeof props>) => void,
 * }}
 */
export function createAgentChatComposer(props) {
  let {
    draft = '',
    attachment = null,
    pending = false,
    onSend,
    onChangeDraft,
    onAttach,
    onAttachClick,
    onRemoveAttachment,
  } = props;

  let inlineWarning = null;

  // Hidden audio file input
  const fileInputEl = createElement('input', {
    type: 'file',
    accept: '.mp3,.wav,.m4a,audio/mpeg,audio/wav,audio/x-m4a,audio/mp4',
    class: 'agent-chat-file-input',
    style: { display: 'none' },
    onChange: (e) => {
      const file = e.target.files && e.target.files[0];
      if (!file) return;

      const validation = validateAudioAttachment(file);
      if (!validation.valid) {
        setInlineWarning(validation.error.message);
        fileInputEl.value = '';
        return;
      }

      setInlineWarning(null);
      if (typeof onAttach === 'function') {
        onAttach(file);
      }
      fileInputEl.value = '';
    },
  });

  // Attachment chip container
  const chipWrapper = createElement('div', { class: 'agent-composer-chip-wrapper' });
  let chipComponent = null;

  function renderChip() {
    chipWrapper.innerHTML = '';
    if (attachment) {
      chipComponent = createAgentAttachmentChip({
        file: attachment,
        disabled: pending,
        onRemove: () => {
          setInlineWarning(null);
          if (typeof onRemoveAttachment === 'function') {
            onRemoveAttachment();
          }
        },
      });
      chipWrapper.appendChild(chipComponent.el);
      chipWrapper.style.display = 'block';
    } else {
      chipComponent = null;
      chipWrapper.style.display = 'none';
    }
  }

  // Inline warning banner
  const warningEl = createElement('div', {
    class: 'agent-composer-warning',
    role: 'alert',
    style: { display: 'none' },
  });

  function setInlineWarning(msg) {
    inlineWarning = msg;
    if (msg) {
      warningEl.textContent = msg;
      warningEl.style.display = 'block';
    } else {
      warningEl.textContent = '';
      warningEl.style.display = 'none';
    }
  }

  // Textarea input
  const textareaEl = createElement('textarea', {
    class: 'agent-composer-textarea',
    placeholder: 'Escribe una instrucción para Ressy...',
    rows: '2',
    disabled: pending,
    'aria-label': 'Mensaje para el asistente Ressy',
    onInput: (e) => {
      draft = e.target.value;
      if (typeof onChangeDraft === 'function') {
        onChangeDraft(draft);
      }
      autoGrowTextarea();
    },
    onKeydown: (e) => {
      if (e.key === 'Enter') {
        if (e.shiftKey) {
          // Multiline newline, do not submit
          return;
        }
        e.preventDefault();
        submitIfAllowed();
      }
    },
  });
  textareaEl.value = draft;

  function autoGrowTextarea() {
    if (typeof textareaEl.scrollHeight === 'number') {
      textareaEl.style.height = 'auto';
      const newHeight = Math.min(Math.max(textareaEl.scrollHeight, 48), 160);
      textareaEl.style.height = `${newHeight}px`;
    }
  }

  // Dedicated attachment button >= 48x48px
  const attachIcon = createIcon({ name: 'paperclip', size: 'base' });
  const attachBtnEl = createElement(
    'button',
    {
      type: 'button',
      class: 'agent-composer-btn agent-composer-btn--attach',
      'aria-label': 'Adjuntar archivo de audio (.mp3, .wav, .m4a)',
      title: 'Adjuntar archivo de audio',
      disabled: pending,
      onClick: () => {
        if (pending) return;
        if (typeof onAttachClick === 'function') {
          onAttachClick();
        }
        if (typeof fileInputEl.click === 'function') {
          fileInputEl.click();
        }
      },
    },
    [attachIcon]
  );

  // Send button >= 48x48px
  const sendIcon = createIcon({ name: 'send', size: 'base' });
  const sendBtnEl = createElement(
    'button',
    {
      type: 'button',
      class: 'agent-composer-btn agent-composer-btn--send',
      'aria-label': 'Enviar mensaje a Ressy',
      title: 'Enviar mensaje',
      disabled: pending,
      onClick: () => {
        submitIfAllowed();
      },
    },
    [sendIcon]
  );

  function submitIfAllowed() {
    if (pending) return;
    const trimmed = (textareaEl.value || '').trim();
    if (!trimmed && !attachment) {
      return; // Cannot send empty turn without audio
    }
    if (typeof onSend === 'function') {
      onSend(textareaEl.value);
    }
  }

  const actionsLeft = createElement('div', { class: 'agent-composer-actions-left' }, [attachBtnEl]);
  const actionsRight = createElement('div', { class: 'agent-composer-actions-right' }, [sendBtnEl]);

  const toolbarEl = createElement('div', { class: 'agent-composer-toolbar' }, [
    actionsLeft,
    actionsRight,
  ]);

  const boxEl = createElement('div', { class: 'agent-composer-box' }, [
    textareaEl,
    toolbarEl,
  ]);

  const el = createElement(
    'div',
    { class: 'agent-chat-composer' },
    [chipWrapper, warningEl, boxEl, fileInputEl]
  );

  // Initial chip render
  renderChip();

  function update(nextProps = {}) {
    if (nextProps.draft !== undefined && nextProps.draft !== draft) {
      draft = nextProps.draft;
      textareaEl.value = draft;
      autoGrowTextarea();
    }
    if (nextProps.attachment !== undefined) {
      attachment = nextProps.attachment;
      renderChip();
    }
    if (nextProps.pending !== undefined) {
      pending = nextProps.pending;
      if (pending) {
        textareaEl.setAttribute('disabled', '');
        attachBtnEl.setAttribute('disabled', '');
        sendBtnEl.setAttribute('disabled', '');
      } else {
        textareaEl.removeAttribute('disabled');
        attachBtnEl.removeAttribute('disabled');
        sendBtnEl.removeAttribute('disabled');
      }
      if (chipComponent) {
        chipComponent.update({ disabled: pending });
      }
    }
    if (nextProps.onSend !== undefined) onSend = nextProps.onSend;
    if (nextProps.onChangeDraft !== undefined) onChangeDraft = nextProps.onChangeDraft;
    if (nextProps.onAttach !== undefined) onAttach = nextProps.onAttach;
    if (nextProps.onRemoveAttachment !== undefined) onRemoveAttachment = nextProps.onRemoveAttachment;
  }

  return {
    el,
    textareaEl,
    attachBtnEl,
    sendBtnEl,
    fileInputEl,
    update,
  };
}
