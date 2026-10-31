/**
 * Public Ticket View and Controller for attendee mobile access (#/ticket/:scan_token).
 * Pure attendee interface consuming exclusively public ticket DTOs.
 * Securely conditions QR SVG rendering and WhatsApp photo card sharing upon verified payment.
 */

import { createElement } from "../../utils/dom/create-element.js";
import { createButton } from "../../components/atoms/button.js";
import { createIcon } from "../../components/atoms/icon.js";
import { generarFotoEntrada, descargarBlob, ensureTicketPhotoStyles } from "./ticket-photo.js";
import * as defaultApi from "./api.js";

/**
 * Formats ISO date string to a human-friendly Spanish format.
 * @param {string} isoString 
 * @returns {string}
 */
export function formatPublicDateTime(isoString) {
  if (!isoString) return "";
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return isoString;
    return d.toLocaleString("es-PE", {
      weekday: "short",
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return isoString;
  }
}

/**
 * Creates the presentational view for a public attendee ticket.
 *
 * @param {{
 *   ticket?: any,
 *   isLoading?: boolean,
 *   errorMessage?: string,
 * }} initialState 
 * @returns {{
 *   el: HTMLElement,
 *   update: (newState: any) => void,
 *   destroy: () => void,
 * }}
 */
export function createPublicTicketView(initialState = {}) {
  let state = {
    ticket: initialState.ticket || null,
    isLoading: Boolean(initialState.isLoading),
    errorMessage: initialState.errorMessage || null,
  };

  ensureTicketPhotoStyles();

  const container = createElement("div", { class: "public-ticket-container" });

  function render() {
    container.innerHTML = "";

    if (state.isLoading) {
      const loadingCard = createElement("div", { class: "public-ticket-card public-ticket-loading" }, [
        createElement("div", { class: "public-ticket-body" }, [
          createElement("div", { class: "spinner" }),
          createElement("p", { class: "text-center font-bold" }, ["Cargando entrada digital..."]),
        ]),
      ]);
      container.appendChild(loadingCard);
      return;
    }

    if (state.errorMessage) {
      const errorCard = createElement("div", { class: "public-ticket-card public-ticket-error" }, [
        createElement("div", { class: "public-ticket-body" }, [
          createElement("h3", { class: "text-danger text-center font-bold" }, ["Entrada no disponible"]),
          createElement("p", { class: "text-center" }, [state.errorMessage]),
          createElement("p", { class: "text-center text-muted" }, [
            "Verifica que el enlace sea correcto o comunícate con la organización del evento.",
          ]),
        ]),
      ]);
      container.appendChild(errorCard);
      return;
    }

    const { ticket } = state;
    if (!ticket) {
      return;
    }

    const isPaid = ticket.estado_pago === "Pagado";
    const hasEntered = ticket.estado_checkin === "Ingresó";

    // Header with Event and Brand
    const headerEl = createElement("div", { class: "public-ticket-header" }, [
      createElement("span", { class: "public-ticket-brand-tag" }, ["Shirley Show • Entrada Digital"]),
      createElement("h2", { class: "public-ticket-event-title" }, [ticket.nombre_evento || "Evento"]),
      createElement("p", { class: "public-ticket-event-meta" }, [
        `${formatPublicDateTime(ticket.fecha_hora)} • ${ticket.lugar || ""}`,
      ]),
    ]);

    // Attendee and Ticket Details
    const buyerRow = createElement("div", { class: "public-ticket-info-row" }, [
      createElement("span", { class: "public-ticket-info-label" }, ["Asistente:"]),
      createElement("strong", { class: "public-ticket-info-value" }, [ticket.nombre_comprador || ""]),
    ]);

    const typeRow = createElement("div", { class: "public-ticket-info-row" }, [
      createElement("span", { class: "public-ticket-info-label" }, ["Tipo de Entrada:"]),
      createElement("strong", { class: "public-ticket-info-value" }, [ticket.tipo_entrada || ""]),
    ]);

    const priceRow = createElement("div", { class: "public-ticket-info-row" }, [
      createElement("span", { class: "public-ticket-info-label" }, ["Precio:"]),
      createElement("span", { class: "public-ticket-info-value" }, [
        `S/ ${Number(ticket.precio || 0).toFixed(2)}`,
      ]),
    ]);

    const statusBadge = createElement(
      "span",
      { class: `events-badge ${isPaid ? "events-badge-success" : "events-badge-warning"}` },
      [ticket.estado_pago]
    );

    const checkinBadge = createElement(
      "span",
      { class: `events-badge ${hasEntered ? "events-badge-active" : "events-badge-muted"}` },
      [hasEntered ? "Ya Ingresó" : "No ingresó"]
    );

    const badgesRow = createElement("div", { class: "public-ticket-badges-row" }, [
      statusBadge,
      checkinBadge,
    ]);

    // Conditional QR Rendering vs Pending Banner
    let credentialSection = null;

    if (isPaid) {
      const qrWrapper = createElement("div", { class: "public-ticket-qr-box" });
      if (ticket.qr_svg) {
        qrWrapper.innerHTML = ticket.qr_svg;
      }
      const instruction = createElement("p", { class: "public-ticket-qr-instruction" }, [
        hasEntered
          ? "Esta entrada ya fue registrada en la puerta del evento."
          : "Presenta este código QR en la puerta del evento para tu ingreso.",
      ]);
      qrWrapper.appendChild(instruction);
      credentialSection = qrWrapper;
    } else {
      // Pending status: NEVER render QR box or SVG
      credentialSection = createElement("div", { class: "public-ticket-pending-banner" }, [
        createElement("h4", { class: "public-ticket-pending-title" }, [
          createElement("span", { class: "public-ticket-pending-dot" }),
          createElement("span", {}, ["Reserva pendiente de pago"]),
        ]),
        createElement("p", { class: "public-ticket-pending-msg" }, [
          `Esta entrada está reservada por S/ ${Number(ticket.precio || 0).toFixed(2)}. `,
          "Para habilitar tu código QR de acceso, realiza el pago coordinado (Yape o Efectivo) ",
          "con el organizador para su confirmación.",
        ]),
      ]);
    }

    // Botón único: descargar la imagen (solo si Pagado)
    let downloadBtn = null;
    let noticeEl = null;

    if (isPaid) {
      noticeEl = createElement("p", { class: "ticket-photo-notice", style: "display: none;" });

      downloadBtn = createButton({
        text: "Descargar imagen",
        variant: "primary",
        fullWidth: true,
        onClick: async () => {
          if (downloadBtn) {
            downloadBtn.update({ text: "Creando imagen…", disabled: true });
          }
          if (noticeEl) {
            noticeEl.style.display = "none";
            noticeEl.textContent = "";
          }
          try {
            const blob = await generarFotoEntrada(
              ticket,
              {
                nombre: ticket.nombre_evento,
                fecha_hora: ticket.fecha_hora,
                lugar: ticket.lugar,
              }
            );
            descargarBlob(blob, `entrada-${ticket.nombre_comprador || "ticket"}.png`);
          } catch (err) {
            console.error("Error al crear la imagen de la entrada:", err);
            if (noticeEl) {
              noticeEl.textContent = "No se pudo crear la imagen de la entrada.";
              noticeEl.style.display = "block";
            }
          } finally {
            if (downloadBtn) {
              downloadBtn.update({ text: "Descargar imagen", disabled: false });
            }
          }
        },
      });
    }

    const actionsChildren = [
      downloadBtn ? downloadBtn.el : null,
      noticeEl,
    ].filter(Boolean);

    const actionsEl = createElement("div", { class: "public-ticket-actions" }, actionsChildren);

    const bodyEl = createElement("div", { class: "public-ticket-body" }, [
      buyerRow,
      typeRow,
      priceRow,
      badgesRow,
      credentialSection,
      actionsEl,
    ]);

    const cardEl = createElement("div", { class: "public-ticket-card" }, [
      headerEl,
      bodyEl,
    ]);

    container.appendChild(cardEl);
  }

  render();

  function update(newState) {
    state = { ...state, ...newState };
    render();
  }

  function destroy() {
    container.innerHTML = "";
  }

  return { el: container, update, destroy };
}

/**
 * Minimal public topbar for attendees.
 * @returns {{ el: HTMLElement }}
 */
export function createPublicTopbar() {
  const brandTitle = createElement("span", { class: "public-topbar-title" }, ["Shirley Show"]);
  const el = createElement("header", { class: "public-topbar" }, [brandTitle]);
  return { el };
}

/**
 * Controller orchestrating the public ticket route (#/ticket/:token).
 *
 * @param {{
 *   scanToken: string,
 *   api?: any,
 *   audioController?: { stop: () => void },
 * }} deps 
 * @returns {{
 *   headerEl: HTMLElement,
 *   viewEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createPublicTicketController(deps = {}) {
  const { scanToken, api = defaultApi, audioController } = deps;

  if (audioController && typeof audioController.stop === "function") {
    audioController.stop();
  }

  const header = createPublicTopbar();
  const view = createPublicTicketView({ isLoading: true });

  async function loadTicket() {
    try {
      const ticket = await api.getPublicTicket(scanToken);
      view.update({
        ticket: { ...ticket, scan_token: scanToken },
        isLoading: false,
        errorMessage: null,
      });
    } catch (err) {
      const message = (err && err.message) || "No se pudo cargar la entrada solicitada.";
      view.update({ ticket: null, isLoading: false, errorMessage: message });
    }
  }

  loadTicket();

  function destroy() {
    view.destroy();
  }

  return {
    headerEl: header.el,
    viewEl: view.el,
    destroy,
  };
}
