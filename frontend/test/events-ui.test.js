import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

if (!globalThis.HTMLElement.prototype.click) {
  globalThis.HTMLElement.prototype.click = function () {
    this.dispatchEvent(new Event('click'));
  };
}

// Provide 2D Canvas mock in FakeDOM environment for ticket image generation
const origCreateElement = globalThis.document.createElement.bind(globalThis.document);
globalThis.document.createElement = function (tag) {
  const el = origCreateElement(tag);
  if (String(tag).toLowerCase() === 'canvas') {
    el.getContext = () => ({
      measureText: (text) => ({ width: (String(text).length || 1) * 12 }),
      fillRect: () => {},
      drawImage: () => {},
      fillText: () => {},
      stroke: () => {},
      fill: () => {},
      beginPath: () => {},
      arc: () => {},
      arcTo: () => {},
      moveTo: () => {},
      lineTo: () => {},
      closePath: () => {},
      save: () => {},
      restore: () => {},
      translate: () => {},
      scale: () => {},
      createLinearGradient: () => ({ addColorStop: () => {} }),
    });
    el.toBlob = (cb) => {
      const blob = new Blob(['fake-ticket-png-data'], { type: 'image/png' });
      cb(blob);
    };
  }
  return el;
};

const topbarModulePath = resolve(process.cwd(), 'frontend/js/components/organisms/app-topbar.js');
const { createAppTopbar } = await import(pathToFileURL(topbarModulePath).href);

const viewsModulePath = resolve(process.cwd(), 'frontend/js/modules/events/views.js');
const { createEventsCatalogView, createEventDetailView, formatEventDateTime } = await import(pathToFileURL(viewsModulePath).href);

const formsModulePath = resolve(process.cwd(), 'frontend/js/modules/events/forms.js');
const {
  createEventFormDialog,
  createExpenseFormDialog,
  createInterestFormDialog,
  createConvertInterestDialog,
  createSellTicketDialog,
} = await import(pathToFileURL(formsModulePath).href);

const ticketPreviewModulePath = resolve(process.cwd(), 'frontend/js/modules/events/ticket-image-preview-dialog.js');
const { createTicketImagePreviewDialog } = await import(pathToFileURL(ticketPreviewModulePath).href);

const ticketPhotoModulePath = resolve(process.cwd(), 'frontend/js/modules/events/ticket-photo.js');
const { generarFotoEntrada } = await import(pathToFileURL(ticketPhotoModulePath).href);

const routerModulePath = resolve(process.cwd(), 'frontend/js/router/parse-route.js');
const { parseRoute } = await import(pathToFileURL(routerModulePath).href);

const navigateModulePath = resolve(process.cwd(), 'frontend/js/router/navigate.js');
const { toPresentations, toEvents, toEvent } = await import(pathToFileURL(navigateModulePath).href);

const controllersModulePath = resolve(process.cwd(), 'frontend/js/modules/events/controllers.js');
const { createEventsCatalogController, createEventDetailController } = await import(pathToFileURL(controllersModulePath).href);

console.log('Running Events UI and Integration Tests...');

{
  const preview = createTicketImagePreviewDialog({
    ticket: { nombre_comprador: 'Ana', estado_pago: 'Pendiente' },
    event: { nombre: 'Noche de Gala' },
    onClose: () => {},
  });
  assert.ok(preview.el.textContent.includes('Imagen de la entrada'));
  // TIS-1: Obsolete clipboard workaround text must be removed
  assert.equal(preview.el.textContent.includes('abre WhatsApp'), false, 'Obsolete mobile clipboard help must be removed');
  assert.equal(preview.el.textContent.includes('clic derecho'), false, 'Obsolete desktop clipboard help must be removed');
  assert.equal(preview.el.textContent.includes('Copiar imagen'), false, 'Obsolete copy action must be removed');
  // Unpaid restriction preserved
  assert.ok(preview.el.textContent.includes('pendiente de pago'), 'Must enforce unpaid restriction');
  preview.destroy();
}

// 1. App Topbar with Section Selector (Repertorios | Eventos)
{
  let selectedSection = null;
  const topbar = createAppTopbar({
    title: 'Shirley Show',
    activeSection: 'presentations',
    onSectionChange: (sec) => {
      selectedSection = sec;
    },
  });

  const nav = topbar.el.querySelector('.app-topbar-nav');
  assert.ok(nav, 'Topbar must render .app-topbar-nav when activeSection is set');

  const buttons = topbar.el.querySelectorAll('.app-topbar-nav-btn');
  assert.equal(buttons.length, 2, 'Must render exactly 2 section buttons: Repertorios and Eventos');
  assert.equal(buttons[0].textContent, 'Repertorios');
  assert.equal(buttons[1].textContent, 'Eventos');

  assert.equal(buttons[0].getAttribute('aria-current'), 'page');
  assert.ok(buttons[0].getAttribute('class').includes('is-active'));
  assert.equal(buttons[1].getAttribute('aria-current'), null);

  // Click on Eventos
  buttons[1].dispatchEvent(new Event('click'));
  assert.equal(selectedSection, 'events', 'Clicking Eventos button triggers onSectionChange with "events"');

  // Update topbar to events active
  topbar.update({ activeSection: 'events' });
  const updatedButtons = topbar.el.querySelectorAll('.app-topbar-nav-btn');
  assert.equal(updatedButtons[1].getAttribute('aria-current'), 'page');
  assert.ok(updatedButtons[1].getAttribute('class').includes('is-active'));
}

// 2. Events Catalog View
{
  const mockEvents = [
    {
      id: 'evt-1',
      nombre: 'Noche Criolla Barranco',
      lugar: 'La Noche',
      fecha_hora: '2026-10-15T21:00:00Z',
      finalizado: 0,
      imagen: 'evt1_poster.jpg',
      tipos_de_entrada: [
        { nombre: 'General', precio: 30, capacidad: 50 },
        { nombre: 'VIP', precio: 60, capacidad: 20 },
      ],
    },
    {
      id: 'evt-2',
      nombre: 'Concierto Aniversario',
      lugar: 'Teatro Municipal',
      fecha_hora: '2026-05-10T20:00:00Z',
      finalizado: 1,
      imagen: null,
      tipos_de_entrada: [
        { nombre: 'Preferencial', precio: 40, capacidad: 80 },
      ],
    },
  ];

  let selectedEventId = null;
  let editedEvent = null;
  let deletedEvent = null;

  const catalogView = createEventsCatalogView({
    events: mockEvents,
    isLoading: false,
    errorMessage: null,
    onCreateEvent: () => {},
    onSelectEvent: (id) => { selectedEventId = id; },
    onEditEvent: (evt) => { editedEvent = evt; },
    onDeleteEvent: (evt) => { deletedEvent = evt; },
  });

  // Verify sections
  const cards = catalogView.el.querySelectorAll('.events-card');
  assert.equal(cards.length, 2, 'Must render two event cards');

  // Active event card
  assert.ok(!cards[0].getAttribute('class').includes('is-finalized'));
  assert.ok(cards[0].textContent.includes('Noche Criolla Barranco'));
  assert.ok(cards[0].textContent.includes('La Noche'));
  assert.ok(cards[0].textContent.includes('General: S/ 30.00'));
  assert.ok(cards[0].textContent.includes('Activo'));

  // Finalized event card
  assert.ok(cards[1].getAttribute('class').includes('is-finalized'));
  assert.ok(cards[1].textContent.includes('Finalizado'));

  // Click card triggers onSelectEvent
  cards[0].dispatchEvent(new Event('click'));
  assert.equal(selectedEventId, 'evt-1');
}

// 3. Event Detail View with Tabs & 7 Metrics
{
  const mockEvent = {
    id: 'evt-1',
    nombre: 'Noche Criolla Barranco',
    lugar: 'La Noche',
    fecha_hora: '2026-10-15T21:00:00Z',
    finalizado: 0,
    imagen: 'evt1_poster.jpg',
    tipos_de_entrada: [{ nombre: 'General', precio: 30, capacidad: 50 }],
  };

  const mockSummary = {
    cobrado: 300.0,
    pendiente: 120.0,
    total_proyectado: 420.0,
    gastos: 180.0,
    ganancia_neta: 120.0,
    capacidad_total: 50,
    entradas_emitidas: 16,
    aforo_proyectado: 19,
    por_medio_pago: {
      Yape: { cobrado: 300.0, pendiente: 0.0, cantidad: 10 },
      Efectivo: { cobrado: 0.0, pendiente: 120.0, cantidad: 4 },
      Cortesía: { cobrado: 0.0, pendiente: 0.0, cantidad: 2 },
    },
    gastos_por_categoria: {
      'Local/Alquiler': 100.0,
      'Sonido/Luces': 80.0,
    },
  };

  const mockTickets = [
    {
      id: 'tix-1',
      evento_id: 'evt-1',
      nombre_comprador: 'Valeria Díaz',
      contacto: '999111222',
      tipo_entrada: 'General',
      precio: 30.0,
      medio_pago: 'Yape',
      estado_pago: 'Pagado',
      estado_checkin: 'No ingresó',
    },
    {
      id: 'tix-2',
      evento_id: 'evt-1',
      nombre_comprador: 'Pedro Gómez',
      contacto: '999333444',
      tipo_entrada: 'General',
      precio: 30.0,
      medio_pago: 'Efectivo',
      estado_pago: 'Pendiente',
      estado_checkin: 'No ingresó',
    },
  ];

  const mockInterests = [
    {
      id: 'int-1',
      evento_id: 'evt-1',
      nombre: 'Mariana Rios',
      contacto: '987654321',
      canal: 'Instagram',
      promotora: 'Shirley',
      personas: 3,
      estado: 'Pendiente',
      notas: 'Mesa cerca al escenario',
    },
  ];

  const mockExpenses = [
    {
      id: 'exp-1',
      evento_id: 'evt-1',
      categoria: 'Local/Alquiler',
      monto: 100.0,
      descripcion: 'Adelanto de local',
      fecha: '2026-10-01',
    },
    {
      id: 'exp-2',
      evento_id: 'evt-1',
      categoria: 'Sonido/Luces',
      monto: 80.0,
      descripcion: 'Consola de audio',
      fecha: '2026-10-02',
    },
  ];

  let currentTab = 'resumen';
  let doorOpened = false;

  const detailView = createEventDetailView({
    event: mockEvent,
    summary: mockSummary,
    tickets: mockTickets,
    interests: mockInterests,
    expenses: mockExpenses,
    activeTab: currentTab,
    onTabChange: (tab) => {
      currentTab = tab;
      detailView.update({ activeTab: tab });
    },
    onOpenDoor: () => { doorOpened = true; },
    onFinalize: () => {},
    onReopen: () => {},
    onUploadPoster: () => {},
    onEditEvent: () => {},
    onDeleteEvent: () => {},
    onSellTickets: () => {},
    onConfirmPayment: () => {},
    onDeleteTicket: () => {},
    onCreateInterest: () => {},
    onEditInterest: () => {},
    onDeleteInterest: () => {},
    onConvertInterest: () => {},
    onCreateExpense: () => {},
    onEditExpense: () => {},
    onDeleteExpense: () => {},
  });

  // Check Hero
  assert.ok(detailView.el.textContent.includes('Noche Criolla Barranco'));
  assert.ok(detailView.el.textContent.includes('La Noche'));

  // Check 7 Metrics in Resumen
  const metricsValues = detailView.el.querySelectorAll('.events-metric-value');
  assert.equal(metricsValues.length, 7, 'Must render exactly 7 metric cards');
  assert.equal(metricsValues[0].textContent, 'S/ 300.00', 'Cobrado');
  assert.equal(metricsValues[1].textContent, 'S/ 120.00', 'Pendiente');
  assert.equal(metricsValues[2].textContent, 'S/ 420.00', 'Total Proyectado');
  assert.equal(metricsValues[3].textContent, 'S/ 180.00', 'Gastos');
  assert.equal(metricsValues[4].textContent, 'S/ 120.00', 'Ganancia Neta');
  assert.equal(metricsValues[5].textContent, '16 / 50', 'Capacidad / Emitidas');
  assert.equal(metricsValues[6].textContent, '19', 'Aforo Proyectado');

  // Test overcapacity warning
  detailView.update({
    summary: { ...mockSummary, capacidad_total: 15, aforo_proyectado: 20 },
  });
  const alertBanner = detailView.el.querySelector('.events-alert-banner');
  assert.ok(alertBanner, 'Must render .events-alert-banner when aforo_proyectado > capacidad_total');

  // Switch to Entradas Tab
  const tabButtons = detailView.el.querySelectorAll('.events-tab-btn');
  assert.equal(tabButtons.length, 4);
  tabButtons[1].dispatchEvent(new Event('click'));
  assert.equal(currentTab, 'entradas');
  assert.ok(detailView.el.textContent.includes('Valeria Díaz'));
  assert.ok(detailView.el.textContent.includes('Pedro Gómez'));

  // Switch to Interesados Tab
  tabButtons[2].dispatchEvent(new Event('click'));
  assert.equal(currentTab, 'interesados');
  assert.ok(detailView.el.textContent.includes('Mariana Rios'));
  assert.ok(detailView.el.textContent.includes('3 pers.'));

  // Switch to Gastos Tab
  tabButtons[3].dispatchEvent(new Event('click'));
  assert.equal(currentTab, 'gastos');
  assert.ok(detailView.el.textContent.includes('Adelanto de local'));
  assert.ok(detailView.el.textContent.includes('Consola de audio'));
}

// 4. Modal Dialogs creation and destruction
{
  const eventDialog = createEventFormDialog({
    onSave: () => {},
    onCancel: () => {},
  });
  assert.ok(eventDialog.el.querySelector('.events-dialog'));
  eventDialog.destroy();

  const expenseDialog = createExpenseFormDialog({
    onSave: () => {},
    onCancel: () => {},
  });
  assert.ok(expenseDialog.el.querySelector('.events-dialog'));
  expenseDialog.destroy();

  const interestDialog = createInterestFormDialog({
    onSave: () => {},
    onCancel: () => {},
  });
  assert.ok(interestDialog.el.querySelector('.events-dialog'));
  interestDialog.destroy();

  // Test Venta Rápida with single ticket type (preselected) and multiple ticket types (not preselected)
  {
    const singleTypeDialog = createSellTicketDialog({
      ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
      onSell: () => {},
      onCancel: () => {},
    });
    const typeSelect = singleTypeDialog.el.querySelector('select');
    assert.equal(typeSelect.value, 'General', 'Single ticket type must be preselected');
    singleTypeDialog.destroy();

    const multiTypeDialog = createSellTicketDialog({
      ticketTypes: [
        { nombre: 'General', precio: 30, capacidad: 50 },
        { nombre: 'VIP', precio: 60, capacidad: 20 },
      ],
      onSell: () => {},
      onCancel: () => {},
    });
    const multiSelect = multiTypeDialog.el.querySelector('select');
    assert.equal(multiSelect.value, '', 'Multiple ticket types must NOT be preselected');
    const multiInputs = multiTypeDialog.el.querySelectorAll('input');
    multiInputs[0].value = 'Comprador sin teléfono';
    multiInputs[1].value = '';
    multiTypeDialog.el.querySelector('form').dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 0));
    assert.equal(
      multiTypeDialog.el.querySelector('.events-form-error').textContent,
      'Selecciona un tipo de entrada.',
      'Validation must identify the missing ticket type without blaming the optional phone'
    );
    multiTypeDialog.destroy();
  }

  // Test Venta Rápida submission and issued tickets result cards with WhatsApp links
  {
    let soldData = null;
    const fakeIssuedTickets = [
      {
        id: 't-1',
        tipo_entrada: 'General',
        precio: 30,
        nombre_comprador: 'Ana Silva',
        contacto: '987654321',
        scan_token: 'tok_ana_1',
        medio_pago: 'Yape',
        estado_pago: 'Pagado',
      },
      {
        id: 't-2',
        tipo_entrada: 'General',
        precio: 30,
        nombre_comprador: 'Ana Silva',
        contacto: '987654321',
        scan_token: 'tok_ana_2',
        medio_pago: 'Yape',
        estado_pago: 'Pagado',
      },
    ];

    const quickSale = createSellTicketDialog({
      eventName: 'Noche de Gala',
      ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
      onSell: async (data) => {
        soldData = data;
        return fakeIssuedTickets;
      },
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Ana Silva';
    assert.equal(inputs[1].getAttribute('required'), null, 'Phone must be optional for a quick ticket sale');
    inputs[1].value = '';
    inputs[2].value = '2';

    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 10));

    assert.equal(soldData.nombre_comprador, 'Ana Silva');
    assert.equal(soldData.contacto, '', 'An empty phone must be sent to the API');
    assert.equal(soldData.cantidad, 2);

    const cardsList = quickSale.el.querySelector('.events-quick-tickets-list');
    assert.ok(cardsList, 'Must render .events-quick-tickets-list after successful sale');
    const cards = quickSale.el.querySelectorAll('.events-quick-ticket-card');
    assert.equal(cards.length, 2, 'Must render 2 ticket cards');

    assert.equal(
      cards[0].querySelector('.events-quick-ticket-wa-btn'),
      null,
      'The issued ticket card must not open WhatsApp with a link'
    );
    assert.equal(
      cards[0].querySelector('.events-quick-ticket-preview-btn'),
      null,
      'Must NOT require intermediate preview button (TIS-1)'
    );
    assert.equal(cards[0].textContent.includes('Ver Ticket'), false);
    assert.equal(cards[0].textContent.includes('Compartir por WhatsApp'), false);
    assert.equal(cards[0].textContent.includes('Ver imagen de la entrada'), false, 'No intermediate preview CTA text');

    // Immediate ticket image presentation
    const img1 = cards[0].querySelector('.ticket-image-preview');
    assert.ok(img1, 'Must render ticket image immediately for ticket #1');
    const img2 = cards[1].querySelector('.ticket-image-preview');
    assert.ok(img2, 'Must render ticket image immediately for ticket #2');

    // No verbose clipboard workaround text
    assert.equal(quickSale.el.textContent.includes('abre WhatsApp'), false);
    assert.equal(quickSale.el.textContent.includes('clic derecho'), false);
    assert.equal(quickSale.el.textContent.includes('Copiar imagen'), false);

    // Unsupported environment fallback (Node / default has no Web Share files)
    const actionBtn = cards[0].querySelector('.events-quick-ticket-download-btn, .events-quick-ticket-share-btn');
    assert.ok(actionBtn, 'Must render ticket action button');

    quickSale.destroy();
  }

  const convertDialog = createConvertInterestDialog({
    interest: { nombre: 'Juan', contacto: '999', personas: 2 },
    ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
    onConvert: () => {},
    onCancel: () => {},
  });
  assert.ok(convertDialog.el.querySelector('.events-dialog'));
  convertDialog.destroy();
}

// 5. Default Route & Navigation Transition Tests
{
  // Clean / default route always resolves to presentations
  assert.equal(parseRoute('').view, 'presentations');
  assert.equal(parseRoute('#/').view, 'presentations');
  assert.equal(parseRoute('#/presentations').view, 'presentations');

  // Transition to #/eventos resolves to events catalog
  assert.equal(parseRoute('#/eventos').view, 'events');

  // Transition to #/eventos/:id resolves to event-detail
  const detailRoute = parseRoute('#/eventos/evt-cena-show');
  assert.equal(detailRoute.view, 'event-detail');
  assert.equal(detailRoute.params.id, 'evt-cena-show');

  // Transition back to #/presentations
  assert.equal(parseRoute('#/presentations').view, 'presentations');
}

// 6. Controllers Integration with Real Axum Envelopes ({eventos}, {evento}, {resumen}, {entradas}, {interesados}, {gastos})
{
  const realEvent = {
    id: 'evt-real-1',
    nombre: 'Noche Real Criolla',
    lugar: 'Barranco Bar',
    fecha_hora: '2026-11-20T21:00:00Z',
    finalizado: 0,
    imagen: null,
    tipos_de_entrada: [
      { nombre: 'General', precio: 35.0, capacidad: 60 },
    ],
  };

  const realSummaryEnvelope = {
    resumen: {
      cobrado: 350.0,
      pendiente: 105.0,
      total_proyectado: 455.0,
      gastos: 200.0,
      ganancia_neta: 150.0,
      capacidad_total: 60,
      entradas_emitidas: 13,
      aforo_proyectado: 15,
      desglose_metodos_pago: {
        yape_cobrado: 350.0,
        yape_pendiente: 0.0,
        yape_entradas: 10,
        efectivo_cobrado: 0.0,
        efectivo_pendiente: 105.0,
        efectivo_entradas: 3,
        cortesia_entradas: 0,
      },
      desglose_gastos: {
        local_alquiler: 120.0,
        sonido_luces: 80.0,
        musicos_personal: 0.0,
        publicidad_marketing: 0.0,
        bebidas_catering: 0.0,
        transporte: 0.0,
        otros: 0.0,
      },
    },
  };

  const realTicketsEnvelope = {
    entradas: [
      {
        id: 't-real-1',
        evento_id: 'evt-real-1',
        nombre_comprador: 'Rosa Maria',
        contacto: '988111222',
        tipo_entrada: 'General',
        precio: 35.0,
        medio_pago: 'Yape',
        estado_pago: 'Pagado',
        scan_token: 'st_tok_1',
        ticket_path: '#/ticket/st_tok_1',
        estado_checkin: 'No ingresó',
        hora_ingreso: null,
        qr_svg: '<svg>qr</svg>',
      },
    ],
  };

  const realInterestsEnvelope = {
    interesados: [
      {
        id: 'int-real-1',
        evento_id: 'evt-real-1',
        nombre: 'Carlos Perez',
        contacto: '999111333',
        canal: 'WhatsApp',
        promotora: 'Shirley',
        personas: 2,
        estado: 'Pendiente',
        convertido_at: null,
      },
    ],
  };

  const realExpensesEnvelope = {
    gastos: [
      {
        id: 'exp-real-1',
        evento_id: 'evt-real-1',
        categoria: 'Sonido/Luces',
        monto: 80.0,
        descripcion: 'Microfonia',
        fecha: '2026-11-19',
      },
    ],
  };

  globalThis.fetch = async (url, opts = {}) => {
    const urlStr = String(url);
    const method = (opts.method || 'GET').toUpperCase();

    if (urlStr.includes('/api/eventos?incluir_finalizados=true')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => ({ eventos: [realEvent] }),
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/resumen')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realSummaryEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/entradas') && method === 'GET') {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realTicketsEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/entradas') && method === 'POST') {
      return {
        ok: true,
        status: 201,
        headers: { get: () => 'application/json' },
        json: async () => ({
          entradas: [
            {
              id: 't-issued-1',
              tipo_entrada: 'General',
              precio: 35.0,
              nombre_comprador: 'Nuevo Comprador',
              contacto: '912345678',
              scan_token: 'st_tok_new',
              ticket_path: '#/ticket/st_tok_new',
              medio_pago: 'Yape',
              estado_pago: 'Pagado',
            },
          ],
        }),
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/interesados')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realInterestsEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/gastos')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realExpensesEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1') && method === 'GET') {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => ({ evento: realEvent }),
      };
    }

    return {
      ok: false,
      status: 404,
      headers: { get: () => 'application/json' },
      json: async () => ({ error: 'not found' }),
    };
  };

  // 6.a Test Events Catalog Controller with { eventos: [...] } envelope
  const catalogCtrl = createEventsCatalogController();
  await new Promise((r) => setTimeout(r, 50));

  const catalogCards = catalogCtrl.viewEl.querySelectorAll('.events-card');
  assert.equal(catalogCards.length, 1, 'Catalog must render 1 card from real { eventos: [...] } envelope');
  assert.ok(catalogCards[0].textContent.includes('Noche Real Criolla'));
  catalogCtrl.destroy();

  // 6.b Test Event Detail Controller with real envelopes
  const detailCtrl = createEventDetailController({ eventId: 'evt-real-1' });
  await new Promise((r) => setTimeout(r, 50));

  assert.ok(detailCtrl.topbarEl.textContent.includes('Noche Real Criolla'), 'Topbar receives event.nombre from real { evento } envelope');

  // Verify 7 metrics rendered from normalized { resumen: ... }
  assert.ok(detailCtrl.viewEl.textContent.includes('S/ 350.00'), 'Cobrado metric rendered from envelope');
  assert.ok(detailCtrl.viewEl.textContent.includes('S/ 105.00'), 'Pendiente metric rendered from envelope');
  assert.ok(detailCtrl.viewEl.textContent.includes('S/ 150.00'), 'Ganancia neta rendered from envelope');

  // Verify breakdown tables populated from desglose_metodos_pago and desglose_gastos
  assert.ok(detailCtrl.viewEl.textContent.includes('Desglose por Medio de Pago'));
  assert.ok(detailCtrl.viewEl.textContent.includes('Desglose de Gastos por Categoría'));
  assert.ok(detailCtrl.viewEl.textContent.includes('Yape'));
  assert.ok(detailCtrl.viewEl.textContent.includes('Sonido/Luces'));

  detailCtrl.destroy();
}

// 7. Ticket Image Sharing & Immediate Presentation (ODD Task TIS-1)
{
  const origNavigatorDesc = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
  const baseNavigator = globalThis.navigator || {};

  function setMockNavigator(mockObj) {
    Object.defineProperty(globalThis, 'navigator', {
      value: mockObj,
      configurable: true,
      writable: true,
    });
  }

  function restoreNavigator() {
    if (origNavigatorDesc) {
      Object.defineProperty(globalThis, 'navigator', origNavigatorDesc);
    }
  }

  // 7.a Native file sharing supported (mobile environment simulation)
  {
    let shareCalls = [];
    setMockNavigator({
      ...baseNavigator,
      canShare: (data) => Boolean(data && data.files && data.files.length > 0),
      share: async (data) => {
        shareCalls.push(data);
      },
    });

    const quickSale = createSellTicketDialog({
      eventName: 'Noche de Salsa',
      ticketTypes: [{ nombre: 'VIP', precio: 80, capacidad: 20 }],
      onSell: async () => [
        {
          id: 'tix-share-1',
          tipo_entrada: 'VIP',
          precio: 80,
          nombre_comprador: 'Mario Vargas',
          scan_token: 'tok_mario_1',
          estado_pago: 'Pagado',
        },
      ],
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Mario Vargas';
    inputs[1].value = '';
    inputs[2].value = '1';
    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 20));

    const card = quickSale.el.querySelector('.events-quick-ticket-card');
    assert.ok(card, 'Card must be rendered');

    // Immediate image preview
    const img = card.querySelector('.ticket-image-preview');
    assert.ok(img, 'Must render ticket image immediately');
    assert.equal(img.style.display, 'block', 'Image must be displayed after generation');

    // Concise Compartir action
    const shareBtn = card.querySelector('.events-quick-ticket-share-btn');
    assert.ok(shareBtn, 'Must render concise Compartir button on Web Share supported device');
    assert.equal(shareBtn.textContent, 'Compartir');
    assert.equal(shareBtn.disabled, false, 'Share button must be enabled once image is ready');

    // Clicking Compartir calls navigator.share directly with pre-generated image file
    shareBtn.dispatchEvent(new Event('click'));
    await new Promise((r) => setTimeout(r, 10));

    assert.equal(shareCalls.length, 1, 'navigator.share must be called');
    assert.ok(shareCalls[0].title.includes('Noche de Salsa'));
    assert.equal(shareCalls[0].files.length, 1);
    assert.equal(shareCalls[0].files[0].type, 'image/png');
    assert.ok(shareCalls[0].files[0].name.startsWith('entrada-'));

    quickSale.destroy();
  }

  // 7.b Silent AbortError handling when user cancels native share sheet
  {
    let abortHandledSilently = true;
    let fallbackDownloadTriggered = false;

    // Track if a download link was clicked
    const origBodyAppend = document.body.appendChild.bind(document.body);
    document.body.appendChild = function (child) {
      if (child.tagName === 'A' && child.download) {
        fallbackDownloadTriggered = true;
      }
      return origBodyAppend(child);
    };

    setMockNavigator({
      ...baseNavigator,
      canShare: (data) => Boolean(data && data.files && data.files.length > 0),
      share: async () => {
        const abortErr = new Error('The user aborted the share operation.');
        abortErr.name = 'AbortError';
        throw abortErr;
      },
    });

    const quickSale = createSellTicketDialog({
      eventName: 'Noche de Salsa',
      ticketTypes: [{ nombre: 'General', precio: 50, capacidad: 10 }],
      onSell: async () => [
        {
          id: 'tix-abort-1',
          tipo_entrada: 'General',
          precio: 50,
          nombre_comprador: 'Sara Morales',
          scan_token: 'tok_sara_1',
          estado_pago: 'Pagado',
        },
      ],
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Sara Morales';
    inputs[1].value = '';
    inputs[2].value = '1';
    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 20));

    const card = quickSale.el.querySelector('.events-quick-ticket-card');
    const shareBtn = card.querySelector('.events-quick-ticket-share-btn');
    assert.ok(shareBtn);

    try {
      shareBtn.dispatchEvent(new Event('click'));
      await new Promise((r) => setTimeout(r, 10));
    } catch {
      abortHandledSilently = false;
    }

    assert.equal(abortHandledSilently, true, 'AbortError must be silently handled without throwing');
    assert.equal(fallbackDownloadTriggered, false, 'AbortError must NOT trigger fallback download');

    document.body.appendChild = origBodyAppend;
    quickSale.destroy();
  }

  // 7.c Desktop / unsupported environment fallback (Descargar)
  {
    setMockNavigator({
      ...baseNavigator,
      canShare: undefined,
      share: undefined,
    });

    let downloadTriggered = false;
    let downloadedFilename = null;
    const origBodyAppend = document.body.appendChild.bind(document.body);
    document.body.appendChild = function (child) {
      if (child.tagName === 'A' && child.download) {
        downloadTriggered = true;
        downloadedFilename = child.download;
      }
      return origBodyAppend(child);
    };

    const quickSale = createSellTicketDialog({
      eventName: 'Concierto Rock',
      ticketTypes: [{ nombre: 'General', precio: 40, capacidad: 30 }],
      onSell: async () => [
        {
          id: 'tix-desk-1',
          tipo_entrada: 'General',
          precio: 40,
          nombre_comprador: 'Luis Rojas',
          scan_token: 'tok_luis_1',
          estado_pago: 'Pagado',
        },
      ],
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Luis Rojas';
    inputs[1].value = '';
    inputs[2].value = '1';
    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 20));

    const card = quickSale.el.querySelector('.events-quick-ticket-card');
    const downloadBtn = card.querySelector('.events-quick-ticket-download-btn');
    assert.ok(downloadBtn, 'Must render Descargar button when Web Share files is unsupported');
    assert.equal(downloadBtn.textContent, 'Descargar');

    downloadBtn.dispatchEvent(new Event('click'));
    await new Promise((r) => setTimeout(r, 10));

    assert.equal(downloadTriggered, true, 'Clicking Descargar must trigger download in unsupported browser');
    assert.ok(downloadedFilename && downloadedFilename.startsWith('entrada-'));

    document.body.appendChild = origBodyAppend;
    quickSale.destroy();
  }

  // 7.d Unpaid ticket restrictions preserved
  {
    const quickSale = createSellTicketDialog({
      eventName: 'Noche de Cumbia',
      ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 20 }],
      onSell: async () => [
        {
          id: 'tix-unpaid-1',
          tipo_entrada: 'General',
          precio: 30,
          nombre_comprador: 'Pedro Gómez',
          scan_token: 'tok_pedro_1',
          estado_pago: 'Pendiente',
        },
      ],
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Pedro Gómez';
    inputs[1].value = '';
    inputs[2].value = '1';
    // Change payment status to Pendiente
    quickSale.el.querySelectorAll('select')[1].value = 'Pendiente';
    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 20));

    const card = quickSale.el.querySelector('.events-quick-ticket-card');
    assert.ok(card, 'Card must be rendered for unpaid ticket');

    // Unpaid restrictions:
    assert.ok(card.querySelector('.events-badge-warning'), 'Must have warning badge');
    assert.ok(card.querySelector('.events-quick-ticket-unpaid-note'), 'Must show unpaid notice');
    assert.ok(card.textContent.includes('pendiente de pago'));
    assert.equal(card.querySelector('.ticket-image-preview'), null, 'Unpaid ticket must NOT render image preview');
    assert.equal(card.querySelector('.events-quick-ticket-share-btn'), null, 'Unpaid ticket must NOT offer share button');
    assert.equal(card.querySelector('.events-quick-ticket-download-btn'), null, 'Unpaid ticket must NOT offer download button');

    quickSale.destroy();
  }

  // 7.e Multiple issued tickets: each gets immediate image and concise action
  {
    setMockNavigator({
      ...baseNavigator,
      canShare: (data) => Boolean(data && data.files && data.files.length > 0),
      share: async () => {},
    });

    const quickSale = createSellTicketDialog({
      eventName: 'Festival Shirley',
      ticketTypes: [{ nombre: 'General', precio: 25, capacidad: 50 }],
      onSell: async () => [
        { id: 'm-1', tipo_entrada: 'General', precio: 25, nombre_comprador: 'Familia Perez', scan_token: 't1', estado_pago: 'Pagado' },
        { id: 'm-2', tipo_entrada: 'General', precio: 25, nombre_comprador: 'Familia Perez', scan_token: 't2', estado_pago: 'Pagado' },
        { id: 'm-3', tipo_entrada: 'General', precio: 25, nombre_comprador: 'Familia Perez', scan_token: 't3', estado_pago: 'Pagado' },
      ],
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Familia Perez';
    inputs[1].value = '';
    inputs[2].value = '3';
    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 30));

    const cards = quickSale.el.querySelectorAll('.events-quick-ticket-card');
    assert.equal(cards.length, 3, 'Must render exactly 3 ticket cards');

    for (let i = 0; i < 3; i++) {
      const img = cards[i].querySelector('.ticket-image-preview');
      assert.ok(img, `Card #${i + 1} must render ticket image immediately`);
      const btn = cards[i].querySelector('.events-quick-ticket-share-btn');
      assert.ok(btn, `Card #${i + 1} must render its own share button`);
      assert.equal(btn.textContent, 'Compartir');
    }

    quickSale.destroy();
  }

  // 7.f Standalone createTicketImagePreviewDialog with paid ticket and Web Share support
  {
    let shareCalled = false;
    setMockNavigator({
      ...baseNavigator,
      canShare: (data) => Boolean(data && data.files && data.files.length > 0),
      share: async () => {
        shareCalled = true;
      },
    });

    const preview = createTicketImagePreviewDialog({
      ticket: { nombre_comprador: 'Elena Rios', estado_pago: 'Pagado', codigo_qr: 'QR987654' },
      event: { nombre: 'Noche Criolla' },
      onClose: () => {},
    });

    await new Promise((r) => setTimeout(r, 20));

    assert.ok(preview.el.querySelector('.ticket-image-preview'));
    const shareBtn = preview.el.querySelector('.ticket-image-share-btn');
    assert.ok(shareBtn);
    assert.equal(shareBtn.textContent, 'Compartir');
    assert.equal(shareBtn.disabled, false);

    shareBtn.dispatchEvent(new Event('click'));
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(shareCalled, true, 'createTicketImagePreviewDialog must invoke navigator.share on click');

    preview.destroy();
  }

  // 7.g Fallback to download when navigator.share encounters unexpected non-abort error
  {
    let fallbackDownloadTriggered = false;
    const origBodyAppend = document.body.appendChild.bind(document.body);
    document.body.appendChild = function (child) {
      if (child.tagName === 'A' && child.download) {
        fallbackDownloadTriggered = true;
      }
      return origBodyAppend(child);
    };

    setMockNavigator({
      ...baseNavigator,
      canShare: (data) => Boolean(data && data.files && data.files.length > 0),
      share: async () => {
        throw new Error('OS Share sheet failure');
      },
    });

    const quickSale = createSellTicketDialog({
      eventName: 'Noche de Salsa',
      ticketTypes: [{ nombre: 'VIP', precio: 80, capacidad: 20 }],
      onSell: async () => [
        {
          id: 'tix-share-fail-1',
          tipo_entrada: 'VIP',
          precio: 80,
          nombre_comprador: 'Carmen Ruiz',
          scan_token: 'tok_carmen_1',
          estado_pago: 'Pagado',
        },
      ],
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Carmen Ruiz';
    inputs[1].value = '';
    inputs[2].value = '1';
    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 20));

    const card = quickSale.el.querySelector('.events-quick-ticket-card');
    const shareBtn = card.querySelector('.events-quick-ticket-share-btn');
    assert.ok(shareBtn);

    // Suppress console.warn for expected fallback test
    const origWarn = console.warn;
    console.warn = () => {};
    shareBtn.dispatchEvent(new Event('click'));
    await new Promise((r) => setTimeout(r, 10));
    console.warn = origWarn;

    assert.equal(fallbackDownloadTriggered, true, 'Non-AbortError in share must trigger fallback download');

    document.body.appendChild = origBodyAppend;
    quickSale.destroy();
  }

  // 7.h Standalone createTicketImagePreviewDialog fails closed on missing or unknown payment status (TIS-3)
  {
    for (const testCase of [
      { label: 'missing payment status', ticket: { nombre_comprador: 'Lucia Diaz', codigo_qr: 'QR111' } },
      { label: 'unknown payment status', ticket: { nombre_comprador: 'Roberto Gomez', estado_pago: 'Desconocido', codigo_qr: 'QR222' } },
      { label: 'null payment status', ticket: { nombre_comprador: 'Ana Paz', estado_pago: null, codigo_qr: 'QR333' } },
    ]) {
      const preview = createTicketImagePreviewDialog({
        ticket: testCase.ticket,
        event: { nombre: 'Noche Criolla' },
        onClose: () => {},
      });

      await new Promise((r) => setTimeout(r, 20));

      const statusEl = preview.el.querySelector('.ticket-image-status');
      assert.ok(statusEl, `${testCase.label}: must render status element`);
      assert.equal(
        statusEl.textContent,
        'Esta entrada está pendiente de pago. Debes confirmar el pago antes de generar la imagen.',
        `${testCase.label}: must show exact unpaid explanation`
      );

      const img = preview.el.querySelector('.ticket-image-preview');
      assert.ok(img, `${testCase.label}: image element exists`);
      assert.equal(img.style.display, 'none', `${testCase.label}: image element must remain hidden`);
      assert.ok(!img.src, `${testCase.label}: image src must not be populated`);

      assert.equal(
        preview.el.querySelector('.ticket-image-share-btn'),
        null,
        `${testCase.label}: must not offer share action`
      );
      assert.equal(
        preview.el.querySelector('.ticket-image-download-btn'),
        null,
        `${testCase.label}: must not offer download action`
      );

      const actionsEl = preview.el.querySelector('.events-dialog-actions');
      assert.ok(actionsEl, `${testCase.label}: actions container must exist`);
      const actionButtons = actionsEl.querySelectorAll('button');
      assert.equal(actionButtons.length, 1, `${testCase.label}: only close button must be present`);
      assert.equal(actionButtons[0].textContent, 'Cerrar');

      preview.destroy();
    }
  }

  // 7.i Quick Sell Ticket Dialog fails closed on missing or unknown payment status (TIS-3)
  {
    for (const testCase of [
      { label: 'missing payment status', ticket: { id: 'tix-missing-1', tipo_entrada: 'General', precio: 40, nombre_comprador: 'Hugo V.', scan_token: 'tok_hugo_1' } },
      { label: 'unknown payment status', ticket: { id: 'tix-unknown-1', tipo_entrada: 'General', precio: 40, nombre_comprador: 'Sara M.', scan_token: 'tok_sara_1', estado_pago: 'EnRevision' } },
    ]) {
      const quickSale = createSellTicketDialog({
        eventName: 'Festival Shirley',
        ticketTypes: [{ nombre: 'General', precio: 40, capacidad: 20 }],
        onSell: async () => [testCase.ticket],
        onCancel: () => {},
      });

      const form = quickSale.el.querySelector('form');
      const inputs = quickSale.el.querySelectorAll('input');
      inputs[0].value = testCase.ticket.nombre_comprador;
      inputs[1].value = '';
      inputs[2].value = '1';
      form.dispatchEvent(new Event('submit'));
      await new Promise((r) => setTimeout(r, 20));

      const card = quickSale.el.querySelector('.events-quick-ticket-card');
      assert.ok(card, `${testCase.label}: card must be rendered`);

      const unpaidNote = card.querySelector('.events-quick-ticket-unpaid-note');
      assert.ok(unpaidNote, `${testCase.label}: must render unpaid note`);
      assert.equal(
        unpaidNote.textContent,
        'Esta entrada está pendiente de pago. Debes confirmar el pago antes de generar la imagen.',
        `${testCase.label}: must show exact unpaid explanation`
      );

      assert.equal(
        card.querySelector('.ticket-image-preview'),
        null,
        `${testCase.label}: must NOT render image preview`
      );
      assert.equal(
        card.querySelector('.events-quick-ticket-share-btn'),
        null,
        `${testCase.label}: must NOT offer share button`
      );
      assert.equal(
        card.querySelector('.events-quick-ticket-download-btn'),
        null,
        `${testCase.label}: must NOT offer download button`
      );

      quickSale.destroy();
    }
  }

  // 7.j Direct regression tests for generarFotoEntrada paid-status defense in depth (TIS-3)
  {
    const unpaidError = {
      message: 'Esta entrada está pendiente de pago. Debes confirmar el pago antes de generar la imagen.',
    };

    // Missing payment status
    await assert.rejects(
      async () => {
        await generarFotoEntrada({
          nombre_comprador: 'Lucas M.',
          codigo_qr: 'QR-MISSING',
        });
      },
      unpaidError,
      'generarFotoEntrada must reject when estado_pago is missing'
    );

    // Null payment status
    await assert.rejects(
      async () => {
        await generarFotoEntrada({
          nombre_comprador: 'Lucas M.',
          estado_pago: null,
          codigo_qr: 'QR-NULL',
        });
      },
      unpaidError,
      'generarFotoEntrada must reject when estado_pago is null'
    );

    // Unknown payment status
    await assert.rejects(
      async () => {
        await generarFotoEntrada({
          nombre_comprador: 'Lucas M.',
          estado_pago: 'Desconocido',
          codigo_qr: 'QR-UNKNOWN',
        });
      },
      unpaidError,
      'generarFotoEntrada must reject when estado_pago is unknown'
    );

    // Pending payment status
    await assert.rejects(
      async () => {
        await generarFotoEntrada({
          nombre_comprador: 'Lucas M.',
          estado_pago: 'Pendiente',
          codigo_qr: 'QR-PENDING',
        });
      },
      unpaidError,
      'generarFotoEntrada must reject when estado_pago is Pendiente'
    );

    // Explicitly paid status must succeed and produce a Blob
    const blob = await generarFotoEntrada({
      nombre_comprador: 'Lucas M.',
      estado_pago: 'Pagado',
      codigo_qr: 'QR-PAID',
    });
    assert.ok(blob, 'generarFotoEntrada must succeed when estado_pago is explicitly Pagado');
  }

  // Restore navigator
  restoreNavigator();
}

console.log('All Events UI and Integration Tests passed cleanly!');
