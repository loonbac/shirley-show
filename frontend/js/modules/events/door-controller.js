/**
 * Door Access Control View and Controller (#/puerta/:id).
 * Optimized for mobile door scanning with Camera BarcodeDetector & jsQR fallback,
 * always-visible manual input, autofocus recovery, unambiguous 3-state banners,
 * and quick door sales tab (DOOR-1 & DOOR-2).
 */

import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { createIcon } from '../../components/atoms/icon.js';
import { toEvent } from '../../router/navigate.js';
import * as defaultApi from './api.js';
import { isBarcodeDetectorSupported, isCameraSupported, createCameraQrScanner } from './camera-qr-scanner.js';
import { formatPublicDateTime } from './public-ticket-controller.js';
import { bindBottomSheetDrag } from '../../behaviors/bind-bottom-sheet-drag.js';

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
    turno: props.turno || {
      numero: 1,
      nombre: 'Turno 1',
      iniciadoEn: new Date().toLocaleTimeString('es-PE', { hour: '2-digit', minute: '2-digit' }),
      ingresados: 0,
      alertas: 0,
      ventas: 0,
      totalVentasSoles: 0,
      porTipo: {},
    },
    historialEscaneos: Array.isArray(props.historialEscaneos) ? props.historialEscaneos : [],
    totalEventoIngresados: typeof props.totalEventoIngresados === 'number' ? props.totalEventoIngresados : 0,
    totalEventoEmitidas: typeof props.totalEventoEmitidas === 'number' ? props.totalEventoEmitidas : 0,
    mostrarModalCierreTurno: Boolean(props.mostrarModalCierreTurno),
    reporteCierreTurno: props.reporteCierreTurno || null,
  };

  const {
    onSubmitCode,
    onToggleCamera,
    onBack,
    onTabChange,
    onSell,
    onCerrarTurno,
    onConfirmarNuevoTurno,
    onCerrarModalTurno,
  } = props;

  const container = createElement('div', { class: 'door-container' });

  // Camera video element (always exists for scanner binding)
  const videoEl = createElement('video', {
    class: 'door-camera-video',
    autoplay: 'true',
    muted: 'true',
    playsinline: 'true',
  });

  let isHistoryExpanded = false;
  let activeBottomSheetDrag = null;

  function onGlobalKeyDown(e) {
    if (e.key === 'Escape' && isHistoryExpanded) {
      isHistoryExpanded = false;
      const logSec = container.querySelector('.door-scan-log-section');
      const backdrop = container.querySelector('.door-history-backdrop');
      if (logSec) logSec.classList.remove('is-expanded');
      if (backdrop) backdrop.classList.remove('is-visible');
      const affordance = container.querySelector('.door-log-expand-btn');
      const titleText = container.querySelector('.door-log-title');
      if (affordance && titleText) {
        affordance.innerHTML = '';
        affordance.appendChild(createIcon({ name: 'chevron-up', size: 'sm' }));
        affordance.appendChild(createElement('span', {}, ['Ver todo']));
        titleText.textContent = `Historial del Turno (${state.historialEscaneos.length})`;
      }
    }
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('keydown', onGlobalKeyDown);
  }

  function render() {
    if (activeBottomSheetDrag) {
      activeBottomSheetDrag.destroy();
      activeBottomSheetDrag = null;
    }
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

    const topbar = createElement('div', { class: 'door-topbar' }, [
      backBtn.el,
      headerTitle,
    ]);

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
        createIcon({ name: 'smartphone', size: 'sm', className: 'door-notice-icon' }),
        createElement('span', {}, ['Recomendación: Usa un teléfono móvil para escanear con mayor agilidad en la puerta.']),
      ]);
      scanPane.appendChild(deviceNotice);
    }

    // Barra de Turno y Control de Sesión
    const shiftSummaryBar = createElement('div', { class: 'door-shift-bar' }, [
      createElement('div', { class: 'door-shift-info' }, [
        createElement('span', { class: 'door-shift-badge' }, [
          createElement('span', { class: 'door-status-dot is-active' }),
          createElement('span', { class: 'door-shift-badge-text' }, [state.turno.nombre || 'Turno Activo']),
        ]),
        createElement('span', { class: 'door-shift-time text-muted' }, [`Inicio: ${state.turno.iniciadoEn || '--:--'}`]),
      ]),
      createButton({
        text: 'Cerrar Turno',
        variant: 'secondary',
        size: 'sm',
        onClick: onCerrarTurno || (() => {}),
      }).el,
    ]);
    scanPane.appendChild(shiftSummaryBar);

    // Contadores en vivo de asistencia
    const metricsGrid = createElement('div', { class: 'door-metrics-kpis' }, [
      createElement('div', { class: 'door-kpi-card door-kpi-turno' }, [
        createElement('span', { class: 'door-kpi-num text-success font-bold' }, [String(state.turno.ingresados || 0)]),
        createElement('span', { class: 'door-kpi-title' }, ['En este Turno']),
      ]),
      createElement('div', { class: 'door-kpi-card door-kpi-total' }, [
        createElement('span', { class: 'door-kpi-num font-bold' }, [
          `${state.totalEventoIngresados || 0} / ${state.totalEventoEmitidas || state.totalEventoIngresados || 0}`,
        ]),
        createElement('span', { class: 'door-kpi-title' }, ['Asistencia Total']),
      ]),
      createElement('div', { class: 'door-kpi-card door-kpi-alertas' }, [
        createElement('span', { class: `door-kpi-num font-bold ${(state.turno.alertas || 0) > 0 ? 'text-warning' : ''}` }, [
          String(state.turno.alertas || 0),
        ]),
        createElement('span', { class: 'door-kpi-title' }, ['Alertas / Repetidos']),
      ]),
      createElement('div', { class: 'door-kpi-card door-kpi-ventas' }, [
        createElement('span', { class: 'door-kpi-num font-bold' }, [String(state.turno.ventas || 0)]),
        createElement('span', { class: 'door-kpi-title' }, ['Ventas Puerta']),
      ]),
    ]);
    scanPane.appendChild(metricsGrid);

    // Camera Viewfinder Section (Tap to Toggle: Active / Battery Saver)
    const cameraSection = createElement('div', {
      class: `door-camera-section ${state.isCameraActive ? 'is-active' : 'is-paused'}`,
      role: 'button',
      tabindex: '0',
      'aria-label': state.isCameraActive ? 'Pausar cámara' : 'Activar cámara',
      title: state.isCameraActive ? 'Toca para pausar la cámara (Ahorro de batería)' : 'Toca para activar la cámara',
      onClick: () => {
        if (typeof onToggleCamera === 'function') {
          onToggleCamera();
        }
      },
    }, [
      videoEl,
      createElement('div', { class: `door-camera-guide ${state.isCameraActive ? 'is-active' : 'is-paused'}` }),
      createElement('div', { class: 'door-camera-hud' }, [
        state.isCameraActive
          ? createElement('span', { class: 'door-camera-pill' }, [
              createElement('span', { class: 'door-status-dot is-live' }),
              createElement('span', {}, ['En vivo • Toca para pausar']),
            ])
          : createElement('div', { class: 'door-camera-standby' }, [
              createElement('span', { class: 'door-camera-standby-icon' }, [
                createIcon({ name: 'camera', size: 'lg', className: 'door-camera-standby-svg' }),
              ]),
              createElement('strong', { class: 'door-camera-standby-title' }, ['Toca para activar la cámara']),
              createElement('span', { class: 'door-camera-standby-sub' }, ['Ahorro de batería activo']),
            ]),
      ]),
    ]);
    videoEl.style.display = state.isCameraActive ? 'block' : 'none';
    scanPane.appendChild(cameraSection);

    // Unambiguous 3-State Feedback Banner (Compact HUD docked bar)
    let bannerEl = null;
    const res = state.result;

    if (!res) {
      bannerEl = createElement('div', { class: 'door-banner door-banner-idle' }, [
        createElement('div', { class: 'door-banner-icon' }, [
          createIcon({ name: 'ticket', size: 'lg', className: 'door-banner-svg' }),
        ]),
        createElement('div', { class: 'door-banner-content' }, [
          createElement('h3', { class: 'door-banner-title' }, ['Listo para Validar']),
          createElement('p', { class: 'door-banner-sub' }, ['Apunta al código QR o toca la cámara']),
        ]),
      ]);
    } else if (res.resultado === 'Valida') {
      bannerEl = createElement('div', { class: 'door-banner door-banner-valid' }, [
        createElement('div', { class: 'door-banner-icon' }, [
          createIcon({ name: 'check', size: 'lg', className: 'door-banner-svg' }),
        ]),
        createElement('div', { class: 'door-banner-content' }, [
          createElement('h2', { class: 'door-banner-title' }, ['ENTRADA VÁLIDA']),
          createElement('div', { class: 'door-banner-details' }, [
            createElement('span', { class: 'door-banner-buyer font-bold' }, [res.nombre_comprador || 'Asistente']),
            createElement('span', { class: 'door-banner-sep' }, [' • ']),
            createElement('span', { class: 'door-banner-type' }, [`Tipo: ${res.tipo_entrada || 'General'}`]),
            createElement('span', { class: 'door-banner-sep' }, [' • ']),
            createElement('span', { class: 'door-banner-time' }, [
              `Ingreso: ${formatPublicDateTime(res.hora_ingreso)}`,
            ]),
          ]),
        ]),
      ]);
    } else if (res.resultado === 'Ya Usada') {
      bannerEl = createElement('div', { class: 'door-banner door-banner-used' }, [
        createElement('div', { class: 'door-banner-icon' }, [
          createIcon({ name: 'alert-triangle', size: 'lg', className: 'door-banner-svg' }),
        ]),
        createElement('div', { class: 'door-banner-content' }, [
          createElement('h2', { class: 'door-banner-title' }, ['YA FUE USADA']),
          createElement('div', { class: 'door-banner-details' }, [
            createElement('span', { class: 'door-banner-sub' }, ['Esta entrada ya fue registrada']),
            createElement('span', { class: 'door-banner-sep' }, [' • ']),
            createElement('span', { class: 'door-banner-time' }, [
              `Ingreso: ${formatPublicDateTime(res.hora_ingreso)}`,
            ]),
          ]),
        ]),
      ]);
    } else {
      // Invalida
      bannerEl = createElement('div', { class: 'door-banner door-banner-invalid' }, [
        createElement('div', { class: 'door-banner-icon' }, [
          createIcon({ name: 'close', size: 'lg', className: 'door-banner-svg' }),
        ]),
        createElement('div', { class: 'door-banner-content' }, [
          createElement('h2', { class: 'door-banner-title' }, ['ENTRADA INVÁLIDA']),
          createElement('div', { class: 'door-banner-details' }, [
            createElement('span', { class: 'door-banner-sub font-bold' }, [
              res.motivo ? `Motivo: ${res.motivo}` : 'Código no reconocido',
            ]),
          ]),
        ]),
      ]);
    }
    scanPane.appendChild(bannerEl);

    // Check-in happens only by scanning the QR. There is no manual code entry:
    // typing a short prefix would turn the 128-bit secret into a guessable code.
    scanPane.appendChild(
      createElement('p', { class: 'door-scan-hint' }, [
        'Apunta la cámara al código QR de la entrada.',
      ])
    );

    // Backdrop para el modal transparente de historial expandido
    const historyBackdrop = createElement('div', { class: 'door-history-backdrop' });

    function setHistoryExpanded(expanded) {
      isHistoryExpanded = !!expanded;
      if (logSection) {
        if (isHistoryExpanded) {
          logSection.classList.add('is-expanded');
          historyBackdrop.classList.add('is-visible');
        } else {
          logSection.classList.remove('is-expanded');
          historyBackdrop.classList.remove('is-visible');
        }
        updateHistoryHeader();
      }
    }

    historyBackdrop.addEventListener('click', () => {
      setHistoryExpanded(false);
    });

    // Bitácora en vivo de escaneos del turno (Deslizable a modal transparente)
    const handleBar = createElement('div', {
      class: 'door-log-handle',
      role: 'button',
      tabindex: '0',
      'aria-label': 'Expandir o contraer historial del turno',
    }, [
      createElement('span', { class: 'door-log-handle-pill' }),
    ]);

    const headerIcon = createIcon({
      name: 'history',
      size: 'sm',
      className: 'door-log-header-icon',
    });

    const headerTitleText = createElement('strong', { class: 'door-log-title' }, [
      `Historial del Turno (${state.historialEscaneos.length})`,
    ]);

    const headerAffordanceBtn = createElement('button', {
      type: 'button',
      class: 'door-log-expand-btn',
      'aria-label': 'Expandir o contraer historial',
    });

    function updateHistoryHeader() {
      headerAffordanceBtn.innerHTML = '';
      if (isHistoryExpanded) {
        headerAffordanceBtn.appendChild(createIcon({ name: 'chevron-down', size: 'sm' }));
        headerAffordanceBtn.appendChild(createElement('span', {}, ['Cerrar']));
        headerTitleText.textContent = `Historial Completo (${state.historialEscaneos.length})`;
      } else {
        headerAffordanceBtn.appendChild(createIcon({ name: 'chevron-up', size: 'sm' }));
        headerAffordanceBtn.appendChild(createElement('span', {}, ['Ver todo']));
        headerTitleText.textContent = `Historial del Turno (${state.historialEscaneos.length})`;
      }
    }
    updateHistoryHeader();

    let bottomSheetDrag = null;

    handleBar.addEventListener('click', (e) => {
      e.stopPropagation();
      if (bottomSheetDrag && bottomSheetDrag.isJustDragged()) return;
      setHistoryExpanded(!isHistoryExpanded);
    });

    headerAffordanceBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      if (bottomSheetDrag && bottomSheetDrag.isJustDragged()) return;
      setHistoryExpanded(!isHistoryExpanded);
    });

    const logHeader = createElement('div', { class: 'door-scan-log-header' }, [
      createElement('div', { class: 'door-log-header-left' }, [
        headerIcon,
        headerTitleText,
      ]),
      headerAffordanceBtn,
    ]);
    logHeader.addEventListener('click', () => {
      if (bottomSheetDrag && bottomSheetDrag.isJustDragged()) return;
      setHistoryExpanded(!isHistoryExpanded);
    });

    const logSection = createElement('div', {
      class: `door-scan-log-section ${isHistoryExpanded ? 'is-expanded' : ''}`,
    }, [
      handleBar,
      logHeader,
    ]);

    if (isHistoryExpanded) {
      historyBackdrop.classList.add('is-visible');
    }

    if (state.historialEscaneos.length === 0) {
      logSection.appendChild(
        createElement('p', { class: 'door-scan-log-empty text-muted text-center' }, [
          'Aún no hay escaneos en este turno.',
        ])
      );
    } else {
      const logList = createElement('div', { class: 'door-scan-log-list' });
      state.historialEscaneos.forEach((item) => {
        const badgeClass =
          item.resultado === 'Valida'
            ? 'events-badge-success'
            : item.resultado === 'Ya Usada'
            ? 'events-badge-warning'
            : 'events-badge-danger';

        const resultIconName =
          item.resultado === 'Valida'
            ? 'check'
            : item.resultado === 'Ya Usada'
            ? 'alert-triangle'
            : 'close';

        const badgeIcon = createIcon({ name: resultIconName, size: 'sm', className: 'door-badge-icon' });

        const row = createElement('div', {
          class: `door-scan-log-row door-log-${String(item.resultado || '').toLowerCase().replace(/\s+/g, '-')}`,
        }, [
          createElement('span', { class: 'door-log-time' }, [item.hora || '']),
          createElement('div', { class: 'door-log-person' }, [
            createElement('strong', { class: 'door-log-name' }, [item.nombre || 'Asistente']),
            item.tipo ? createElement('span', { class: 'door-log-type text-muted' }, [item.tipo]) : null,
          ].filter(Boolean)),
          createElement('span', { class: `events-badge ${badgeClass} door-log-badge font-bold` }, [
            badgeIcon,
            createElement('span', {}, [item.resultado]),
          ]),
        ]);
        logList.appendChild(row);
      });
      logSection.appendChild(logList);
    }

    // Arrastre táctil interactivo en tiempo real (1:1 siguiendo el dedo)
    bottomSheetDrag = bindBottomSheetDrag({
      sheetEl: logSection,
      dragHandles: [handleBar, logHeader],
      scrollEl: logSection.querySelector('.door-scan-log-list'),
      backdropEl: historyBackdrop,
      getIsExpanded: () => isHistoryExpanded,
      onExpandChange: (expanded) => {
        setHistoryExpanded(expanded);
      },
    });
    activeBottomSheetDrag = bottomSheetDrag;

    // Espaciador que reserva el lugar exacto del historial acoplado en el flujo del scanPane
    const logSpacer = createElement('div', { class: 'door-scan-log-spacer' });
    scanPane.appendChild(logSpacer);
    scanPane.appendChild(logSection);

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
        createElement('div', { class: 'door-banner-icon' }, [
          createIcon({ name: 'close', size: 'lg', className: 'door-banner-svg' }),
        ]),
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
            createElement('div', { class: 'door-banner-icon' }, [
              createIcon({ name: 'check', size: 'lg', className: 'door-banner-svg' }),
            ]),
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
            createElement('div', { class: 'door-banner-icon' }, [
              createIcon({ name: 'close', size: 'lg', className: 'door-banner-svg' }),
            ]),
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
    container.appendChild(historyBackdrop);

    if (state.mostrarModalCierreTurno && state.reporteCierreTurno) {
      const rep = state.reporteCierreTurno;
      const modalOverlay = createElement('div', { class: 'door-modal-overlay' }, [
        createElement('div', { class: 'door-modal-card' }, [
          createElement('h3', { class: 'door-modal-title' }, [`Cierre de ${rep.nombre}`]),
          createElement('p', { class: 'door-modal-period text-muted font-bold' }, [
            `Período: ${rep.iniciadoEn} a ${rep.cerradoEn}`,
          ]),
          createElement('div', { class: 'door-modal-stats' }, [
            createElement('div', { class: 'door-modal-stat-row' }, [
              createElement('span', {}, ['Total ingresados en el turno:']),
              createElement('strong', { class: 'text-success' }, [String(rep.ingresados || 0)]),
            ]),
            createElement('div', { class: 'door-modal-stat-row' }, [
              createElement('span', {}, ['Alertas (Ya Usadas / Inválidas):']),
              createElement('strong', { class: 'text-warning' }, [String(rep.alertas || 0)]),
            ]),
            createElement('div', { class: 'door-modal-stat-row' }, [
              createElement('span', {}, ['Entradas vendidas en puerta:']),
              createElement('strong', {}, [`${rep.ventas || 0} (S/ ${(rep.totalVentasSoles || 0).toFixed(2)})`]),
            ]),
            createElement('div', { class: 'door-modal-stat-row' }, [
              createElement('span', {}, ['Asistencia acumulada del evento:']),
              createElement('strong', {}, [`${state.totalEventoIngresados || 0} ingresados`]),
            ]),
          ]),
          createElement('div', { class: 'door-modal-actions' }, [
            createButton({
              text: 'Iniciar Siguiente Turno',
              variant: 'primary',
              fullWidth: true,
              onClick: onConfirmarNuevoTurno || (() => {}),
            }).el,
            createButton({
              text: 'Volver al Turno Actual',
              variant: 'ghost',
              fullWidth: true,
              onClick: onCerrarModalTurno || (() => {}),
            }).el,
          ]),
        ]),
      ]);
      container.appendChild(modalOverlay);
    }
  }

  render();

  function update(newState) {
    state = { ...state, ...newState };
    render();
  }

  function destroy() {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', onGlobalKeyDown);
    }
    if (activeBottomSheetDrag) {
      activeBottomSheetDrag.destroy();
      activeBottomSheetDrag = null;
    }
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

  if (typeof document !== 'undefined' && document.body) {
    document.body.classList.add('door-mode-active');
  }

  let eventData = null;
  let scanner = null;
  let isCameraActive = false;
  let sessionSold = 0;
  let destroyed = false;

  const shiftKey = `shirley_door_shift_${eventId}`;
  let turno = {
    numero: 1,
    nombre: 'Turno 1',
    iniciadoEn: new Date().toLocaleTimeString('es-PE', { hour: '2-digit', minute: '2-digit' }),
    ingresados: 0,
    alertas: 0,
    ventas: 0,
    totalVentasSoles: 0,
    porTipo: {},
  };
  let historialEscaneos = [];
  let totalEventoIngresados = 0;
  let totalEventoEmitidas = 0;

  try {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem(shiftKey);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed && parsed.turno) {
          turno = parsed.turno;
          historialEscaneos = Array.isArray(parsed.historial) ? parsed.historial : [];
        }
      }
    }
  } catch {}

  function saveShiftToStorage() {
    try {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(shiftKey, JSON.stringify({ turno, historial: historialEscaneos.slice(0, 50) }));
      }
    } catch {}
  }

  const view = createDoorView({
    eventName: 'Cargando...',
    isCameraActive: false,
    sessionSold: 0,
    turno,
    historialEscaneos,
    totalEventoIngresados,
    totalEventoEmitidas,
    onBack: () => toEvent(eventId),
    onToggleCamera: handleToggleCamera,
    onSubmitCode: handleSubmitCode,
    onTabChange: handleTabChange,
    onSell: handleSellTickets,
    onCerrarTurno: handleCerrarTurno,
    onConfirmarNuevoTurno: handleConfirmarNuevoTurno,
    onCerrarModalTurno: handleCerrarModalTurno,
  });

  async function loadEvent() {
    try {
      const [rawEvent, rawSummary] = await Promise.all([
        api.getEvent(eventId),
        typeof api.getEventSummary === 'function' ? api.getEventSummary(eventId).catch(() => null) : null,
      ]);
      eventData = (rawEvent && rawEvent.evento) || rawEvent;
      const sum = (rawSummary && rawSummary.resumen) || rawSummary;
      if (sum) {
        totalEventoIngresados = sum.entradas_ingresadas || 0;
        totalEventoEmitidas = sum.entradas_emitidas || 0;
      }
      view.update({
        eventName: eventData ? eventData.nombre : 'Evento',
        tiposDeEntrada: eventData ? (eventData.tipos_de_entrada || []) : [],
        totalEventoIngresados,
        totalEventoEmitidas,
        turno: { ...turno },
        historialEscaneos: [...historialEscaneos],
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
  let autoStartCameraTimer = null;
  if (isCameraSupported()) {
    autoStartCameraTimer = setTimeout(() => {
      if (!destroyed && !isCameraActive) startCamera();
    }, 150);
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
      const horaStr = new Date().toLocaleTimeString('es-PE', { hour: '2-digit', minute: '2-digit', second: '2-digit' });

      if (result.resultado === 'Valida') {
        turno.ingresados++;
        totalEventoIngresados++;
        const t = result.tipo_entrada || 'General';
        turno.porTipo[t] = (turno.porTipo[t] || 0) + 1;
        historialEscaneos.unshift({
          hora: horaStr,
          nombre: result.nombre_comprador || 'Asistente',
          tipo: t,
          resultado: 'Valida',
        });
      } else if (result.resultado === 'Ya Usada') {
        turno.alertas++;
        historialEscaneos.unshift({
          hora: horaStr,
          nombre: 'Entrada Ya Usada',
          tipo: result.hora_ingreso ? `Ingresó: ${formatPublicDateTime(result.hora_ingreso)}` : '',
          resultado: 'Ya Usada',
        });
      } else {
        turno.alertas++;
        historialEscaneos.unshift({
          hora: horaStr,
          nombre: 'Inválida',
          tipo: result.motivo || 'No autorizada',
          resultado: 'Invalida',
        });
      }

      saveShiftToStorage();
      view.update({
        result,
        turno: { ...turno },
        historialEscaneos: [...historialEscaneos],
        totalEventoIngresados,
      });
    } catch (err) {
      view.update({
        result: {
          resultado: 'Invalida',
          motivo: (err && err.message) || 'Error de conexión',
        },
      });
    }
  }

  function handleCerrarTurno() {
    const reporte = {
      numero: turno.numero,
      nombre: turno.nombre,
      iniciadoEn: turno.iniciadoEn,
      cerradoEn: new Date().toLocaleTimeString('es-PE', { hour: '2-digit', minute: '2-digit' }),
      ingresados: turno.ingresados,
      alertas: turno.alertas,
      ventas: turno.ventas,
      totalVentasSoles: turno.totalVentasSoles,
      porTipo: { ...turno.porTipo },
    };
    view.update({
      mostrarModalCierreTurno: true,
      reporteCierreTurno: reporte,
    });
  }

  function handleConfirmarNuevoTurno() {
    const nextNum = (turno.numero || 1) + 1;
    turno = {
      numero: nextNum,
      nombre: `Turno ${nextNum}`,
      iniciadoEn: new Date().toLocaleTimeString('es-PE', { hour: '2-digit', minute: '2-digit' }),
      ingresados: 0,
      alertas: 0,
      ventas: 0,
      totalVentasSoles: 0,
      porTipo: {},
    };
    historialEscaneos = [];
    saveShiftToStorage();
    view.update({
      mostrarModalCierreTurno: false,
      reporteCierreTurno: null,
      turno: { ...turno },
      historialEscaneos: [],
    });
  }

  function handleCerrarModalTurno() {
    view.update({
      mostrarModalCierreTurno: false,
      reporteCierreTurno: null,
    });
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

      turno.ventas += cantidad;
      turno.totalVentasSoles += unitPrice * cantidad;
      saveShiftToStorage();

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
        turno: { ...turno },
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
    if (autoStartCameraTimer) {
      clearTimeout(autoStartCameraTimer);
      autoStartCameraTimer = null;
    }
    stopCamera();
    if (typeof document !== 'undefined' && document.body) {
      document.body.classList.remove('door-mode-active');
    }
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
