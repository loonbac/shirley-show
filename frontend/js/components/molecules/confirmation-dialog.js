import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';

/**
 * Creates an accessible modal confirmation dialog molecule.
 * @param {{
 *   title: string,
 *   message: string,
 *   confirmText?: string,
 *   cancelText?: string,
 *   isDanger?: boolean,
 *   onConfirm: () => Promise<void> | void,
 *   onCancel: () => void,
 * }} props 
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createConfirmationDialog(props) {
  const {
    title,
    message,
    confirmText = 'Confirmar',
    cancelText = 'Cancelar',
    isDanger = false,
    onConfirm,
    onCancel,
  } = props;

  let isBusy = false;

  const titleEl = createElement('h3', { class: 'confirmation-dialog-title' }, [title]);
  const messageEl = createElement('p', { class: 'confirmation-dialog-message' }, [message]);

  const cancelBtn = createButton({
    text: cancelText,
    variant: 'secondary',
    onClick: () => {
      if (!isBusy) onCancel();
    },
  });

  const confirmBtn = createButton({
    text: confirmText,
    variant: isDanger ? 'danger' : 'primary',
    onClick: async () => {
      if (isBusy) return;
      isBusy = true;
      confirmBtn.update({ disabled: true, text: 'Procesando...' });
      cancelBtn.update({ disabled: true });
      try {
        await onConfirm();
      } catch (err) {
        console.error('Confirmation action failed:', err);
        isBusy = false;
        confirmBtn.update({ disabled: false, text: confirmText });
        cancelBtn.update({ disabled: false });
      }
    },
  });

  const actionsEl = createElement('div', { class: 'confirmation-dialog-actions' }, [
    cancelBtn.el,
    confirmBtn.el,
  ]);

  const dialogCard = createElement('div', {
    class: 'confirmation-dialog',
    role: 'dialog',
    'aria-modal': 'true',
    'aria-labelledby': 'dialog-title',
  }, [titleEl, messageEl, actionsEl]);

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);

  // Bind Escape & backdrop dismiss
  const unbindDismiss = bindDialogDismiss(overlayEl, () => {
    if (!isBusy) onCancel();
  });

  function destroy() {
    unbindDismiss();
    overlayEl.remove();
  }

  // Focus cancel button by default for safety on danger actions
  setTimeout(() => {
    if (cancelBtn.el) cancelBtn.el.focus();
  }, 50);

  return { el: overlayEl, destroy };
}
