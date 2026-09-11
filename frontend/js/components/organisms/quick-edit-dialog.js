import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createTextInput } from '../atoms/text-input.js';
import { createTextArea } from '../atoms/text-area.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';
import { bindFormSubmit } from '../../behaviors/bind-form-submit.js';
import { trimRequired } from '../../utils/validation/trim-required.js';

/**
 * Organism dialog for quick-editing show item title or text body.
 * Preloads content, supports keyboard shortcuts (Enter for track, Cmd/Ctrl+Enter for text, Escape to cancel),
 * and preserves draft on network or validation errors.
 * 
 * @param {{
 *   item: { id: string, type: 'track' | 'lyrics' | 'note', title?: string, body?: string },
 *   onSave: (payload: { title?: string, body?: string }) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createQuickEditDialog(props) {
  const { item, onSave, onCancel } = props;

  const isTrack = item.type === 'track';
  const isLyrics = item.type === 'lyrics';

  const titles = {
    track: 'Editar Pista',
    lyrics: 'Editar Letra',
    note: 'Editar Nota de Escenario',
  };

  const titleEl = createElement('h3', { class: 'quick-edit-title' }, [
    titles[item.type] || 'Editar Elemento',
  ]);

  const errorEl = createElement(
    'div',
    {
      class: 'quick-edit-error',
      role: 'alert',
      style: { display: 'none' },
    },
    ['']
  );

  let field;
  let shortcutText;

  if (isTrack) {
    field = createTextInput({
      label: 'Nombre de la pista',
      name: 'track-title',
      value: item.title || '',
      placeholder: 'ej. Canción 1 - Intro',
      required: true,
    });
    shortcutText = 'Enter para guardar • Escape para cancelar';
  } else if (isLyrics) {
    field = createTextArea({
      label: 'Letra de la canción',
      name: 'lyrics-body',
      value: item.body || '',
      placeholder: 'Escribe o edita la letra...',
      rows: 8,
      required: true,
    });
    shortcutText = 'Cmd/Ctrl + Enter para guardar • Escape para cancelar';
  } else {
    field = createTextArea({
      label: 'Nota de escenario',
      name: 'note-body',
      value: item.body || '',
      placeholder: 'Escribe o edita la indicación...',
      rows: 4,
      required: true,
    });
    shortcutText = 'Cmd/Ctrl + Enter para guardar • Escape para cancelar';
  }

  const shortcutHintEl = createElement('span', { class: 'quick-edit-shortcut-hint' }, [shortcutText]);

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: 'Guardar Cambios',
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'quick-edit-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement('form', { class: 'quick-edit-form' }, [
    errorEl,
    field.el,
    shortcutHintEl,
    actionsEl,
  ]);

  const dialogCard = createElement(
    'div',
    {
      class: 'quick-edit-dialog',
      role: 'dialog',
      'aria-modal': 'true',
    },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  // Autofocus field on render
  setTimeout(() => {
    field.focus();
  }, 50);

  let isSubmitting = false;

  async function handleSave() {
    if (isSubmitting) return;

    errorEl.style.display = 'none';
    errorEl.textContent = '';
    field.setError(null);

    let payload = {};
    if (isTrack) {
      const val = trimRequired(field.getValue(), 'El título de la pista');
      if (!val.valid) {
        field.setError(val.error);
        field.focus();
        return;
      }
      payload = { title: val.value };
    } else {
      const label = isLyrics ? 'La letra' : 'La nota de escenario';
      const val = trimRequired(field.getValue(), label);
      if (!val.valid) {
        field.setError(val.error);
        field.focus();
        return;
      }
      payload = { body: val.value };
    }

    isSubmitting = true;
    submitBtn.update({ disabled: true, text: 'Guardando...' });
    cancelBtn.update({ disabled: true });

    try {
      await onSave(payload);
    } catch (err) {
      isSubmitting = false;
      submitBtn.update({ disabled: false, text: 'Guardar Cambios' });
      cancelBtn.update({ disabled: false });

      // Preserve uncommitted draft in field and display actionable error
      const message = err && err.message ? err.message : 'Error al guardar los cambios.';
      errorEl.textContent = message;
      errorEl.style.display = 'block';
      field.focus();
    }
  }

  // Bind textarea shortcut (Cmd/Ctrl + Enter)
  if (!isTrack) {
    const rawTextarea = field.el.querySelector('textarea');
    if (rawTextarea) {
      rawTextarea.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
          e.preventDefault();
          handleSave();
        }
      });
    }
  }

  const unbindDismiss = bindDialogDismiss(overlayEl, () => {
    if (!isSubmitting) {
      onCancel();
    }
  });

  const unbindSubmit = bindFormSubmit(formEl, handleSave);

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    overlayEl.remove();
  }

  return { el: overlayEl, destroy };
}
