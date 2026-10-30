/**
 * Door Access Control View and Controller (#/puerta/:id).
 * Optimized for mobile door scanning with Camera BarcodeDetector & jsQR fallback,
 * always-visible manual input, autofocus recovery, unambiguous 3-state banners,
 * and quick door sales tab (DOOR-1 & DOOR-2).
 */

import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { toEvent } from '../../router/navigate.js';
import * as defaultApi from './api.js';
import { isBarcodeDetectorSupported, isCameraSupported, createCameraQrScanner } from './camera-qr-scanner.js';
import { formatPublicDateTime } from './public-ticket-controller.js';

/**
 * Resolves the ticket type to use for door sales.
 * Searches case-insensitively for "General"; falls back to the first available type.
 * Returns null if no ticket types exist.
 *
 * @param {Array<{ nombre: string, precio: number, capacidad?: number }>} tipos
 * @returns {{ nombre: string, precio: number, capacidad?: number } | null}
 */
export function resolveDoorTicketType(tipos) {
  if (!Array.isArray(tipos) || tipos.length === 0) {
    return null;
  }
  const general = tipos.find(
    (t) => t && t.nombre && String(t.nombre).trim().toLowerCase() === 'general'
  );
  return general || tipos[0];
}

/**
 * Returns a human-friendly error message for camera initialization failures.
 *
 * @param {Error|any} err
 * @returns {string}
 */
export function formatCameraErrorMessage(err) {
  if (!err) return 'No se pudo acceder a la cámara.';
  const name = err.name || '';
  if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
    return 'Permiso de cámara denegado. Permite el acceso a la cámara para escanear.';
  }
  if (name === 'NotFoundError' || name === 'DevicesNotFoundError') {
    return 'No se detectó ninguna cámara en este dispositivo.';
  }
  if (name === 'NotReadableError' || name === 'TrackStartError') {
    return 'La cámara está ocupada por otra aplicación.';
  }
  return err.message || 'No se pudo iniciar la cámara.';
}

/**
 * Creates the presentational view for door validation and quick sales.
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
 *   canUseCamera?: boolean,
 *   activeTab?: 'scan' | 'sell',
 *   tiposDeEntrada?: Array<{ nombre: string, precio: number, capacidad?: number }>,
 *   sellQuantity?: number,
 *   sellPaymentMethod?: 'Efectivo' | 'Yape',
 *   sessionSold?: number,
 *   sellFeedback?: {
 *     type: 'success' | 'error',
 *     quantity?: number,
 *     total?: string,
 *     medioPago?: string,
 *     message?: string,
 *   } | null,
 *   isSubmittingSell?: boolean,
 *   onSubmitCode: (code: string) => Promise<void> | void,  // called with the scanned 32-hex secret
 *   onToggleCamera?: () => void,
 *   onBack?: () => void,
 *   onTabChange?: (tab: 'scan' | 'sell') => void,
 *   onSell?: (data: { tipoEntrada: string, cantidad: number, medioPago: 'Efectivo' | 'Yape' }) => Promise<void> | void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   videoEl: HTMLVideoElement,
 *   update: (newState: any) => void,
 *   destroy: () => void,
 * }}
 */
export function createDoorView(props) {
  let state = {
    eventName: props.eventName || 'Evento',
    result: props.result || null,
    isCameraActive: Boolean(props.isCameraActive),
    canUseCamera: props.canUseCamera !== undefined ? Boolean(props.canUseCamera) : isCameraSupported(),
    activeTab: props.activeTab || 'scan',
    tiposDeEntrada: props.tiposDeEntrada || [],
    sellQuantity: typeof props.sellQuantity === 'number' && props.sellQuantity >= 1 ? props.sellQuantity : 1,
    sellPaymentMethod: props.sellPaymentMethod || 'Efectivo',
    sessionSold: typeof props.sessionSold === 'number' ? props.sessionSold : 0,
    sellFeedback: props.sellFeedback || null,
    isSubmittingSell: Boolean(props.isSubmittingSell),
  };

  const {
    onSubmitCode,
    onToggleCamera,
    onBack,
    onTabChange,
    onSell,
  } = props;

  const container = createElement('div', { class: 'door-container' });

  // Camera video element (always exists for scanner binding)
  const videoEl = createElement('video', {
    class: 'door-camera-video',
    autoplay: 'true',
    muted: 'true',
    playsinline: 'true',
  });

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

    const camBtnAtom = (state.activeTab === 'scan' && state.canUseCamera)
      ? createButton({
          text: state.isCameraActive ? 'Cerrar Cámara' : 'Abrir Cámara',
          variant: state.isCameraActive ? 'secondary' : 'primary',
          size: 'sm',
          onClick: onToggleCamera || (() => {}),
        })
      : null;
    if (camBtnAtom) {
      camBtnAtom.el.classList.add('door-camera-toggle-btn');
    }
    const camBtn = camBtnAtom ? camBtnAtom.el : null;

    const topbar = createElement('div', { class: 'door-topbar' }, [
      backBtn.el,
      headerTitle,
      camBtn,
    ].filter(Boolean));

    // Tabs Nav (Escanear / Vender)
    const scanTabBtn = createElement('button', {
      type: 'button',
      role: 'tab',
      class: `door-tab-btn ${state.activeTab === 'scan' ? 'active' : ''}`,
      'aria-selected': state.activeTab === 'scan' ? 'true' : 'false',
      onClick: () => {
        if (state.activeTab !== 'scan') {
          state.activeTab = 'scan';
          if (typeof onTabChange === 'function') onTabChange('scan');
          render();
        }
      },
    }, ['Escanear']);

    const sellTabBtn = createElement('button', {
      type: 'button',
      role: 'tab',
      class: `door-tab-btn ${state.activeTab === 'sell' ? 'active' : ''}`,
      'aria-selected': state.activeTab === 'sell' ? 'true' : 'false',
      onClick: () => {
        if (state.activeTab !== 'sell') {
          state.activeTab = 'sell';
          if (typeof onTabChange === 'function') onTabChange('sell');
          render();
        }
      },
    }, ['Vender']);

    const tabsNav = createElement('div', { class: 'door-tabs-nav' }, [
      scanTabBtn,
      sellTabBtn,
    ]);

    // Build Scan Pane
    const scanPane = createElement('div', {
      class: `door-tab-pane door-scan-pane ${state.activeTab !== 'scan' ? 'is-hidden' : ''}`,
      hidden: state.activeTab !== 'scan' ? 'true' : undefined,
    });
    scanPane.style.display = state.activeTab === 'scan' ? 'flex' : 'none';

    // Notice if not mobile phone
    const isDesktop = typeof window !== 'undefined' && window.innerWidth > 768;
    if (isDesktop) {
      const deviceNotice = createElement('div', { class: 'door-device-notice' }, [
        '📱 Recomendación: Usa un teléfono móvil para escanear con mayor agilidad en la puerta.',
      ]);
      scanPane.appendChild(deviceNotice);
    }

    // Unambiguous 3-State Feedback Banner
    let bannerEl = null;
    const res = state.result;

    if (!res) {
      bannerEl = createElement('div', { class: 'door-banner door-banner-idle' }, [
        createElement('div', { class: 'door-banner-icon' }, ['🎟️']),
        createElement('h3', { class: 'door-banner-title' }, ['Listo para Validar']),
        createElement('p', {}, ['Apunta la cámara al código QR de la entrada.']),
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
    scanPane.appendChild(bannerEl);

    // Camera Section (rendered only when active)
    if (state.isCameraActive) {
      const cameraSection = createElement('div', { class: 'door-camera-section' }, [
        videoEl,
        createElement('div', { class: 'door-camera-guide' }),
      ]);
      scanPane.appendChild(cameraSection);
    }

    // Check-in happens only by scanning the QR. There is no manual code entry:
    // typing a short prefix would turn the 128-bit secret into a guessable code.
    scanPane.appendChild(
      createElement('p', { class: 'door-scan-hint' }, [
        'Apunta la cámara al código QR de la entrada.',
      ])
    );

    // Build Sell Pane
    const sellPane = createElement('div', {
      class: `door-tab-pane door-sell-pane ${state.activeTab !== 'sell' ? 'is-hidden' : ''}`,
      hidden: state.activeTab !== 'sell' ? 'true' : undefined,
    });
    sellPane.style.display = state.activeTab === 'sell' ? 'flex' : 'none';

    const selectedType = resolveDoorTicketType(state.tiposDeEntrada);
    const sellSection = createElement('div', { class: 'door-sell-section' });

    if (!selectedType) {
      const noTypesBanner = createElement('div', { class: 'door-banner door-banner-invalid door-sell-error' }, [
        createElement('div', { class: 'door-banner-icon' }, ['✕']),
        createElement('h2', { class: 'door-banner-title' }, ['VENTA NO DISPONIBLE']),
        createElement('div', { class: 'door-banner-details' }, [
          createElement('p', { class: 'door-banner-sub font-bold' }, [
            'El evento no tiene tipos de entrada configurados.',
          ]),
        ]),
      ]);
      sellSection.appendChild(noTypesBanner);

      const disabledSellBtn = createButton({
        text: 'Registrar venta',
        variant: 'primary',
        disabled: true,
      });
      disabledSellBtn.el.classList.add('door-register-sell-btn');
      disabledSellBtn.el.setAttribute('disabled', 'true');
      sellSection.appendChild(disabledSellBtn.el);
    } else {
      const unitPrice = Number(selectedType.precio) || 0;
      const typeCard = createElement('div', { class: 'door-sell-type-card' }, [
        createElement('span', { class: 'door-sell-type-name' }, [selectedType.nombre]),
        createElement('span', { class: 'door-sell-type-price' }, [`S/ ${unitPrice.toFixed(2)} c/u`]),
      ]);
      sellSection.appendChild(typeCard);

      if (state.sellFeedback) {
        const feed = state.sellFeedback;
        if (feed.type === 'success') {
          const successBanner = createElement('div', { class: 'door-banner door-banner-valid door-sell-feedback' }, [
            createElement('div', { class: 'door-banner-icon' }, ['✓']),
            createElement('h2', { class: 'door-banner-title' }, ['VENTA REGISTRADA']),
            createElement('div', { class: 'door-banner-details' }, [
              createElement('p', { class: 'door-banner-buyer font-bold' }, [
                `${feed.quantity} entrada(s) por S/ ${feed.total}`,
              ]),
              createElement('p', { class: 'door-banner-type' }, [
                `Medio de pago: ${feed.medioPago}`,
              ]),
            ]),
          ]);
          sellSection.appendChild(successBanner);
        } else {
          const errorBanner = createElement('div', { class: 'door-banner door-banner-invalid door-sell-feedback' }, [
            createElement('div', { class: 'door-banner-icon' }, ['✕']),
            createElement('h2', { class: 'door-banner-title' }, ['ERROR EN VENTA']),
            createElement('div', { class: 'door-banner-details' }, [
              createElement('p', { class: 'door-banner-sub font-bold' }, [
                feed.message || 'Error al procesar la venta.',
              ]),
            ]),
          ]);
          sellSection.appendChild(errorBanner);
        }
      }

      // Stepper for quantity
      const decBtn = createElement('button', {
        type: 'button',
        class: 'door-stepper-btn door-stepper-dec',
        'aria-label': 'Disminuir cantidad',
      }, ['−']);
      if (state.sellQuantity <= 1) {
        decBtn.setAttribute('disabled', 'true');
        decBtn.disabled = true;
      }

      const qtyInput = createElement('input', {
        type: 'number',
        class: 'door-stepper-input',
        min: '1',
        step: '1',
        value: String(state.sellQuantity),
      });
      qtyInput.value = String(state.sellQuantity);

      const incBtn = createElement('button', {
        type: 'button',
        class: 'door-stepper-btn door-stepper-inc',
        'aria-label': 'Aumentar cantidad',
      }, ['+']);

      const totalValueEl = createElement('span', { class: 'door-sell-total-value' }, [
        `S/ ${(unitPrice * state.sellQuantity).toFixed(2)}`,
      ]);

      function updateSellCalculations() {
        const total = (unitPrice * state.sellQuantity).toFixed(2);
        totalValueEl.textContent = `S/ ${total}`;
        qtyInput.value = String(state.sellQuantity);
        if (state.sellQuantity <= 1) {
          decBtn.setAttribute('disabled', 'true');
          decBtn.disabled = true;
        } else {
          decBtn.removeAttribute('disabled');
          decBtn.disabled = false;
        }
      }

      decBtn.addEventListener('click', () => {
        if (state.sellQuantity > 1) {
          state.sellQuantity -= 1;
          updateSellCalculations();
        }
      });

      incBtn.addEventListener('click', () => {
        state.sellQuantity += 1;
        updateSellCalculations();
      });

      qtyInput.addEventListener('input', () => {
        const val = parseInt(qtyInput.value, 10);
        if (!isNaN(val) && val >= 1) {
          state.sellQuantity = val;
          updateSellCalculations();
        }
      });

      qtyInput.addEventListener('blur', () => {
        const val = parseInt(qtyInput.value, 10);
        if (isNaN(val) || val < 1) {
          state.sellQuantity = 1;
          updateSellCalculations();
        }
      });

      const stepperContainer = createElement('div', { class: 'door-sell-field' }, [
        createElement('label', { class: 'door-field-label font-bold' }, ['Cantidad']),
        createElement('div', { class: 'door-stepper' }, [decBtn, qtyInput, incBtn]),
      ]);
      sellSection.appendChild(stepperContainer);

      // Total row
      const totalRow = createElement('div', { class: 'door-sell-total-row' }, [
        createElement('span', { class: 'door-sell-total-label font-bold' }, ['Total:']),
        totalValueEl,
      ]);
      sellSection.appendChild(totalRow);

      // Payment segmented control
      const cashBtn = createElement('button', {
        type: 'button',
        class: `door-segment-btn ${state.sellPaymentMethod === 'Efectivo' ? 'active' : ''}`,
        'aria-pressed': state.sellPaymentMethod === 'Efectivo' ? 'true' : 'false',
        onClick: () => {
          if (state.sellPaymentMethod !== 'Efectivo') {
            state.sellPaymentMethod = 'Efectivo';
            render();
          }
        },
      }, ['Efectivo']);

      const yapeBtn = createElement('button', {
        type: 'button',
        class: `door-segment-btn ${state.sellPaymentMethod === 'Yape' ? 'active' : ''}`,
        'aria-pressed': state.sellPaymentMethod === 'Yape' ? 'true' : 'false',
        onClick: () => {
          if (state.sellPaymentMethod !== 'Yape') {
            state.sellPaymentMethod = 'Yape';
            render();
          }
        },
      }, ['Yape']);

      const paymentGroup = createElement('div', { class: 'door-sell-field' }, [
        createElement('label', { class: 'door-field-label font-bold' }, ['Medio de pago']),
        createElement('div', { class: 'door-segment-group' }, [cashBtn, yapeBtn]),
      ]);
      sellSection.appendChild(paymentGroup);

      // Yape QR image
      if (state.sellPaymentMethod === 'Yape') {
        const yapeContainer = createElement('div', { class: 'door-yape-container' }, [
          createElement('img', {
            src: '/assets/public/yape-qr.jpg',
            alt: 'QR de Yape',
            class: 'door-yape-qr',
          }),
          createElement('p', { class: 'door-yape-hint' }, ['Muestra este QR al comprador']),
        ]);
        sellSection.appendChild(yapeContainer);
      }

      // Register Sell button
      const registerBtn = createButton({
        text: state.isSubmittingSell ? 'Registrando...' : 'Registrar venta',
        variant: 'primary',
        disabled: state.isSubmittingSell,
        onClick: async () => {
          if (state.isSubmittingSell) return;
          if (typeof onSell === 'function') {
            await onSell({
              tipoEntrada: selectedType.nombre,
              cantidad: state.sellQuantity,
              medioPago: state.sellPaymentMethod,
            });
          }
        },
      });
      registerBtn.el.classList.add('door-register-sell-btn');
      if (state.isSubmittingSell) {
        registerBtn.el.setAttribute('disabled', 'true');
      }
      sellSection.appendChild(registerBtn.el);
    }

    // Session Counter
    const sessionCounter = createElement('div', { class: 'door-sell-session-count font-bold' }, [
      `Vendidas en esta sesión: ${state.sessionSold || 0}`,
    ]);
    sellSection.appendChild(sessionCounter);

    sellPane.appendChild(sellSection);

    // Assemble container
    container.appendChild(topbar);
    container.appendChild(tabsNav);
    container.appendChild(scanPane);
    container.appendChild(sellPane);
  }

  render();

  function update(newState) {
    state = { ...state, ...newState };
    render();
  }

  function destroy() {
    container.innerHTML = '';
  }

  return {
    el: container,
    videoEl,
    update,
    destroy,
  };
}

/**
 * Controller orchestrating the door check-in and quick sales mode (#/puerta/:id).
 *
 * @param {{
 *   eventId: string,
 *   api?: any,
 *   audioController?: { stop: () => void },
 * }} deps 
 * @returns {{
 *   headerEl: HTMLElement | null,
 *   viewEl: HTMLElement,
 *   submitCode: (code: string) => Promise<void>,
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
  let sessionSold = 0;
  let destroyed = false;

  const view = createDoorView({
    eventName: 'Cargando...',
    isCameraActive: false,
    sessionSold: 0,
    onBack: () => toEvent(eventId),
    onToggleCamera: handleToggleCamera,
    onSubmitCode: handleSubmitCode,
    onTabChange: handleTabChange,
    onSell: handleSellTickets,
  });

  async function loadEvent() {
    try {
      const raw = await api.getEvent(eventId);
      eventData = (raw && raw.evento) || raw;
      view.update({
        eventName: eventData ? eventData.nombre : 'Evento',
        tiposDeEntrada: eventData ? (eventData.tipos_de_entrada || []) : [],
      });
    } catch {
      view.update({
        eventName: 'Evento',
        tiposDeEntrada: [],
      });
    }
  }

  loadEvent();

  // La cámara es lo primero: arranca sola al entrar al modo puerta.
  if (isCameraSupported()) {
    setTimeout(() => {
      if (!destroyed && !isCameraActive) startCamera();
    }, 300);
  }

  function handleTabChange(tab) {
    if (tab === 'sell' && isCameraActive) {
      stopCamera();
    } else if (tab === 'scan' && !isCameraActive && isCameraSupported()) {
      startCamera();
    }
  }

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
          view.update({
            result: {
              resultado: 'Invalida',
              motivo: formatCameraErrorMessage(err),
            },
          });
        },
      });
    }

    try {
      isCameraActive = true;
      view.update({ isCameraActive: true });
      await scanner.start();
    } catch (err) {
      stopCamera();
      view.update({
        result: {
          resultado: 'Invalida',
          motivo: formatCameraErrorMessage(err),
        },
      });
    }
  }

  function stopCamera() {
    if (scanner) {
      scanner.stop();
    }
    isCameraActive = false;
    view.update({ isCameraActive: false });
  }

  async function handleSubmitCode(code) {
    if (!code) return;
    const trimmed = String(code).trim();
    if (!trimmed) return;

    // Acepta el secreto de 32 hex directo o contenido en una URL de entrada
    let secret = trimmed;
    const hexMatch = trimmed.match(/[0-9a-fA-F]{32}/);
    if (hexMatch) {
      secret = hexMatch[0].toLowerCase();
    }

    // Only the full 32-hex QR secret reaches the backend. Anything else is
    // rejected locally so the operator sees it immediately.
    if (!/^[0-9a-fA-F]{32}$/.test(secret)) {
      view.update({
        result: { resultado: 'Invalida', motivo: 'Formato de código inválido' },
      });
      return;
    }

    await submitCheckin(secret);
  }

  async function submitCheckin(secret) {
    try {
      const result = await api.checkInTicket(eventId, secret);
      view.update({ result });
    } catch (err) {
      view.update({
        result: {
          resultado: 'Invalida',
          motivo: (err && err.message) || 'Error de conexión',
        },
      });
    }
  }

  async function handleSellTickets({ tipoEntrada, cantidad, medioPago }) {
    try {
      view.update({ isSubmittingSell: true, sellFeedback: null });

      const payload = {
        nombre_comprador: 'Venta en puerta',
        contacto: '',
        tipo_entrada: tipoEntrada,
        cantidad,
        medio_pago: medioPago,
        estado_pago: 'Pagado',
      };

      await api.sellTickets(eventId, payload);

      sessionSold += cantidad;
      const selectedType = resolveDoorTicketType(eventData ? eventData.tipos_de_entrada : []);
      const unitPrice = selectedType ? (Number(selectedType.precio) || 0) : 0;
      const totalFormatted = (unitPrice * cantidad).toFixed(2);

      view.update({
        isSubmittingSell: false,
        sessionSold,
        sellQuantity: 1,
        sellFeedback: {
          type: 'success',
          quantity: cantidad,
          total: totalFormatted,
          medioPago,
        },
      });
    } catch (err) {
      view.update({
        isSubmittingSell: false,
        sellFeedback: {
          type: 'error',
          message: (err && err.message) || 'Error al procesar la venta',
        },
      });
    }
  }

  function destroy() {
    destroyed = true;
    stopCamera();
    if (scanner) {
      scanner.destroy();
      scanner = null;
    }
    view.destroy();
  }

  return {
    headerEl: null,
    viewEl: view.el,
    // Entry point for a decoded QR. The camera scanner calls it, and so does
    // the test suite; the only accepted value is the full 32-hex secret.
    submitCode: handleSubmitCode,
    destroy,
  };
}
