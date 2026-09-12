/**
 * Door Access Control View and Controller (#/puerta/:id).
 * Optimized for mobile door scanning with Camera BarcodeDetector integration,
 * always-visible manual input fallback, autofocus recovery, and unambiguous 3-state banners.
 */

import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { toEvent } from '../../router/navigate.js';
import * as defaultApi from './api.js';
import { isBarcodeDetectorSupported, createCameraQrScanner } from './camera-qr-scanner.js';
import { formatPublicDateTime } from './public-ticket-controller.js';

/**
 * Creates the presentational view for door validation.
 *
 * @param {{
 *   eventName?: string,
 *   result?: {
 *     resultado: 'Valida' | 'Ya Usada' | 'Invalida',
 *     nombre_comprador?: string,
 *     tipo_entrada?: string,
 *     hora_ingreso?: string,
 *     motivo?: string,
 *   } | null,
 *   isCameraActive?: boolean,
 *   onSubmitCode: (code: string) => Promise<void> | void,
 *   onToggleCamera?: () => void,
 *   onBack?: () => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   videoEl: HTMLVideoElement,
 *   focusInput: () => void,
 *   clearInput: () => void,
 *   update: (newState: any) => void,
 *   destroy: () => void,
 * }}
 */
export function createDoorView(props) {
  let state = {
    eventName: props.eventName || 'Evento',
    result: props.result || null,
    isCameraActive: Boolean(props.isCameraActive),
    canUseCamera: isBarcodeDetectorSupported(),
  };

  const { onSubmitCode, onToggleCamera, onBack } = props;

  const container = createElement('div', { class: 'door-container' });

  // Camera video element (always exists for scanner binding)
  const videoEl = createElement('video', {
    class: 'door-camera-video',
    autoplay: 'true',
    muted: 'true',
    playsinline: 'true',
  });

  const manualInput = createElement('input', {
    type: 'text',
    class: 'door-manual-input',
    placeholder: 'Código QR (32 hex) o escanear...',
    autocomplete: 'off',
    autocapitalize: 'none',
    spellcheck: 'false',
  });

  function focusInput() {
    setTimeout(() => {
      try {
        manualInput.focus();
      } catch {}
    }, 50);
  }

  function clearInput() {
    manualInput.value = '';
  }

  function render() {
    container.innerHTML = '';

    // Header bar
    const backBtn = createButton({
      text: '← Salir',
      variant: 'ghost',
      size: 'sm',
      onClick: onBack || (() => {}),
    });

    const headerTitle = createElement('div', { class: 'door-header-text' }, [
      createElement('strong', {}, ['Control en Puerta']),
      createElement('div', { class: 'door-header-event text-muted font-bold' }, [state.eventName]),
    ]);

    const camBtn = state.canUseCamera
      ? createButton({
          text: state.isCameraActive ? 'Cerrar Cámara' : 'Abrir Cámara',
          variant: state.isCameraActive ? 'secondary' : 'primary',
          size: 'sm',
          onClick: onToggleCamera || (() => {}),
        }).el
      : null;

    const topbar = createElement('div', { class: 'door-topbar' }, [
      backBtn.el,
      headerTitle,
      camBtn,
    ].filter(Boolean));

    // Notice if not mobile phone
    let deviceNotice = null;
    const isDesktop = typeof window !== 'undefined' && window.innerWidth > 768;
    if (isDesktop) {
      deviceNotice = createElement('div', { class: 'door-device-notice' }, [
        '📱 Recomendación: Usa un teléfono móvil para escanear con mayor agilidad en la puerta.',
      ]);
    }

    // Unambiguous 3-State Feedback Banner
    let bannerEl = null;
    const res = state.result;

    if (!res) {
      bannerEl = createElement('div', { class: 'door-banner door-banner-idle' }, [
        createElement('div', { class: 'door-banner-icon' }, ['🎟️']),
        createElement('h3', { class: 'door-banner-title' }, ['Listo para Validar']),
        createElement('p', {}, ['Escanea un código QR o ingrésalo manualmente abajo.']),
      ]);
    } else if (res.resultado === 'Valida') {
      bannerEl = createElement('div', { class: 'door-banner door-banner-valid' }, [
        createElement('div', { class: 'door-banner-icon' }, ['✓']),
        createElement('h2', { class: 'door-banner-title' }, ['ENTRADA VÁLIDA']),
        createElement('div', { class: 'door-banner-details' }, [
          createElement('p', { class: 'door-banner-buyer font-bold' }, [res.nombre_comprador || 'Asistente']),
          createElement('p', { class: 'door-banner-type' }, [`Tipo: ${res.tipo_entrada || 'General'}`]),
          createElement('p', { class: 'door-banner-time' }, [
            `Ingreso: ${formatPublicDateTime(res.hora_ingreso)}`,
          ]),
        ]),
      ]);
    } else if (res.resultado === 'Ya Usada') {
      bannerEl = createElement('div', { class: 'door-banner door-banner-used' }, [
        createElement('div', { class: 'door-banner-icon' }, ['⚠️']),
        createElement('h2', { class: 'door-banner-title' }, ['YA FUE USADA']),
        createElement('div', { class: 'door-banner-details' }, [
          createElement('p', { class: 'door-banner-sub' }, ['Esta entrada ya fue registrada anteriormente.']),
          createElement('p', { class: 'door-banner-time' }, [
            `Hora de primer ingreso: ${formatPublicDateTime(res.hora_ingreso)}`,
          ]),
        ]),
      ]);
    } else {
      // Invalida
      bannerEl = createElement('div', { class: 'door-banner door-banner-invalid' }, [
        createElement('div', { class: 'door-banner-icon' }, ['✕']),
        createElement('h2', { class: 'door-banner-title' }, ['ENTRADA INVÁLIDA']),
        createElement('div', { class: 'door-banner-details' }, [
          createElement('p', { class: 'door-banner-sub font-bold' }, [
            res.motivo ? `Motivo: ${res.motivo}` : 'Código no reconocido',
          ]),
        ]),
      ]);
    }

    // Camera Section (rendered only when active)
    let cameraSection = null;
    if (state.isCameraActive) {
      cameraSection = createElement('div', { class: 'door-camera-section' }, [
        videoEl,
        createElement('div', { class: 'door-camera-guide' }),
      ]);
    }

    // Manual Input Form (ALWAYS visible)
    const submitBtn = createButton({
      text: 'Validar Código',
      variant: 'primary',
      type: 'submit',
      class: 'door-submit-btn',
    });

    const formEl = createElement('form', { class: 'door-manual-form' }, [
      manualInput,
      submitBtn.el,
    ]);

    formEl.addEventListener('submit', async (e) => {
      e.preventDefault();
      const code = manualInput.value.trim();
      if (!code) return;
      await onSubmitCode(code);
    });

    const manualSection = createElement('div', { class: 'door-manual-section' }, [
      createElement('label', { class: 'door-manual-label font-bold' }, ['Entrada Manual']),
      formEl,
    ]);

    container.appendChild(topbar);
    if (deviceNotice) container.appendChild(deviceNotice);
    container.appendChild(bannerEl);
    if (cameraSection) container.appendChild(cameraSection);
    container.appendChild(manualSection);
  }

  render();
  focusInput();

  function update(newState) {
    state = { ...state, ...newState };
    render();
    focusInput();
  }

  function destroy() {
    container.innerHTML = '';
  }

  return {
    el: container,
    videoEl,
    focusInput,
    clearInput,
    update,
    destroy,
  };
}

/**
 * Controller orchestrating the door check-in mode (#/puerta/:id).
 *
 * @param {{
 *   eventId: string,
 *   api?: any,
 *   audioController?: { stop: () => void },
 * }} deps 
 * @returns {{
 *   headerEl: HTMLElement | null,
 *   viewEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createDoorController(deps = {}) {
  const { eventId, api = defaultApi, audioController } = deps;

  if (audioController && typeof audioController.stop === 'function') {
    audioController.stop();
  }

  let eventData = null;
  let scanner = null;
  let isCameraActive = false;

  const view = createDoorView({
    eventName: 'Cargando...',
    isCameraActive: false,
    onBack: () => toEvent(eventId),
    onToggleCamera: handleToggleCamera,
    onSubmitCode: handleSubmitCode,
  });

  async function loadEvent() {
    try {
      eventData = await api.getEvent(eventId);
      view.update({ eventName: eventData.nombre });
    } catch {
      view.update({ eventName: 'Evento' });
    }
  }

  loadEvent();

  async function handleToggleCamera() {
    if (isCameraActive) {
      stopCamera();
    } else {
      await startCamera();
    }
  }

  async function startCamera() {
    if (!scanner) {
      scanner = createCameraQrScanner({
        videoEl: view.videoEl,
        onScan: (code) => {
          handleSubmitCode(code);
        },
        onError: (err) => {
          console.warn('Camera scan error:', err);
          stopCamera();
        },
      });
    }

    try {
      isCameraActive = true;
      view.update({ isCameraActive: true });
      await scanner.start();
    } catch (err) {
      isCameraActive = false;
      view.update({ isCameraActive: false });
    }
  }

  function stopCamera() {
    if (scanner) {
      scanner.stop();
    }
    isCameraActive = false;
    view.update({ isCameraActive: false });
    view.focusInput();
  }

  async function handleSubmitCode(code) {
    if (!code) return;

    try {
      const result = await api.checkInTicket(eventId, code);
      view.update({ result });
    } catch (err) {
      view.update({
        result: {
          resultado: 'Invalida',
          motivo: (err && err.message) || 'Error de conexión',
        },
      });
    } finally {
      view.clearInput();
      view.focusInput();
    }
  }

  function destroy() {
    stopCamera();
    if (scanner) {
      scanner.destroy();
      scanner = null;
    }
    view.destroy();
  }

  return {
    headerEl: null, // Door mode renders its own self-contained mobile topbar
    viewEl: view.el,
    destroy,
  };
}
