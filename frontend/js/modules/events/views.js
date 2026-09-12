import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../../components/atoms/button.js';
import { createSpinner } from '../../components/atoms/spinner.js';

/**
 * Formats an ISO date string into a friendly localized date and time in Peru (es-PE).
 * @param {string} isoString 
 * @returns {string}
 */
export function formatEventDateTime(isoString) {
  if (!isoString) return '';
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return isoString;
    return d.toLocaleString('es-PE', {
      weekday: 'short',
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return isoString;
  }
}

/**
 * Creates the events catalog view organism.
 * Displays upcoming and finalized events with responsive cards, search, and creation actions.
 *
 * @param {{
 *   events: Array<any>,
 *   isLoading: boolean,
 *   errorMessage: string | null,
 *   onCreateEvent: () => void,
 *   onSelectEvent: (id: string) => void,
 *   onEditEvent: (event: any) => void,
 *   onDeleteEvent: (event: any) => void,
 * }} props 
 * @returns {{ el: HTMLElement, update: (props: Partial<typeof props>) => void }}
 */
export function createEventsCatalogView(props) {
  let {
    events = [],
    isLoading = false,
    errorMessage = null,
    onCreateEvent,
    onSelectEvent,
    onEditEvent,
    onDeleteEvent,
  } = props;

  const el = createElement('div', { class: 'events-catalog-view' });

  function render() {
    el.innerHTML = '';

    // Header bar
    const createBtn = createButton({
      text: '+ Nuevo Evento',
      variant: 'primary',
      onClick: onCreateEvent,
    });

    const headerEl = createElement('div', { class: 'events-catalog-header' }, [
      createElement('div', {}, [
        createElement('h2', { class: 'events-catalog-title' }, ['Catálogo de Eventos']),
        createElement('p', { class: 'events-catalog-subtitle' }, ['Gestión de fechas, venta de entradas y finanzas de Shirley Show']),
      ]),
      createBtn.el,
    ]);
    el.appendChild(headerEl);

    // Loading state
    if (isLoading) {
      const spinner = createSpinner({ size: 'lg' });
      const loadingEl = createElement('div', { class: 'events-state-container' }, [
        spinner.el,
        createElement('p', { class: 'events-state-text' }, ['Cargando eventos...']),
      ]);
      el.appendChild(loadingEl);
      return;
    }

    // Error state
    if (errorMessage) {
      const errorCard = createElement('div', { class: 'events-state-container events-error-box' }, [
        createElement('p', { class: 'events-error-text' }, [errorMessage]),
      ]);
      el.appendChild(errorCard);
      return;
    }

    // Empty state
    if (!events || events.length === 0) {
      const emptyCard = createElement('div', { class: 'events-state-container events-empty-box' }, [
        createElement('h3', { class: 'events-empty-title' }, ['No hay eventos registrados']),
        createElement('p', { class: 'events-empty-desc' }, ['Organiza tu primer concierto, cena show o presentación con boletería.']),
        createButton({
          text: 'Crear Primer Evento',
          variant: 'primary',
          onClick: onCreateEvent,
        }).el,
      ]);
      el.appendChild(emptyCard);
      return;
    }

    // Split active vs finalized events
    const activeEvents = events.filter((e) => Number(e.finalizado) === 0);
    const finalizedEvents = events.filter((e) => Number(e.finalizado) === 1);

    function createEventCard(event) {
      const isFinalized = Number(event.finalizado) === 1;

      // Poster or placeholder
      let posterEl;
      if (event.imagen) {
        posterEl = createElement('img', {
          src: `/api/eventos/${encodeURIComponent(event.id)}/imagen`,
          alt: `Afiche de ${event.nombre}`,
          class: 'events-card-poster',
          loading: 'lazy',
        });
      } else {
        posterEl = createElement('div', { class: 'events-card-poster-placeholder' }, [
          createElement('span', { class: 'events-poster-icon' }, ['🎭']),
        ]);
      }

      const badgeEl = createElement(
        'span',
        { class: `events-badge ${isFinalized ? 'events-badge-muted' : 'events-badge-active'}` },
        [isFinalized ? 'Finalizado' : 'Activo']
      );

      const titleEl = createElement('h3', { class: 'events-card-title' }, [event.nombre]);
      const dateEl = createElement('div', { class: 'events-card-meta' }, [
        createElement('span', { class: 'events-card-icon' }, ['📅']),
        createElement('span', {}, [formatEventDateTime(event.fecha_hora)]),
      ]);
      const venueEl = createElement('div', { class: 'events-card-meta' }, [
        createElement('span', { class: 'events-card-icon' }, ['📍']),
        createElement('span', {}, [event.lugar]),
      ]);

      // Ticket types summary
      const typesList = Array.isArray(event.tipos_de_entrada)
        ? event.tipos_de_entrada.map((t) => `${t.nombre}: S/ ${Number(t.precio).toFixed(2)}`).join(' • ')
        : '';
      const typesEl = createElement('div', { class: 'events-card-types' }, [typesList]);

      const openBtn = createButton({
        text: 'Ver Detalle',
        variant: 'primary',
        size: 'sm',
        onClick: (e) => {
          e.stopPropagation();
          onSelectEvent(event.id);
        },
      });

      const editBtn = createButton({
        text: 'Editar',
        variant: 'ghost',
        size: 'sm',
        onClick: (e) => {
          e.stopPropagation();
          onEditEvent(event);
        },
      });

      const delBtn = createButton({
        text: 'Eliminar',
        variant: 'danger',
        size: 'sm',
        onClick: (e) => {
          e.stopPropagation();
          onDeleteEvent(event);
        },
      });

      const actionsEl = createElement('div', { class: 'events-card-actions' }, [
        openBtn.el,
        editBtn.el,
        delBtn.el,
      ]);

      const bodyEl = createElement('div', { class: 'events-card-body' }, [
        createElement('div', { class: 'events-card-header-row' }, [titleEl, badgeEl]),
        dateEl,
        venueEl,
        typesEl,
        actionsEl,
      ]);

      const card = createElement(
        'article',
        {
          class: `events-card ${isFinalized ? 'is-finalized' : ''}`,
          tabindex: '0',
          role: 'button',
          'aria-label': `Evento ${event.nombre}`,
          onClick: () => onSelectEvent(event.id),
        },
        [posterEl, bodyEl]
      );

      return card;
    }

    // Active Section
    if (activeEvents.length > 0) {
      const activeSection = createElement('section', { class: 'events-section' }, [
        createElement('h3', { class: 'events-section-title' }, [`Próximos Eventos (${activeEvents.length})`]),
        createElement('div', { class: 'events-grid' }, activeEvents.map(createEventCard)),
      ]);
      el.appendChild(activeSection);
    }

    // Finalized Section
    if (finalizedEvents.length > 0) {
      const finalizedSection = createElement('section', { class: 'events-section events-finalized-section' }, [
        createElement('h3', { class: 'events-section-title' }, [`Eventos Finalizados (${finalizedEvents.length})`]),
        createElement('div', { class: 'events-grid' }, finalizedEvents.map(createEventCard)),
      ]);
      el.appendChild(finalizedSection);
    }
  }

  render();

  function update(nextProps = {}) {
    if (nextProps.events !== undefined) events = nextProps.events;
    if (nextProps.isLoading !== undefined) isLoading = nextProps.isLoading;
    if (nextProps.errorMessage !== undefined) errorMessage = nextProps.errorMessage;
    if (nextProps.onCreateEvent !== undefined) onCreateEvent = nextProps.onCreateEvent;
    if (nextProps.onSelectEvent !== undefined) onSelectEvent = nextProps.onSelectEvent;
    if (nextProps.onEditEvent !== undefined) onEditEvent = nextProps.onEditEvent;
    if (nextProps.onDeleteEvent !== undefined) onDeleteEvent = nextProps.onDeleteEvent;
    render();
  }

  return { el, update };
}

/**
 * Creates the complete event detail view with administrative tabs:
 * Resumen (7 metrics), Entradas, Interesados, and Gastos.
 *
 * @param {{
 *   event: any,
 *   summary: any,
 *   tickets: Array<any>,
 *   interests: Array<any>,
 *   expenses: Array<any>,
 *   activeTab: 'resumen' | 'entradas' | 'interesados' | 'gastos',
 *   onTabChange: (tab: string) => void,
 *   onFinalize: () => void,
 *   onReopen: () => void,
 *   onUploadPoster: (file: File) => void,
 *   onOpenDoor: () => void,
 *   onEditEvent: () => void,
 *   onDeleteEvent: () => void,
 *   onSellTickets: () => void,
 *   onConfirmPayment: (ticketId: string) => void,
 *   onDeleteTicket: (ticket: any) => void,
 *   onTicketPhoto?: (ticket: any) => void,
 *   onCreateInterest: () => void,
 *   onEditInterest: (interest: any) => void,
 *   onDeleteInterest: (interest: any) => void,
 *   onConvertInterest: (interest: any) => void,
 *   onCreateExpense: () => void,
 *   onEditExpense: (expense: any) => void,
 *   onDeleteExpense: (expense: any) => void,
 * }} props 
 * @returns {{ el: HTMLElement, update: (props: Partial<typeof props>) => void }}
 */
export function createEventDetailView(props) {
  let state = { ...props };
  const el = createElement('div', { class: 'events-detail-view' });

  function renderHero() {
    const { event, onFinalize, onReopen, onUploadPoster, onOpenDoor, onEditEvent, onDeleteEvent } = state;
    if (!event) return createElement('div');

    const isFinalized = Number(event.finalizado) === 1;

    // Poster element + upload input
    const fileInput = createElement('input', {
      type: 'file',
      accept: 'image/jpeg,image/png,image/webp',
      style: 'display: none;',
      onChange: (e) => {
        const file = e.target.files && e.target.files[0];
        if (file && typeof onUploadPoster === 'function') {
          onUploadPoster(file);
        }
      },
    });

    let posterDisplay;
    if (event.imagen) {
      posterDisplay = createElement('div', { class: 'events-hero-poster-wrapper' }, [
        createElement('img', {
          src: `/api/eventos/${encodeURIComponent(event.id)}/imagen`,
          alt: event.nombre,
          class: 'events-hero-poster-img',
        }),
        createElement('button', {
          type: 'button',
          class: 'btn btn-ghost btn-sm events-poster-change-btn',
          onClick: () => fileInput.click(),
        }, ['Cambiar afiche']),
      ]);
    } else {
      posterDisplay = createElement('div', { class: 'events-hero-poster-wrapper' }, [
        createElement('div', { class: 'events-hero-poster-empty' }, ['Sin afiche']),
        createElement('button', {
          type: 'button',
          class: 'btn btn-secondary btn-sm events-poster-change-btn',
          onClick: () => fileInput.click(),
        }, ['+ Subir afiche']),
      ]);
    }

    const badge = createElement(
      'span',
      { class: `events-badge ${isFinalized ? 'events-badge-muted' : 'events-badge-active'}` },
      [isFinalized ? 'Finalizado' : 'Activo']
    );

    const titleEl = createElement('h2', { class: 'events-hero-title' }, [event.nombre]);
    const metaEl = createElement('div', { class: 'events-hero-meta' }, [
      createElement('div', { class: 'events-hero-meta-item' }, [
        createElement('span', {}, ['📅 ']),
        createElement('strong', {}, [formatEventDateTime(event.fecha_hora)]),
      ]),
      createElement('div', { class: 'events-hero-meta-item' }, [
        createElement('span', {}, ['📍 ']),
        createElement('span', {}, [event.lugar]),
      ]),
    ]);

    // Primary action: Door mode
    const doorBtn = createButton({
      text: '🚪 Modo Puerta',
      variant: 'primary',
      onClick: onOpenDoor,
    });

    const statusToggleBtn = isFinalized
      ? createButton({ text: 'Reabrir Evento', variant: 'secondary', onClick: onReopen })
      : createButton({ text: 'Finalizar Evento', variant: 'secondary', onClick: onFinalize });

    const editBtn = createButton({ text: 'Editar', variant: 'ghost', onClick: onEditEvent });
    const deleteBtn = createButton({ text: 'Eliminar', variant: 'danger', onClick: onDeleteEvent });

    const actionsContainer = createElement('div', { class: 'events-hero-actions' }, [
      doorBtn.el,
      statusToggleBtn.el,
      editBtn.el,
      deleteBtn.el,
    ]);

    const heroContent = createElement('div', { class: 'events-hero-content' }, [
      createElement('div', { class: 'events-hero-title-row' }, [titleEl, badge]),
      metaEl,
      actionsContainer,
    ]);

    return createElement('section', { class: 'events-hero' }, [
      posterDisplay,
      fileInput,
      heroContent,
    ]);
  }

  function renderTabsBar() {
    const { activeTab = 'resumen', tickets = [], interests = [], expenses = [], onTabChange } = state;

    const tabs = [
      { id: 'resumen', label: '📊 Resumen' },
      { id: 'entradas', label: `🎟️ Entradas (${tickets.length})` },
      { id: 'interesados', label: `👥 Interesados (${interests.length})` },
      { id: 'gastos', label: `💸 Gastos (${expenses.length})` },
    ];

    const tabButtons = tabs.map((t) => {
      const isActive = activeTab === t.id;
      return createElement(
        'button',
        {
          type: 'button',
          role: 'tab',
          'aria-selected': isActive ? 'true' : 'false',
          class: `events-tab-btn ${isActive ? 'is-active' : ''}`,
          onClick: () => {
            if (typeof onTabChange === 'function') {
              onTabChange(t.id);
            }
          },
        },
        [t.label]
      );
    });

    return createElement('nav', { class: 'events-tabs-nav', role: 'tablist' }, tabButtons);
  }

  function renderResumenTab() {
    const { summary } = state;
    if (!summary) {
      return createElement('div', { class: 'events-tab-panel' }, [
        createElement('p', {}, ['Cargando resumen financiero...']),
      ]);
    }

    const netPositive = summary.ganancia_neta >= 0;
    const overCapacity = summary.aforo_proyectado > summary.capacidad_total;

    // Overcapacity warning banner
    let alertBanner = null;
    if (overCapacity) {
      alertBanner = createElement('div', { class: 'events-alert-banner' }, [
        createElement('strong', {}, ['⚠️ Alerta de Sobrecapacidad: ']),
        createElement('span', {}, [
          `El aforo proyectado (${summary.aforo_proyectado} personas) supera la capacidad total configurada (${summary.capacidad_total} lugares).`,
        ]),
      ]);
    }

    // 7 Metrics Cards
    const metricsGrid = createElement('div', { class: 'events-metrics-grid' }, [
      createElement('div', { class: 'events-metric-card' }, [
        createElement('span', { class: 'events-metric-label' }, ['Cobrado']),
        createElement('span', { class: 'events-metric-value text-success' }, [`S/ ${summary.cobrado.toFixed(2)}`]),
      ]),
      createElement('div', { class: 'events-metric-card' }, [
        createElement('span', { class: 'events-metric-label' }, ['Pendiente']),
        createElement('span', { class: 'events-metric-value text-warning' }, [`S/ ${summary.pendiente.toFixed(2)}`]),
      ]),
      createElement('div', { class: 'events-metric-card' }, [
        createElement('span', { class: 'events-metric-label' }, ['Total Proyectado']),
        createElement('span', { class: 'events-metric-value' }, [`S/ ${summary.total_proyectado.toFixed(2)}`]),
      ]),
      createElement('div', { class: 'events-metric-card' }, [
        createElement('span', { class: 'events-metric-label' }, ['Gastos']),
        createElement('span', { class: 'events-metric-value text-danger' }, [`S/ ${summary.gastos.toFixed(2)}`]),
      ]),
      createElement('div', { class: `events-metric-card ${netPositive ? 'is-positive' : 'is-negative'}` }, [
        createElement('span', { class: 'events-metric-label' }, ['Ganancia Neta']),
        createElement('span', { class: `events-metric-value ${netPositive ? 'text-success' : 'text-danger'}` }, [
          `S/ ${summary.ganancia_neta.toFixed(2)}`,
        ]),
      ]),
      createElement('div', { class: 'events-metric-card' }, [
        createElement('span', { class: 'events-metric-label' }, ['Capacidad / Emitidas']),
        createElement('span', { class: 'events-metric-value' }, [
          `${summary.entradas_emitidas} / ${summary.capacidad_total}`,
        ]),
      ]),
      createElement('div', { class: `events-metric-card ${overCapacity ? 'is-danger' : ''}` }, [
        createElement('span', { class: 'events-metric-label' }, ['Aforo Proyectado']),
        createElement('span', { class: `events-metric-value ${overCapacity ? 'text-danger' : ''}` }, [
          `${summary.aforo_proyectado}`,
        ]),
      ]),
    ]);

    // Payment methods breakdown
    const methods = summary.por_medio_pago || {};
    const methodRows = Object.entries(methods).map(([method, data]) => {
      return createElement('tr', {}, [
        createElement('td', {}, [method]),
        createElement('td', { class: 'text-right' }, [String(data.cantidad || 0)]),
        createElement('td', { class: 'text-right' }, [`S/ ${(data.cobrado || 0).toFixed(2)}`]),
        createElement('td', { class: 'text-right' }, [`S/ ${(data.pendiente || 0).toFixed(2)}`]),
      ]);
    });

    const paymentTable = createElement('div', { class: 'events-breakdown-card' }, [
      createElement('h4', { class: 'events-breakdown-title' }, ['Desglose por Medio de Pago']),
      createElement('div', { class: 'events-table-wrapper' }, [
        createElement('table', { class: 'events-table' }, [
          createElement('thead', {}, [
            createElement('tr', {}, [
              createElement('th', {}, ['Medio']),
              createElement('th', { class: 'text-right' }, ['Entradas']),
              createElement('th', { class: 'text-right' }, ['Cobrado']),
              createElement('th', { class: 'text-right' }, ['Pendiente']),
            ]),
          ]),
          createElement('tbody', {}, methodRows),
        ]),
      ]),
    ]);

    // Expenses by category breakdown
    const expensesMap = summary.gastos_por_categoria || {};
    const categoryRows = Object.entries(expensesMap).map(([cat, amount]) => {
      return createElement('tr', {}, [
        createElement('td', {}, [cat]),
        createElement('td', { class: 'text-right' }, [`S/ ${Number(amount).toFixed(2)}`]),
      ]);
    });

    const expenseTable = createElement('div', { class: 'events-breakdown-card' }, [
      createElement('h4', { class: 'events-breakdown-title' }, ['Desglose de Gastos por Categoría']),
      createElement('div', { class: 'events-table-wrapper' }, [
        createElement('table', { class: 'events-table' }, [
          createElement('thead', {}, [
            createElement('tr', {}, [
              createElement('th', {}, ['Categoría']),
              createElement('th', { class: 'text-right' }, ['Total (S/)']),
            ]),
          ]),
          createElement('tbody', {}, categoryRows),
        ]),
      ]),
    ]);

    return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
      alertBanner,
      metricsGrid,
      createElement('div', { class: 'events-breakdowns-row' }, [
        paymentTable,
        expenseTable,
      ]),
    ].filter(Boolean));
  }

  function renderEntradasTab() {
    const { tickets = [], onSellTickets, onConfirmPayment, onDeleteTicket, onTicketPhoto } = state;

    const sellBtn = createButton({
      text: '+ Vender Entradas',
      variant: 'primary',
      onClick: onSellTickets,
    });

    const toolbar = createElement('div', { class: 'events-tab-toolbar' }, [
      createElement('h3', { class: 'events-tab-heading' }, [`Entradas Emitidas (${tickets.length})`]),
      sellBtn.el,
    ]);

    if (tickets.length === 0) {
      return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
        toolbar,
        createElement('div', { class: 'events-empty-box' }, [
          createElement('p', {}, ['No se han emitido entradas para este evento.']),
        ]),
      ]);
    }

    const rows = tickets.map((t) => {
      const isPaid = t.estado_pago === 'Pagado';
      const hasEntered = t.estado_checkin === 'Ingresó';

      const paymentBadge = createElement(
        'span',
        { class: `events-badge ${isPaid ? 'events-badge-success' : 'events-badge-warning'}` },
        [t.estado_pago]
      );

      const checkinBadge = createElement(
        'span',
        { class: `events-badge ${hasEntered ? 'events-badge-active' : 'events-badge-muted'}` },
        [t.estado_checkin]
      );

      const confirmBtn = !isPaid
        ? createButton({
            text: 'Confirmar Pago',
            variant: 'secondary',
            size: 'sm',
            onClick: () => onConfirmPayment(t.id),
          }).el
        : null;

      let photoBtn = null;
      if (isPaid && typeof onTicketPhoto === 'function') {
        const btn = createButton({
          text: 'Imagen',
          variant: 'ghost',
          size: 'sm',
          onClick: async () => {
            btn.update({ text: 'Creando…', disabled: true });
            try {
              await onTicketPhoto(t);
            } finally {
              btn.update({ text: 'Imagen', disabled: false });
            }
          },
        });
        photoBtn = btn.el;
      }

      const delBtn = !hasEntered
        ? createButton({
            text: 'Eliminar',
            variant: 'danger',
            size: 'sm',
            onClick: () => onDeleteTicket(t),
          }).el
        : null;

      return createElement('tr', {}, [
        createElement('td', {}, [
          createElement('strong', {}, [t.nombre_comprador]),
          createElement('div', { class: 'events-table-subtext' }, [t.contacto]),
        ]),
        createElement('td', {}, [t.tipo_entrada]),
        createElement('td', {}, [`S/ ${Number(t.precio).toFixed(2)}`]),
        createElement('td', {}, [t.medio_pago]),
        createElement('td', {}, [paymentBadge]),
        createElement('td', {}, [checkinBadge]),
        createElement('td', { class: 'events-table-actions' }, [
          confirmBtn,
          photoBtn,
          delBtn,
        ].filter(Boolean)),
      ]);
    });

    const tableWrapper = createElement('div', { class: 'events-table-wrapper' }, [
      createElement('table', { class: 'events-table' }, [
        createElement('thead', {}, [
          createElement('tr', {}, [
            createElement('th', {}, ['Comprador']),
            createElement('th', {}, ['Tipo']),
            createElement('th', {}, ['Precio']),
            createElement('th', {}, ['Medio']),
            createElement('th', {}, ['Pago']),
            createElement('th', {}, ['Check-in']),
            createElement('th', {}, ['Acciones']),
          ]),
        ]),
        createElement('tbody', {}, rows),
      ]),
    ]);

    return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
      toolbar,
      tableWrapper,
    ]);
  }

  function renderInteresadosTab() {
    const { interests = [], onCreateInterest, onEditInterest, onDeleteInterest, onConvertInterest } = state;

    const createBtn = createButton({
      text: '+ Registrar Interesado',
      variant: 'primary',
      onClick: onCreateInterest,
    });

    const toolbar = createElement('div', { class: 'events-tab-toolbar' }, [
      createElement('h3', { class: 'events-tab-heading' }, [`Interesados / Reservas (${interests.length})`]),
      createBtn.el,
    ]);

    if (interests.length === 0) {
      return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
        toolbar,
        createElement('div', { class: 'events-empty-box' }, [
          createElement('p', {}, ['No hay interesados o reservas registradas aún.']),
        ]),
      ]);
    }

    const rows = interests.map((item) => {
      const isConverted = item.estado === 'Confirmado' || Boolean(item.convertido_at);

      const statusBadge = createElement(
        'span',
        {
          class: `events-badge ${
            item.estado === 'Confirmado'
              ? 'events-badge-success'
              : item.estado === 'Cancelado'
              ? 'events-badge-danger'
              : 'events-badge-warning'
          }`,
        },
        [item.estado]
      );

      const convertBtn = !isConverted
        ? createButton({
            text: 'Convertir a Venta',
            variant: 'primary',
            size: 'sm',
            onClick: () => onConvertInterest(item),
          }).el
        : null;

      const editBtn = createButton({
        text: 'Editar',
        variant: 'ghost',
        size: 'sm',
        onClick: () => onEditInterest(item),
      }).el;

      const delBtn = createButton({
        text: 'Eliminar',
        variant: 'danger',
        size: 'sm',
        onClick: () => onDeleteInterest(item),
      }).el;

      return createElement('tr', {}, [
        createElement('td', {}, [
          createElement('strong', {}, [item.nombre]),
          createElement('div', { class: 'events-table-subtext' }, [item.contacto]),
        ]),
        createElement('td', {}, [item.canal]),
        createElement('td', {}, [item.promotora]),
        createElement('td', {}, [`${item.personas} pers.`]),
        createElement('td', {}, [statusBadge]),
        createElement('td', {}, [item.notas || '—']),
        createElement('td', { class: 'events-table-actions' }, [
          convertBtn,
          editBtn,
          delBtn,
        ].filter(Boolean)),
      ]);
    });

    const tableWrapper = createElement('div', { class: 'events-table-wrapper' }, [
      createElement('table', { class: 'events-table' }, [
        createElement('thead', {}, [
          createElement('tr', {}, [
            createElement('th', {}, ['Contacto']),
            createElement('th', {}, ['Canal']),
            createElement('th', {}, ['Promotora']),
            createElement('th', {}, ['Personas']),
            createElement('th', {}, ['Estado']),
            createElement('th', {}, ['Notas']),
            createElement('th', {}, ['Acciones']),
          ]),
        ]),
        createElement('tbody', {}, rows),
      ]),
    ]);

    return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
      toolbar,
      tableWrapper,
    ]);
  }

  function renderGastosTab() {
    const { expenses = [], onCreateExpense, onEditExpense, onDeleteExpense } = state;

    const createBtn = createButton({
      text: '+ Registrar Gasto',
      variant: 'primary',
      onClick: onCreateExpense,
    });

    const toolbar = createElement('div', { class: 'events-tab-toolbar' }, [
      createElement('h3', { class: 'events-tab-heading' }, [`Gastos Registrados (${expenses.length})`]),
      createBtn.el,
    ]);

    if (expenses.length === 0) {
      return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
        toolbar,
        createElement('div', { class: 'events-empty-box' }, [
          createElement('p', {}, ['No se han registrado gastos para este evento.']),
        ]),
      ]);
    }

    const rows = expenses.map((g) => {
      const editBtn = createButton({
        text: 'Editar',
        variant: 'ghost',
        size: 'sm',
        onClick: () => onEditExpense(g),
      }).el;

      const delBtn = createButton({
        text: 'Eliminar',
        variant: 'danger',
        size: 'sm',
        onClick: () => onDeleteExpense(g),
      }).el;

      return createElement('tr', {}, [
        createElement('td', {}, [g.fecha]),
        createElement('td', {}, [
          createElement('span', { class: 'events-badge events-badge-category' }, [g.categoria]),
        ]),
        createElement('td', {}, [g.descripcion]),
        createElement('td', { class: 'text-right font-bold text-danger' }, [`S/ ${Number(g.monto).toFixed(2)}`]),
        createElement('td', { class: 'events-table-actions' }, [editBtn, delBtn]),
      ]);
    });

    const tableWrapper = createElement('div', { class: 'events-table-wrapper' }, [
      createElement('table', { class: 'events-table' }, [
        createElement('thead', {}, [
          createElement('tr', {}, [
            createElement('th', {}, ['Fecha']),
            createElement('th', {}, ['Categoría']),
            createElement('th', {}, ['Descripción']),
            createElement('th', { class: 'text-right' }, ['Monto']),
            createElement('th', {}, ['Acciones']),
          ]),
        ]),
        createElement('tbody', {}, rows),
      ]),
    ]);

    return createElement('div', { class: 'events-tab-panel', role: 'tabpanel' }, [
      toolbar,
      tableWrapper,
    ]);
  }

  function render() {
    el.innerHTML = '';
    el.appendChild(renderHero());
    el.appendChild(renderTabsBar());

    const activeTab = state.activeTab || 'resumen';
    if (activeTab === 'resumen') {
      el.appendChild(renderResumenTab());
    } else if (activeTab === 'entradas') {
      el.appendChild(renderEntradasTab());
    } else if (activeTab === 'interesados') {
      el.appendChild(renderInteresadosTab());
    } else if (activeTab === 'gastos') {
      el.appendChild(renderGastosTab());
    }
  }

  render();

  function update(nextProps = {}) {
    state = { ...state, ...nextProps };
    render();
  }

  return { el, update };
}
