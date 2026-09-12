import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';
import { generarFotoEntrada } from './ticket-photo.js';

/**
 * Displays the generated ticket image and explains how to send it without
 * assuming a recipient or opening an external application.
 *
 * @param {{
 *   ticket: any,
 *   event?: { nombre?: string, fecha_hora?: string, lugar?: string } | null,
 *   onClose: () => void,
 * }} props
 * @returns {{ el: HTMLElement, destroy: () => void }}
 */
export function createTicketImagePreviewDialog({ ticket, event = null, onClose }) {
  let imageBlob = null;
  let imageUrl = null;
  let destroyed = false;

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, [
    'Imagen de la entrada',
  ]);
  const mobileHelp = createElement('p', { class: 'ticket-image-help' }, [
    'En celular: pulsa “Copiar imagen”. Luego abre WhatsApp, mantén presionado donde escribes y elige “Pegar”.',
  ]);
  const desktopHelp = createElement('p', { class: 'ticket-image-help' }, [
    'En computadora también puedes hacer clic derecho sobre la entrada y elegir “Copiar imagen”.',
  ]);
  const statusEl = createElement('p', {
    class: 'ticket-image-status',
    role: 'status',
    'aria-live': 'polite',
  }, ['Preparando la imagen…']);
  const imageEl = createElement('img', {
    class: 'ticket-image-preview',
    alt: `Entrada de ${ticket.nombre_comprador || 'participante'}`,
    style: 'display: none;',
  });

  const copyBtn = createButton({
    text: 'Copiar imagen',
    variant: 'primary',
    disabled: true,
    onClick: async () => {
      if (!imageBlob) return;
      copyBtn.update({ disabled: true, text: 'Copiando…' });
      try {
        if (
          typeof navigator === 'undefined' ||
          !navigator.clipboard ||
          typeof navigator.clipboard.write !== 'function' ||
          typeof ClipboardItem === 'undefined'
        ) {
          throw new Error('copy_not_supported');
        }
        await navigator.clipboard.write([
          new ClipboardItem({ 'image/png': imageBlob }),
        ]);
        statusEl.textContent = '¡Listo! Abre WhatsApp, mantén presionado donde escribes y toca “Pegar”.';
      } catch (err) {
        statusEl.textContent =
          err && err.message === 'copy_not_supported'
            ? 'Mantén presionada la imagen y elige “Copiar”; después pégala en WhatsApp.'
            : 'No se pudo copiar automáticamente. Mantén presionada la imagen y elige “Copiar”.';
      } finally {
        copyBtn.update({ disabled: false, text: 'Copiar imagen' });
      }
    },
  });

  const closeBtn = createButton({
    text: 'Cerrar',
    variant: 'secondary',
    onClick: onClose,
  });
  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    closeBtn.el,
    copyBtn.el,
  ]);
  const dialogCard = createElement('div', {
    class: 'events-dialog ticket-image-dialog',
    role: 'dialog',
    'aria-modal': 'true',
  }, [titleEl, mobileHelp, desktopHelp, statusEl, imageEl, actionsEl]);
  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);
  const unbindDismiss = bindDialogDismiss(overlayEl, onClose);

  generarFotoEntrada(ticket, event)
    .then((blob) => {
      if (destroyed) return;
      imageBlob = blob;
      imageUrl = URL.createObjectURL(blob);
      imageEl.src = imageUrl;
      imageEl.style.display = 'block';
      statusEl.textContent = 'La imagen está lista para enviarla.';
      copyBtn.update({ disabled: false });
    })
    .catch((err) => {
      if (destroyed) return;
      statusEl.textContent = (err && err.message) || 'No se pudo preparar la imagen.';
    });

  function destroy() {
    destroyed = true;
    unbindDismiss();
    if (imageUrl) {
      URL.revokeObjectURL(imageUrl);
      imageUrl = null;
    }
    if (overlayEl.parentNode) {
      overlayEl.parentNode.removeChild(overlayEl);
    } else if (typeof overlayEl.remove === 'function') {
      overlayEl.remove();
    }
  }

  return { el: overlayEl, destroy };
}
