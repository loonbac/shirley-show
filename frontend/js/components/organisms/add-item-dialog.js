import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createTextInput } from '../atoms/text-input.js';
import { createTextArea } from '../atoms/text-area.js';
import { createItemTypeSelector } from '../molecules/item-type-selector.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';
import { bindFormSubmit } from '../../behaviors/bind-form-submit.js';
import { trimRequired } from '../../utils/validation/trim-required.js';

/**
 * Creates a modal dialog organism for adding Track, Lyrics, or Note items.
 * @param {{
 *   onSubmit: (payload: { type: 'track' | 'lyrics' | 'note', title?: string, file?: File, body?: string }) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createAddItemDialog(props) {
  const { onSubmit, onCancel } = props;

  let selectedType = 'track';

  const titleEl = createElement('h3', { class: 'add-item-dialog-title' }, ['Agregar Elemento al Show']);

  const typeSelector = createItemTypeSelector({
    selected: 'track',
    onChange: (newType) => {
      selectedType = newType;
      renderTypeFields();
    },
  });

  // Dynamic fields container
  const fieldsContainer = createElement('div', { class: 'add-item-fields' });

  // Inputs
  const trackTitleInput = createTextInput({
    label: 'Nombre de la pista',
    name: 'track-title',
    placeholder: 'ej. Canción 1 - Intro',
    required: true,
  });

  const fileInputEl = createElement('input', {
    type: 'file',
    name: 'audio',
    accept: '.mp3,.wav,.m4a,audio/mpeg,audio/wav,audio/x-m4a,audio/mp4',
    class: 'file-input',
    required: true,
  });

  const fileInputHint = createElement('span', { class: 'file-input-hint' }, [
    'Formatos admitidos: MP3, WAV, M4A. Tamaño máximo: 100 MiB.',
  ]);

  const fileInputError = createElement('span', {
    class: 'text-input-hint',
    style: { display: 'none' },
  });

  const fileWrapper = createElement('div', { class: 'file-input-wrapper' }, [
    createElement('label', { class: 'text-input-label' }, ['Archivo de audio']),
    fileInputEl,
    fileInputHint,
    fileInputError,
  ]);

  const lyricsArea = createTextArea({
    label: 'Letra de la canción',
    name: 'lyrics-body',
    placeholder: 'Escribe o pega aquí la letra para el atril...',
    rows: 6,
    required: true,
  });

  const noteArea = createTextArea({
    label: 'Nota de escenario',
    name: 'note-body',
    placeholder: 'ej. Presentar a la banda / Bajar luces / Cambio de guitarra...',
    rows: 3,
    required: true,
  });

  function renderTypeFields() {
    fieldsContainer.innerHTML = '';
    if (selectedType === 'track') {
      fieldsContainer.appendChild(trackTitleInput.el);
      fieldsContainer.appendChild(fileWrapper);
      setTimeout(() => trackTitleInput.focus(), 50);
    } else if (selectedType === 'lyrics') {
      fieldsContainer.appendChild(lyricsArea.el);
      setTimeout(() => lyricsArea.focus(), 50);
    } else if (selectedType === 'note') {
      fieldsContainer.appendChild(noteArea.el);
      setTimeout(() => noteArea.focus(), 50);
    }
  }

  // Initial render
  renderTypeFields();

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: 'Agregar al Show',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'add-item-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'add-item-form' }, [
    typeSelector.el,
    fieldsContainer,
    actionsEl,
  ]);

  const dialogCard = createElement(
    'div',
    {
      class: 'add-item-dialog',
      role: 'dialog',
      'aria-modal': 'true',
    },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);

  const unbindSubmit = bindFormSubmit(formEl, async () => {
    if (selectedType === 'track') {
      const titleVal = trimRequired(trackTitleInput.getValue(), 'El título de la pista');
      if (!titleVal.valid) {
        trackTitleInput.setError(titleVal.error);
        trackTitleInput.focus();
        return;
      }
      trackTitleInput.setError(null);

      const files = fileInputEl.files;
      if (!files || files.length === 0) {
        fileInputError.textContent = 'Selecciona un archivo de audio (.mp3, .wav o .m4a).';
        fileInputError.style.display = 'block';
        return;
      }

      const file = files[0];
      const maxBytes = 100 * 1024 * 1024;
      if (file.size > maxBytes) {
        fileInputError.textContent = 'El archivo supera el límite de 100 MiB.';
        fileInputError.style.display = 'block';
        return;
      }
      fileInputError.style.display = 'none';

      await onSubmit({
        type: 'track',
        title: titleVal.value,
        file,
      });
    } else if (selectedType === 'lyrics') {
      const bodyVal = trimRequired(lyricsArea.getValue(), 'La letra');
      if (!bodyVal.valid) {
        lyricsArea.setError(bodyVal.error);
        lyricsArea.focus();
        return;
      }
      lyricsArea.setError(null);

      await onSubmit({
        type: 'lyrics',
        body: bodyVal.value,
      });
    } else if (selectedType === 'note') {
      const noteVal = trimRequired(noteArea.getValue(), 'La nota de escenario');
      if (!noteVal.valid) {
        noteArea.setError(noteVal.error);
        noteArea.focus();
        return;
      }
      noteArea.setError(null);

      await onSubmit({
        type: 'note',
        body: noteVal.value,
      });
    }
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    overlayEl.remove();
  }

  return { el: overlayEl, destroy };
}
