import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { bindDialogDismiss } from '../../behaviors/bind-dialog-dismiss.js';
import {
  generarFotoEntrada,
  compartirFotoEntrada,
  descargarBlob,
  puedeCompartirArchivos,
  crearArchivoEntrada,
  obtenerCodigoCorto,
} from './ticket-photo.js';

/**
 * Displays the generated ticket image with concise native sharing or download fallback,
 * without obsolete clipboard workaround text.
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
  let imageFile = null;
  let imageUrl = null;
  let destroyed = false;

  const isPaid = ticket?.estado_pago === 'Pagado';

  const titleEl = createElement('h3', { class: 'events-dialog-title' }, [
    'Imagen de la entrada',
  ]);

  const statusEl = createElement('p', {
    class: 'ticket-image-status',
    role: 'status',
    'aria-live': 'polite',
  }, [isPaid ? 'Preparando la imagen…' : 'Esta entrada está pendiente de pago. Debes confirmar el pago antes de generar la imagen.']);

  const imageEl = createElement('img', {
    class: 'ticket-image-preview',
    alt: `Entrada de ${ticket?.nombre_comprador || 'participante'}`,
    style: 'display: none;',
  });

  const initiallyCanShare = puedeCompartirArchivos();
  let canShare = initiallyCanShare;

  const actionBtn = createButton({
    text: initiallyCanShare ? 'Compartir' : 'Descargar',
    variant: 'primary',
    disabled: true,
    onClick: () => {
      if (!imageBlob) return;
      if (canShare && imageFile) {
        compartirFotoEntrada(ticket, event, { file: imageFile, blob: imageBlob });
      } else {
        const codigo = obtenerCodigoCorto(ticket) || 'entrada';
        descargarBlob(imageBlob, `entrada-${codigo}.png`);
      }
    },
  });
  actionBtn.el.classList.add(
    initiallyCanShare ? 'ticket-image-share-btn' : 'ticket-image-download-btn'
  );

  const closeBtn = createButton({
    text: 'Cerrar',
    variant: 'secondary',
    onClick: onClose,
  });

  const actionsEl = createElement('div', { class: 'events-dialog-actions' }, [
    closeBtn.el,
    ...(isPaid ? [actionBtn.el] : []),
  ]);

  const dialogCard = createElement('div', {
    class: 'events-dialog ticket-image-dialog',
    role: 'dialog',
    'aria-modal': 'true',
  }, [titleEl, statusEl, imageEl, actionsEl]);

  const overlayEl = createElement('div', { class: 'dialog-overlay' }, [dialogCard]);
  const unbindDismiss = bindDialogDismiss(overlayEl, onClose);

  if (isPaid) {
    generarFotoEntrada(ticket, event)
      .then((blob) => {
        if (destroyed) return;
        imageBlob = blob;
        imageFile = crearArchivoEntrada(blob, ticket);
        imageUrl = URL.createObjectURL(blob);
        imageEl.src = imageUrl;
        imageEl.style.display = 'block';
        statusEl.textContent = '';
        statusEl.style.display = 'none';

        canShare = puedeCompartirArchivos(imageFile);
        if (canShare) {
          actionBtn.el.classList.remove('ticket-image-download-btn');
          actionBtn.el.classList.add('ticket-image-share-btn');
        } else {
          actionBtn.el.classList.remove('ticket-image-share-btn');
          actionBtn.el.classList.add('ticket-image-download-btn');
        }
        actionBtn.update({
          text: canShare ? 'Compartir' : 'Descargar',
          disabled: false,
        });
      })
      .catch((err) => {
        if (destroyed) return;
        statusEl.style.display = 'block';
        statusEl.textContent = (err && err.message) || 'No se pudo preparar la imagen.';
      });
  }

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
