import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createTextInput } from '../atoms/text-input.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';
import { bindFormSubmit } from '../../behaviors/bind-form-submit.js';
import { trimRequired } from '../../utils/validation/trim-required.js';

/**
 * Creates a modal dialog organism for creating or renaming a presentation.
 * @param {{
 *   mode: 'create' | 'rename',
 *   initialTitle?: string,
 *   onSubmit: (title: string) => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createPresentationFormDialog(props) {
  const {
    mode = 'create',
    initialTitle = '',
    onSubmit,
    onCancel,
  } = props;

  const isCreate = mode === 'create';
  const titleText = isCreate ? 'Nueva Presentación' : 'Renombrar Presentación';
  const confirmText = isCreate ? 'Crear' : 'Guardar';

  const titleEl = createElement('h3', { class: 'presentation-form-dialog-title' }, [titleText]);

  const textInput = createTextInput({
    label: 'Nombre de la presentación',
    name: 'title',
    value: initialTitle,
    placeholder: 'ej. Concierto Acústico 2025',
    required: true,
  });

  const cancelBtn = createButton({
    text: 'Cancelar',
    variant: 'secondary',
    type: 'button',
    onClick: onCancel,
  });

  const submitBtn = createButton({
    text: confirmText,
    variant: 'primary',
    type: 'submit',
  });

  const actionsEl = createElement('div', { class: 'presentation-form-actions' }, [
    cancelBtn.el,
    submitBtn.el,
  ]);

  const formEl = createElement(
    'form',
    { class: 'presentation-form' },
    [textInput.el, actionsEl]
  );

  const dialogCard = createElement(
    'div',
    {
      class: 'presentation-form-dialog',
      role: 'dialog',
      'aria-modal': 'true',
    },
    [titleEl, formEl]
  );

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  const unbindDismiss = bindDialogDismiss(overlayEl, onCancel);

  const unbindSubmit = bindFormSubmit(formEl, async () => {
    const raw = textInput.getValue();
    const validation = trimRequired(raw, 'El nombre de la presentación');
    if (!validation.valid) {
      textInput.setError(validation.error);
      textInput.focus();
      return;
    }
    textInput.setError(null);

    await onSubmit(validation.value);
  });

  function destroy() {
    unbindDismiss();
    unbindSubmit();
    overlayEl.remove();
  }

  setTimeout(() => {
    textInput.focus();
  }, 50);

  return { el: overlayEl, destroy };
}
